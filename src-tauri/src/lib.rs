mod cache;
mod screen_capture;
mod translator;
mod windows_api;
mod windows_uia;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use cache::TranslationCache;
use screen_capture::{capture_region_if_changed, CaptureRegion};
use translator::{sanitize_trailing_punct, TranslationConfig, Translator};

/// 某个窗口连续失败到这个次数后，就认定它不支持 UIA，以后直接跳过（省几十~几百毫秒）
const UIA_FAIL_THRESHOLD: u32 = 2;

pub struct AppState {
    config: Mutex<TranslationConfig>,
    translator: Arc<Translator>,
    /// 译文缓存：相同句子直接返回，命中时 0ms
    cache: Mutex<TranslationCache>,
    /// 窗口句柄 → 连续「UIA 读不到文本」次数
    uia_read_fail: Mutex<HashMap<isize, u32>>,
    /// 窗口句柄 → 连续「UIA 写不进文本」次数（Chromium 系常见）
    uia_write_fail: Mutex<HashMap<isize, u32>>,
    /// 是否正在执行一次替换（防止连按热键导致并发粘贴）
    busy: AtomicBool,
}

/// 提示词版本号。
/// 只要改了 system prompt（措辞、术语表、风格），就必须把它 +1，
/// 否则缓存里旧风格译文会一直命中，看起来像"改了没生效"。
const PROMPT_VERSION: &str = "2";

/// 缓存键：原文归一化 + 目标语言 + 是否游戏模式 + 引擎。
/// 归一化只做「不影响含义」的处理：去空白、去句末标点、小写，
/// 这样「别送了，」和「别送了」能命中同一条。
fn cache_key(text: &str, config: &TranslationConfig) -> String {
    const PUNCT: &str = "，。！？、；：,.!?;:…~～\"'“”‘’ \t\r\n";

    let normalized: String = text
        .chars()
        .filter(|c| !PUNCT.contains(*c))
        .flat_map(|c| c.to_lowercase())
        .collect();

    let engine = if config.uses_free_channel() {
        "free".to_string()
    } else {
        format!("{}/{}", config.provider, config.model)
    };

    format!(
        "{}|{}|{}|{}|p{}",
        normalized,
        config.target_lang.to_lowercase(),
        config.is_game_mode,
        engine,
        PROMPT_VERSION
    )
}

/// 读剪贴板文本，并判断它是否与"操作前的旧内容"不同。
/// 用基线比对替代"先清空剪贴板"，这样不会动用户原来的剪贴板内容。
fn read_clipboard_change(app: &AppHandle, baseline: Option<&str>) -> Option<String> {
    let txt = app.clipboard().read_text().ok()?;
    let trimmed = txt.trim();
    if trimmed.is_empty() {
        return None;
    }
    if Some(trimmed) == baseline {
        return None;
    }
    Some(trimmed.to_string())
}

/// 粘贴前确认前台还是同一个窗口。请求要 1 秒多，中途 Alt+Tab 会把译文粘到别人窗口里。
fn foreground_unchanged(hwnd: isize) -> bool {
    windows_api::foreground_window() == hwnd
}

/// 链路结束后把剪贴板还原成原来的文本（图片/文件无法备份，只能还原文本）
async fn restore_clipboard_text(app: &AppHandle, backup: &Option<String>) {
    if let Some(text) = backup {
        // 等粘贴真正落地再还原，否则可能把旧内容粘进输入框
        tokio::time::sleep(Duration::from_millis(180)).await;
        let _ = app.clipboard().write_text(text);
    }
}

/// 带缓存的翻译：命中直接返回 (译文, true)，未命中才走网络 (译文, false)
async fn translate_with_cache(
    app: &AppHandle,
    text: &str,
    config: &TranslationConfig,
) -> Result<(String, bool), String> {
    let key = cache_key(text, config);

    {
        let state = app.state::<AppState>();
        if let Ok(mut c) = state.cache.lock() {
            if let Some(hit) = c.get(&key) {
                return Ok((hit, true));
            }
        }
    }

    let translated = {
        let state = app.state::<AppState>();
        state.translator.translate_llm(text, config).await?
    };

    {
        let state = app.state::<AppState>();
        if let Ok(mut c) = state.cache.lock() {
            c.put(key, translated.clone());
        }
    }

    Ok((translated, false))
}

fn log_to_ui(app: &AppHandle, msg: &str) {
    println!("{}", msg);
    let _ = app.emit("translation_log", msg.to_string());
}

/// 明确告知本次走的是哪个引擎，避免"Key 到底有没有生效"无法判断
fn log_engine(app: &AppHandle, config: &TranslationConfig) {
    if config.uses_free_channel() {
        log_to_ui(
            app,
            "🌐 [引擎] 内置免费通道 MyMemory 机翻（非大模型，质量有限）",
        );
    } else {
        log_to_ui(
            app,
            &format!(
                "🧠 [引擎] 大模型 {} @ {} → {}",
                config.model, config.api_base, config.target_lang
            ),
        );
    }
}

fn config_file_path(app: &AppHandle) -> Option<std::path::PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join("config.json"))
}

fn load_config(app: &AppHandle) -> Option<TranslationConfig> {
    let path = config_file_path(app)?;
    let raw = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str::<TranslationConfig>(&raw).ok()
}

fn persist_config(app: &AppHandle, config: &TranslationConfig) -> Result<(), String> {
    let path = config_file_path(app).ok_or_else(|| "无法获取配置目录".to_string())?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("创建配置目录失败: {e}"))?;
    }
    let json =
        serde_json::to_string_pretty(config).map_err(|e| format!("序列化配置失败: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("写入配置文件失败: {e}"))?;
    Ok(())
}

#[tauri::command]
async fn translate(
    app: AppHandle,
    text: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    if text.trim().is_empty() {
        return Ok("".to_string());
    }
    let config = {
        let lock = state.config.lock().map_err(|e| e.to_string())?;
        lock.clone()
    };
    // 走缓存：配置页反复点"立即翻译"同一句时不再重复烧钱
    let (result, _cached) = translate_with_cache(&app, &text, &config).await?;
    let sanitized = sanitize_trailing_punct(&result, config.keep_trailing_punct);
    Ok(sanitized)
}

/// 配置页"测试连接"：用当前配置真实请求一次，把失败原因原样返回给 UI
#[tauri::command]
async fn test_connection(
    app: AppHandle,
    text: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let config = {
        state
            .config
            .lock()
            .map_err(|_| "配置锁异常".to_string())?
            .clone()
    };

    if config.uses_free_channel() {
        return Err(
            "当前是「内置免费通道」，不需要也无法测试连接——它不走大模型。请选择具体服务商并填写 API Key。"
                .to_string(),
        );
    }
    if config.provider != "ollama" && config.api_key.trim().is_empty() {
        return Err("未填写 API Key：当前会走免费机翻通道，不是大模型".to_string());
    }
    if config.api_base.trim().is_empty() {
        return Err("API Base URL 为空".to_string());
    }

    let started = std::time::Instant::now();
    let result = state.translator.translate_llm(&text, &config).await?;
    let ms = started.elapsed().as_millis();
    log_to_ui(
        &app,
        &format!("🧪 [测试连接] {} 连通，耗时 {}ms → \"{}\"", config.model, ms, result),
    );
    Ok(result)
}

#[tauri::command]
fn get_config(state: State<'_, AppState>) -> TranslationConfig {
    match state.config.lock() {
        Ok(lock) => lock.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    }
}

#[tauri::command]
fn set_autostart(enable: bool) -> Result<bool, String> {
    #[cfg(target_os = "windows")]
    {
        let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let exe_path = current_exe.to_string_lossy().to_string();

        if enable {
            let cmd = format!(
                "reg add \"HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run\" /v ChatMorph /t REG_SZ /d \"\\\"{}\\\"\" /f",
                exe_path
            );
            let output = std::process::Command::new("cmd")
                .args(["/C", &cmd])
                .creation_flags(0x08000000)
                .output()
                .map_err(|e| format!("写入自启动注册表失败: {e}"))?;
            if output.status.success() {
                Ok(true)
            } else {
                Err(String::from_utf8_lossy(&output.stderr).to_string())
            }
        } else {
            let _ = std::process::Command::new("cmd")
                .args(["/C", "reg delete \"HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run\" /v ChatMorph /f"])
                .creation_flags(0x08000000)
                .output();
            Ok(false)
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(false)
    }
}

#[tauri::command]
fn is_autostart_enabled() -> bool {
    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("cmd")
            .args(["/C", "reg query \"HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run\" /v ChatMorph"])
            .creation_flags(0x08000000)
            .output();
        match output {
            Ok(out) => out.status.success(),
            Err(_) => false,
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

#[tauri::command]
fn save_config(
    app: AppHandle,
    new_config: TranslationConfig,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // 联动设置开机自启动
    let _ = set_autostart(new_config.autostart);

    // 先落盘再更新内存，保证重启后配置不丢
    persist_config(&app, &new_config)?;
    let mut lock = state.config.lock().map_err(|_| "配置锁异常".to_string())?;
    *lock = new_config;
    Ok(())
}

/// 动态修改并注册全局热键
#[tauri::command]
fn update_hotkey(app: AppHandle, new_hotkey: String, state: State<'_, AppState>) -> Result<String, String> {
    let new_shortcut = new_hotkey.parse::<Shortcut>()
        .map_err(|e| format!("快捷键格式无效: {}", e))?;

    let old_hotkey = {
        state
            .config
            .lock()
            .map_err(|_| "配置锁异常".to_string())?
            .hotkey
            .clone()
    };

    // 设成同一个热键时直接返回：重复注册同一组合会被系统判为"已被占用"
    if old_hotkey == new_hotkey {
        return Ok(new_hotkey);
    }

    // 关键修复：先注册新键，成功后才注销旧键。
    // 否则新键被占用时会连旧热键一起丢掉，程序彻底失去响应。
    app.global_shortcut().register(new_shortcut)
        .map_err(|e| format!("注册快捷键失败，可能被 Windows 系统或其他软件占用: {}", e))?;

    if old_hotkey != new_hotkey {
        if let Ok(old_shortcut) = old_hotkey.parse::<Shortcut>() {
            let _ = app.global_shortcut().unregister(old_shortcut);
        }
    }

    let snapshot = {
        let mut lock = state.config.lock().map_err(|_| "配置锁异常".to_string())?;
        lock.hotkey = new_hotkey.clone();
        lock.clone()
    };
    persist_config(&app, &snapshot)?;

    log_to_ui(&app, &format!("[系统] 成功更新全局快捷键为: {}", new_hotkey));
    Ok(new_hotkey)
}

/// 供前端调试用的延迟倒计时原地替换
#[tauri::command]
fn test_in_place_after_delay(app: AppHandle, delay_secs: u64) {
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        log_to_ui(&handle, &format!("[测试] 正在倒计时 {} 秒，请在此期间将鼠标光标聚焦到目标聊天框...", delay_secs));
        tokio::time::sleep(Duration::from_secs(delay_secs)).await;
        trigger_in_place_translate(handle).await;
    });
}

/// 核心原地翻译替换逻辑入口。
/// 同一时间只允许一条链路：连按两次热键会让两条链路同时抢剪贴板、各粘一次，
/// 可能重复发送或粘错内容，所以用原子标志把第二次直接挡掉。
async fn trigger_in_place_translate(app: AppHandle) {
    let already_busy = {
        let state = app.state::<AppState>();
        state.busy.swap(true, Ordering::SeqCst)
    };
    if already_busy {
        log_to_ui(&app, "⏳ [已忽略] 上一次翻译还没结束，本次按键丢弃（防止重复粘贴）");
        return;
    }

    do_in_place_translate(app.clone()).await;

    let state = app.state::<AppState>();
    state.busy.store(false, Ordering::SeqCst);
}

/// 核心原地翻译替换逻辑：全链路带日志推送、双通道回填
async fn do_in_place_translate(app: AppHandle) {
    let started = std::time::Instant::now();
    let state = app.state::<AppState>();
    let config = {
        let lock = match state.config.lock() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        };
        lock
    };
    // 记住"是哪个窗口"，用于判断 UIA 在那个窗口上能不能用，以及粘贴前确认没跑偏
    let hwnd = windows_api::foreground_window();

    log_to_ui(&app, "🟢 [热键触发] 已捕获转换按键！");

    // 关键消抖：智能等待用户物理手指彻底离开 Alt / Ctrl 键，避免快捷键粘连
    windows_api::wait_for_modifiers_released();
    tokio::time::sleep(Duration::from_millis(10)).await;

    // 记下剪贴板原有文本：整条链路结束后还原，避免"换个词就把你剪贴板清了"
    let clipboard_before = app
        .clipboard()
        .read_text()
        .map(|t| t.trim().to_string())
        .ok()
        .filter(|t| !t.is_empty());

    // ==============================================================
    // 方案一：优先通过 Windows UI Automation 直接读取并修改控件文本
    // 完美支持：微信、Discord、Telegram、浏览器、记事本等一切标准窗口
    //
    // 优化：UIA 在游戏/自绘控件上必然失败，而每次失败都要等 UIA 自身超时。
    // 所以按窗口记住失败次数，连续失败 2 次后直接跳过，省掉这段白等。
    // ==============================================================
    let uia_read_fails = state
        .uia_read_fail
        .lock()
        .map(|m| m.get(&hwnd).copied().unwrap_or(0))
        .unwrap_or(0);

    if uia_read_fails < UIA_FAIL_THRESHOLD {
        if let Ok(uia_text) = windows_uia::get_focused_text() {
            let trimmed = uia_text.trim().to_string();
            if !trimmed.is_empty() {
                // 这个窗口 UIA 可用，清零历史失败次数
                if let Ok(mut m) = state.uia_read_fail.lock() {
                    m.insert(hwnd, 0);
                }

                log_to_ui(&app, &format!("🎯 [方案一: UIA直取] 成功提取输入框文字: \"{}\"", trimmed));
                log_engine(&app, &config);
                log_to_ui(&app, "⏳ [网络请求] 正在调用翻译引擎...");

                match translate_with_cache(&app, &trimmed, &config).await {
                    Ok((translated, cached)) => {
                        if cached {
                            log_to_ui(&app, "⚡ [缓存命中] 这句话之前翻过，直接复用译文（0ms）");
                        }
                        log_to_ui(&app, &format!("✨ [翻译成功] 结果: \"{}\"", translated));

                        // 优先尝试原生设值；控件拒绝写入时才退回粘贴。
                        // 同样按窗口记忆：连续写失败 2 次后不再尝试，省掉一次 UIA 调用。
                        let write_fails = state
                            .uia_write_fail
                            .lock()
                            .map(|m| m.get(&hwnd).copied().unwrap_or(0))
                            .unwrap_or(0);

                        let mut written = false;
                        if write_fails < UIA_FAIL_THRESHOLD {
                            match windows_uia::set_focused_text(&translated) {
                                Ok(_) => {
                                    written = true;
                                    if let Ok(mut m) = state.uia_write_fail.lock() {
                                        m.insert(hwnd, 0);
                                    }
                                }
                                Err(e) => {
                                    if let Ok(mut m) = state.uia_write_fail.lock() {
                                        *m.entry(hwnd).or_insert(0) += 1;
                                    }
                                    log_to_ui(&app, &format!("🖊️ [UIA写回不可用] {}", e));
                                }
                            }
                        } else {
                            log_to_ui(&app, "⏭️ [跳过UIA写入] 这个窗口之前写不进去，直接粘贴");
                        }

                        let mut used_clipboard = false;
                        if !written {
                            // 请求花了一秒多，中途 Alt+Tab / 游戏弹窗都会让粘贴跑到别人窗口里
                            if !foreground_unchanged(hwnd) {
                                log_to_ui(&app, "⚠️ [已中止] 前台窗口已变化（你切走了），为避免粘到别的窗口，本次未写入");
                                return;
                            }
                            log_to_ui(&app, "📝 [极速写入] 自动覆盖粘贴更新聊天框...");
                            windows_api::release_modifier_keys();
                            if let Err(e) = app.clipboard().write_text(&translated) {
                                log_to_ui(
                                    &app,
                                    &format!("❌ [剪贴板写入失败] {} —— 已中止，避免粘贴出上一次的旧内容", e),
                                );
                                return;
                            }
                            used_clipboard = true;
                            tokio::time::sleep(Duration::from_millis(15)).await;
                            windows_api::send_select_all_combo();
                            tokio::time::sleep(Duration::from_millis(15)).await;
                            windows_api::send_paste_combo();
                        }

                        log_to_ui(
                            &app,
                            &format!(
                                "🎉 [替换完成] 输入框已成功更新为外语！总耗时 {}ms",
                                started.elapsed().as_millis()
                            ),
                        );
                        if config.auto_send {
                            if foreground_unchanged(hwnd) {
                                tokio::time::sleep(Duration::from_millis(40)).await;
                                windows_api::send_enter_key();
                                log_to_ui(&app, "🚀 [自动发送] 已回车发送！");
                            } else {
                                log_to_ui(&app, "⚠️ [未自动发送] 前台窗口已变化，已跳过回车（避免误发到别处）");
                            }
                        }
                        if used_clipboard {
                            restore_clipboard_text(&app, &clipboard_before).await;
                        }
                        return;
                    }
                    Err(e) => {
                        log_to_ui(&app, &format!("❌ [翻译接口报错] {}", e));
                        return;
                    }
                }
            }
        }

        // 没读到文本：累计失败次数，下次同窗口直接跳过 UIA
        if let Ok(mut m) = state.uia_read_fail.lock() {
            *m.entry(hwnd).or_insert(0) += 1;
        }
    } else {
        log_to_ui(&app, "⏭️ [跳过UIA] 这个窗口之前读不到控件文本，直接走剪贴板通道");
    }

    log_to_ui(&app, "🔄 [精准通道] 正在提取待翻译内容...");

    // 以前这里是"先清空剪贴板，再 Ctrl+C，谁有内容就是谁"，
    // 代价是你剪贴板里原本的东西被永久清掉了。
    // 现在不清空，改成和"操作前的基线"比对：内容没变就说明没复制到选区。

    // 步骤 1：先检查用户是否已经手动划选了文字（发送 Ctrl+C 试探复制选区）
    // 划选替换是最安全、最精准的模式，100% 绝不波及输入框外部任何元素！
    windows_api::send_copy_combo();
    tokio::time::sleep(Duration::from_millis(30)).await;

    let mut original_text = String::new();
    if let Some(t) = read_clipboard_change(&app, clipboard_before.as_deref()) {
        original_text = t;
        log_to_ui(&app, &format!("🎯 [划选提取] 检测到用户鼠标选区: \"{}\"", original_text));
    }

    // 步骤 2：如果用户打完字未手动划选（打完字直接按热键）：
    // 自动全选输入框：Ctrl + A -> 自动复制：Ctrl + C！彻底解放双手，无需手动全选！
    if original_text.is_empty() {
        if config.auto_select_all {
            log_to_ui(&app, "⚡ [全自动提取] 正在自动全选当前输入框文字...");
            windows_api::release_modifier_keys();

            windows_api::send_select_all_combo();
            tokio::time::sleep(Duration::from_millis(25)).await;

            windows_api::send_copy_combo();
            tokio::time::sleep(Duration::from_millis(35)).await;

            // 循环尝试读取剪贴板（最多等 120ms）
            for _ in 0..8 {
                if let Some(t) = read_clipboard_change(&app, clipboard_before.as_deref()) {
                    original_text = t;
                    break;
                }
                tokio::time::sleep(Duration::from_millis(15)).await;
            }

            // 安全防线：如果提取出的内容异常过大（误触长篇网页文章 > 300字），立即撤销选区
            if original_text.chars().count() > 300 || original_text.lines().count() > 6 {
                log_to_ui(&app, "⚠️ [安全拦截] 选区超出单输入框范围，已自动取消全选保护！");
                windows_api::clear_selection_by_arrow_key();
                return;
            }
        }
    }

    if original_text.is_empty() {
        log_to_ui(&app, "⚠️ [提取未获取到文本] 未在输入框内获取到内容！");
        return;
    }

    log_to_ui(&app, &format!("🎯 [提取成功] 原文: \"{}\"", original_text));
    log_engine(&app, &config);
    log_to_ui(&app, "⏳ [网络请求] 正在调用翻译引擎...");

    // 步骤 3：调用翻译（带缓存，命中直接返回）
    match translate_with_cache(&app, &original_text, &config).await {
        Ok((translated, cached)) => {
            if cached {
                log_to_ui(&app, "⚡ [缓存命中] 这句话之前翻过，直接复用译文（0ms）");
            }
            let final_translated = sanitize_trailing_punct(&translated, config.keep_trailing_punct);
            log_to_ui(&app, &format!("✨ [翻译成功] 结果: \"{}\"", final_translated));

            // 步骤 4 之前先确认真没跑偏：整条链路已经过了一秒多
            if !foreground_unchanged(hwnd) {
                log_to_ui(&app, "⚠️ [已中止] 前台窗口已变化（你切走了），为避免粘到别的窗口，译文未写入");
                return;
            }

            // 步骤 4：将译文写回剪贴板并模拟 Ctrl + V 原地覆盖
            if let Err(e) = app.clipboard().write_text(&final_translated) {
                log_to_ui(
                    &app,
                    &format!("❌ [剪贴板写入失败] {} —— 已中止，避免粘贴出上一次的旧内容", e),
                );
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
            windows_api::send_paste_combo();
            log_to_ui(
                &app,
                &format!(
                    "🎉 [替换完成] 输入框已成功更新为外语！总耗时 {}ms",
                    started.elapsed().as_millis()
                ),
            );

            // 步骤 5：如果开启了自动回车发送
            if config.auto_send {
                if foreground_unchanged(hwnd) {
                    tokio::time::sleep(Duration::from_millis(40)).await;
                    windows_api::send_enter_key();
                    log_to_ui(&app, "🚀 [自动发送] 已回车发送！");
                } else {
                    log_to_ui(&app, "⚠️ [未自动发送] 前台窗口已变化，已跳过回车（避免误发到别处）");
                }
            }

            // 步骤 6：还原你原来的剪贴板内容
            restore_clipboard_text(&app, &clipboard_before).await;
        }
        Err(e) => {
            log_to_ui(&app, &format!("❌ [翻译接口报错] {}", e));
        }
    }
}

/// 设置悬浮 HUD 窗口是否开启鼠标穿透
#[tauri::command]
fn set_hud_click_through(app: AppHandle, enable: bool) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("hud_overlay") {
        #[cfg(target_os = "windows")]
        {
            if let Ok(hwnd) = win.hwnd() {
                windows_api::set_click_through(hwnd.0 as isize, enable)?;
            }
        }
    }
    Ok(())
}

/// 检查指定区域是否有新滚动文字
#[tauri::command]
fn check_chat_roi(region: CaptureRegion) -> Result<Option<String>, String> {
    capture_region_if_changed(&region)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        let handle = app.clone();
                        tauri::async_runtime::spawn(async move {
                            trigger_in_place_translate(handle).await;
                        });
                    }
                })
                .build(),
        )
        .manage(AppState {
            config: Mutex::new(TranslationConfig::default()),
            translator: Arc::new(Translator::new()),
            cache: Mutex::new(TranslationCache::new()),
            uia_read_fail: Mutex::new(HashMap::new()),
            uia_write_fail: Mutex::new(HashMap::new()),
            busy: AtomicBool::new(false),
        })
        .invoke_handler(tauri::generate_handler![
            translate,
            get_config,
            save_config,
            update_hotkey,
            test_connection,
            test_in_place_after_delay,
            set_hud_click_through,
            check_chat_roi,
            set_autostart,
            is_autostart_enabled,
        ])
        .on_window_event(|window, event| {
            // 点右上角 X 窗口不退出，而是隐藏到后台，保持全局热键和翻译常驻
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(|app| {
            // 启动时读取已落盘的配置（API Key / 模型 / 目标语言 / 热键 / 开机自启等）
            let handle = app.handle().clone();
            let cfg = load_config(&handle).unwrap_or_default();
            let hotkey = cfg.hotkey.clone();
            let autostart_cfg = cfg.autostart;
            {
                let state = app.state::<AppState>();
                if let Ok(mut lock) = state.config.lock() {
                    *lock = cfg;
                }
            }

            // 同步一次开机自启动设置（如果配置为 true，则确保注册表已指向当前 exe）
            if autostart_cfg {
                let _ = set_autostart(true);
            }

            // 创建系统托盘图标与右键菜单
            let show_item = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出微语", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            let mut tray_builder = TrayIconBuilder::new()
                .menu(&tray_menu)
                .tooltip("微语 (ChatMorph) - 原生聊天翻译器 (后台运行中)")
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.unminimize();
                                let _ = window.set_focus();
                            }
                        }
                    }
                });

            if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            }
            let _tray = tray_builder.build(app)?;

            // 按配置里保存的热键注册；失败才回退默认值
            let mut registered = false;
            if let Ok(s) = hotkey.parse::<Shortcut>() {
                match app.global_shortcut().register(s) {
                    Ok(_) => {
                        println!("[热键系统] ✅ 已按配置注册全局热键: {hotkey}");
                        registered = true;
                    }
                    Err(e) => eprintln!("[热键系统] ❌ 配置热键 {hotkey} 注册失败: {e}"),
                }
            } else {
                eprintln!("[热键系统] ⚠️ 配置中的热键无法解析: {hotkey}");
            }

            if !registered {
                // 配置里的键被占用/非法时退回默认键，避免程序启动后完全没有热键
                let default = "Alt+Backquote"
                    .parse::<Shortcut>()
                    .expect("默认热键必然可解析");
                match app.global_shortcut().register(default) {
                    Ok(_) => println!("[热键系统] ✅ 已回退默认热键 Alt+Backquote"),
                    Err(e) => eprintln!("[热键系统] ❌ 默认热键同样注册失败: {e}"),
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
