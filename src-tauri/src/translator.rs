use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

fn default_provider() -> String {
    // "auto" = 老配置文件没有该字段时的行为：按 API Key 是否为空自动判定
    "auto".to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TranslationConfig {
    /// 翻译引擎：deepseek | siliconflow | openai | ollama | free | auto
    /// 必须显式记录，否则"选了内置免费但 Key 还留着"就永远走不到免费通道；
    /// 反过来 Ollama 不需要 Key，也会被误判成免费通道。
    #[serde(default = "default_provider")]
    pub provider: String,
    pub api_base: String,     // e.g. "https://api.deepseek.com/v1" or "https://api.openai.com/v1"
    pub api_key: String,
    pub model: String,        // e.g. "deepseek-chat" or "gpt-4o-mini"
    pub target_lang: String,  // e.g. "English", "Japanese", "Korean"
    pub is_game_mode: bool,   // 开启游戏黑话增强
    pub hotkey: String,       // 全局快捷键，默认 "Alt+Backquote" (即 Alt + ~，ESC 正下方按键)
    pub auto_select_all: bool,// 快捷键触发时是否自动全选输入框
    pub auto_send: bool,      // 替换后是否自动回车发出
    #[serde(default)]
    pub keep_trailing_punct: bool,// 是否保留结尾标点符号（默认 false：自动剥离句末句号标点，适配聊天）
    #[serde(default)]
    pub autostart: bool,          // 开机自启动
}

/// 根据配置剥离结尾标点（如句号、感叹号、问号等）
pub fn sanitize_trailing_punct(text: &str, keep_trailing_punct: bool) -> String {
    let trimmed = text.trim();
    if keep_trailing_punct {
        return trimmed.to_string();
    }
    let mut s = trimmed.to_string();
    while let Some(ch) = s.chars().last() {
        if ch == '.' || ch == '。' || ch == '!' || ch == '！' || ch == '?' || ch == '？' || ch == ',' || ch == '，' || ch == ';' || ch == '；' {
            s.pop();
        } else {
            break;
        }
    }
    s.trim().to_string()
}

impl TranslationConfig {
    /// 本次翻译是否走"内置免费通道"（MyMemory 公共接口）
    pub fn uses_free_channel(&self) -> bool {
        match self.provider.as_str() {
            "free" => true,
            // 本地 Ollama 本来就不需要 Key，不能判成免费通道
            "ollama" => false,
            // 其它服务商：没填 Key 才降级
            _ => self.api_key.trim().is_empty(),
        }
    }
}

impl Default for TranslationConfig {
    fn default() -> Self {
        Self {
            provider: "deepseek".to_string(),
            api_base: "https://api.deepseek.com/v1".to_string(),
            api_key: "".to_string(),
            model: "deepseek-chat".to_string(),
            target_lang: "English".to_string(),
            is_game_mode: true,
            hotkey: "Alt+Backquote".to_string(),
            auto_select_all: true,
            auto_send: false,
            keep_trailing_punct: false,
            autostart: false,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessage,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
}

pub struct Translator {
    client: Client,
}

impl Translator {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(8))
                .build()
                .unwrap_or_else(|_| Client::new()),
        }
    }

    /// 使用配置的大模型 API 进行高质量翻译
    pub async fn translate_llm(
        &self,
        text: &str,
        config: &TranslationConfig,
    ) -> Result<String, String> {
        if config.uses_free_channel() {
            // 未配置 Key / 显式选择了内置免费通道 → 自动降级到免费机翻
            return self.translate_free_fallback(text, &config.target_lang).await;
        }

        // 提示词精简过：原来 182 tokens 一大半是重复解释，实测同样的句子省约 180ms。
        // 术语示例必须保留 —— "守高地" 能不能变成 "defend HG" 全靠这几条映射。
        let system_prompt = if config.is_game_mode {
            format!(
                "Translate into {} for competitive game chat. Use in-game slang/abbreviations: \
                 开大=Ult, 抓人=Gank, 守高地=defend HG, 别送=stop feeding, 中单差距=mid diff, \
                 集合打龙=group baron, 残血=low HP, 打野=jungler, 回城=recall/B. \
                 Keep it short and punchy. Output ONLY the translation, no quotes, no explanation.",
                config.target_lang
            )
        } else {
            format!(
                "Translate into {} for casual chat (Discord/X/WeChat). Natural and friendly. \
                 Output ONLY the translation, no quotes, no explanation.",
                config.target_lang
            )
        };

        let endpoint = format!("{}/chat/completions", config.api_base.trim_end_matches('/'));

        let body = ChatCompletionRequest {
            model: config.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_prompt,
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: text.to_string(),
                },
            ],
            // 翻译要的是稳定复现，不是创意：温度 0 让同一句每次得到同一结果，
            // 也让缓存真正有意义（否则每次措辞都变，缓存看着像没生效）。
            temperature: 0.0,
            // 上限给宽一点：对普通模型没有副作用（只是上限），
            // 推理模型才有机会在思考完之后还留出正文的空间。
            max_tokens: 800,
        };

        let response = self
            .client
            .post(&endpoint)
            .header("Authorization", format!("Bearer {}", config.api_key.trim()))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Network request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let err_text = response.text().await.unwrap_or_default();
            return Err(format!("API Error ({}): {}", status, err_text));
        }

        let res_json: ChatCompletionResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response JSON: {}", e))?;

        if let Some(choice) = res_json.choices.into_iter().next() {
            let result = choice.message.content.trim().to_string();

            // 关键防线：推理模型（如 deepseek-flash）会先把 token 全花在思维链上，
            // 正文可能是空串。空译文绝不能当成成功——否则会把空内容粘进输入框，
            // 把用户辛苦打的中文直接清掉。
            if result.is_empty() {
                let reason = choice.finish_reason.unwrap_or_default();
                return Err(if reason == "length" {
                    format!(
                        "模型没有返回译文，输出被截断（finish_reason=length）。\
                         当前模型 {} 可能是推理模型，把 max_tokens 全用在了思考过程上；\
                         请换用 deepseek-chat 这类非推理模型。",
                        config.model
                    )
                } else {
                    format!("模型返回了空译文（finish_reason={}）", reason)
                });
            }

            // 去除可能误加的外层双引号
            let cleaned = result.trim_matches('"').trim_matches('\'').to_string();
            Ok(cleaned)
        } else {
            Err("No translation choices returned".to_string())
        }
    }

    /// 免费免 Key 备用翻译引擎 (用于开箱即测，采用 POST 表单提交杜绝 414 URI Too Long)
    pub async fn translate_free_fallback(
        &self,
        text: &str,
        target_lang: &str,
    ) -> Result<String, String> {
        let trimmed = text.trim();
        // 智能保护：如果是网址 URL，避免破坏链接结构，原样返回
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            return Ok(trimmed.to_string());
        }

        let langpair = match target_lang.to_lowercase().as_str() {
            "english" | "en" => "zh-CN|en",
            "japanese" | "ja" | "jp" => "zh-CN|ja",
            "korean" | "ko" | "kr" => "zh-CN|ko",
            "russian" | "ru" => "zh-CN|ru",
            "spanish" | "es" => "zh-CN|es",
            "german" | "de" => "zh-CN|de",
            _ => "zh-CN|en",
        };

        let params = [
            ("q", trimmed),
            ("langpair", langpair),
        ];

        let response = self
            .client
            .post("https://api.mymemory.translated.net/get")
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("Free translation request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(format!("Free API returned error status: {}", status));
        }

        let parsed: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Parse error: {}", e))?;

        if let Some(trans) = parsed["responseData"]["translatedText"].as_str() {
            let cleaned = trans.trim().to_string();
            if !cleaned.is_empty() {
                return Ok(cleaned);
            }
        }

        Err("Failed to parse free translation result".to_string())
    }
}
