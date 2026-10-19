<template>
  <div class="main-dashboard" :class="`theme-${theme}`">
    <!-- 顶部状态栏 -->
    <header class="header">
      <div class="brand">
        <div class="logo-badge">⚡</div>
        <div class="brand-title-row">
          <h1>微语 (ChatMorph)</h1>
          <span class="version-tag">v0.2.0 测试版</span>
        </div>
        <p class="sub">原生级游戏聊天与社交原地变身翻译引擎</p>
      </div>
      <div class="header-status">
        <button class="theme-btn" @click="toggleTheme" :title="theme === 'dark' ? '点击切换为清新淡色系' : '点击切换为暗夜深色系'">
          {{ theme === 'dark' ? '☀️ 淡色系' : '🌙 深色系' }}
        </button>
        <div class="status-pill">
          <span class="dot online"></span>
          <span>呼出热键: <kbd>{{ formatHotkey(config.hotkey) }}</kbd></span>
        </div>
      </div>
    </header>

    <!-- 主体两列布局 -->
    <main class="content-grid">
      <!-- 左列：实时翻译调试与快捷操作 -->
      <section class="card test-card">
        <div class="card-header">
          <h2>💬 实时翻译与注入测试</h2>
          <span class="badge" :class="{ game: config.is_game_mode }">
            {{ config.is_game_mode ? '🎮 游戏报点模式' : '💬 口语聊天模式' }}
          </span>
        </div>

        <div class="input-group">
          <label>输入源文本 (中文)：</label>
          <div class="quick-preset-chips">
            <span class="chip" @click="setPreset('中单不见了小心抓人')">中单不见了</span>
            <span class="chip" @click="setPreset('别送了，先守住高地')">守高地</span>
            <span class="chip" @click="setPreset('快来打大龙')">打大龙</span>
            <span class="chip" @click="setPreset('给我扔一把大狙')">发把大狙</span>
          </div>
          <textarea
            v-model="testInput"
            placeholder="输入你想在游戏或社交媒体中说的话..."
            rows="2"
          ></textarea>
        </div>

        <div class="action-row">
          <button class="accent-btn" :disabled="translating" @click="doTestTranslate">
            {{ translating ? '正在翻译...' : '立即翻译' }}
          </button>
        </div>

        <div class="test-countdown-box">
          <button class="countdown-btn" :disabled="countdownActive" @click="startInPlaceTest">
            {{ countdownActive ? `⏱️ 倒计时中！请点击到目标聊天框 (${countdown}s)...` : '⏱️ 3 秒后测试原地替换 (最直观测试)' }}
          </button>
          <span class="field-hint">点击后立刻点进游戏/微信输入框打字，3 秒后自动全选替换。</span>
        </div>

        <div class="result-box" v-if="testOutput">
          <div class="result-header">
            <span>
              翻译结果 ({{ resultLang }})
              <span v-if="resultLang !== config.target_lang" class="stale-tip">
                · 目标语言已改为 {{ config.target_lang }}，重新翻译后生效
              </span>
            </span>
            <span class="copy-hint" @click="copyResult">点击复制</span>
          </div>
          <div class="result-content">{{ testOutput }}</div>
        </div>

        <!-- 实时转换监控日志 -->
        <div class="log-console-box">
          <div class="log-header">
            <span>🖥️ 转换链路实时监控</span>
            <span class="clear-log-btn" @click="logs = []">清空</span>
          </div>
          <div class="log-body" ref="logBodyRef">
            <div v-if="logs.length === 0" class="log-empty">
              按下快捷键或点击测试后，此处将实时展示每一个步骤（按键捕获、文本提取、网络调用、文本回填）...
            </div>
            <div v-for="(log, i) in logs" :key="i" class="log-line">
              {{ log }}
            </div>
          </div>
        </div>
      </section>

      <!-- 右列：API 与翻译模型配置 -->
      <section class="card config-card">
        <div class="card-header">
          <h2>⚙️ 引擎配置与全局热键</h2>
          <div class="header-right">
            <span v-if="saveState !== 'idle'" class="save-state" :class="saveState">
              {{ saveState === 'saving' ? '保存中…' : saveState === 'saved' ? '✓ 已保存' : '✗ 保存失败' }}
            </span>
            <button class="save-btn" @click="flushSave">立即保存</button>
          </div>
        </div>

        <div class="form-body" @focusout="onFormFocusOut">
          <!-- ===== ① 翻译设置：翻成什么语言、用哪个引擎 ===== -->
          <div class="group-title">翻译设置</div>

          <div class="form-row-2">
            <div class="form-item">
              <label>目标语言：</label>
              <select v-model="config.target_lang">
                <option value="English">英语 (English)</option>
                <option value="Japanese">日语 (Japanese)</option>
                <option value="Korean">韩语 (Korean)</option>
                <option value="Russian">俄语 (Russian)</option>
                <option value="Spanish">西班牙语 (Spanish)</option>
                <option value="German">德语 (German)</option>
              </select>
            </div>

            <div class="form-item">
              <label>翻译引擎：</label>
              <select v-model="config.provider">
                <option value="deepseek">DeepSeek 官方</option>
                <option value="siliconflow">硅基流动 SiliconFlow</option>
                <option value="openai">OpenAI</option>
                <option value="ollama">本地 Ollama</option>
                <option value="free">内置免费通道</option>
              </select>
            </div>
          </div>

          <!-- 内置免费通道：不需要任何配置 -->
          <div v-if="isFreeChannel" class="channel-note free">
            <span class="note-title">🌐 内置免费通道，无需填写任何内容</span>
            <span class="note-sub">
              直接可用，走 MyMemory 公共机翻接口。代价：质量明显弱于大模型、有频率限制，且你输入的文本会发送给第三方服务。
            </span>
          </div>

          <template v-else>
            <div class="form-row-2">
              <div class="form-item">
                <label>API Base URL：</label>
                <input v-model="config.api_base" type="text" :placeholder="providerPreset.api_base" />
              </div>
              <div class="form-item">
                <label>模型名称：</label>
                <input v-model="config.model" type="text" :placeholder="providerPreset.model" />
              </div>
            </div>

            <!-- 本地 Ollama 不需要 Key -->
            <div v-if="!needsKey" class="channel-note neutral">
              <span class="note-title">🖥️ 本地 Ollama 不需要 API Key</span>
              <span class="note-sub">请确认 <code>ollama serve</code> 已在运行，并提前 <code>ollama pull</code> 好对应模型。</span>
            </div>

            <div v-else class="form-item">
              <label>API Key：</label>
              <div class="password-input">
                <input
                  :type="showApiKey ? 'text' : 'password'"
                  v-model="config.api_key"
                  placeholder="sk-..."
                />
                <button class="icon-toggle" @click="showApiKey = !showApiKey">
                  {{ showApiKey ? '🙈' : '👁️' }}
                </button>
              </div>
              <span class="field-hint">Key 只存在本机配置文件里；留空会自动降级到内置免费通道。</span>
            </div>

            <div class="conn-row">
              <button class="test-conn-btn" :disabled="testing" @click="testConnection">
                {{ testing ? '测试中…' : '🔌 测试连接' }}
              </button>
              <span v-if="connResult" class="conn-result" :class="{ ok: connOk }">{{ connResult }}</span>
              <span v-else class="field-hint">填完先点这里确认，比进游戏试错快得多</span>
            </div>
          </template>

          <!-- 两个模式：唯一差别是发给模型的 system prompt -->
          <div class="form-item">
            <label>翻译风格（两种模式只差一段提示词，模型和速度完全一样）：</label>
            <div class="mode-switch">
              <button
                class="mode-btn"
                :class="{ active: config.is_game_mode }"
                @click="setMode(true)"
              >
                🎮 游戏报点
              </button>
              <button
                class="mode-btn"
                :class="{ active: !config.is_game_mode }"
                @click="setMode(false)"
              >
                💬 口语聊天
              </button>
            </div>
            <span class="field-hint">
              {{
                config.is_game_mode
                  ? '5~9 个词，用 Ult / Gank / HG / B 这类缩写，团战里一眼读完。例：我残血了先回城，你守高地 → "I\'m low HP, B first; you defend HG."'
                  : '自然完整的句子，适合 Discord / 微信私聊，游戏里也凑合能用。例：这个价格能不能再便宜一点 → "Could you do a little better on the price?"'
              }}
            </span>
          </div>

          <!-- ===== ② 快捷替换：怎么触发、触发后做什么 ===== -->
          <div class="group-title">快捷替换（在游戏原生输入框里用）</div>

          <div class="form-item hotkey-box">
            <label>🎮 原地翻译快捷键（点下方框，然后直接按下你想要的组合键）：</label>
            <div class="hotkey-row">
              <div
                class="hotkey-recorder"
                :class="{ recording }"
                tabindex="0"
                @focus="startRecording"
                @blur="stopRecording"
                @keydown="onHotkeyKeydown"
              >
                <span v-if="recording" class="rec-hint">请按下组合键…（Esc 取消）</span>
                <kbd v-else class="rec-value">{{ formatHotkey(config.hotkey) }}</kbd>
              </div>
              <button class="reset-hk" @click="resetHotkey">恢复默认</button>
            </div>
            <span class="field-hint">
              💡 默认 <strong>Alt + ~</strong>（ESC 正下方那颗键）。必须带 Ctrl / Alt / Shift，避免和正常打字冲突。
            </span>
          </div>

          <div class="form-item switch-item">
            <div>
              <span class="switch-title">⚡ 自动全选替换 (推荐开启)</span>
              <p class="switch-sub">打完字直接按热键，自动全选整句话并替换；关闭后需先手动划选文字。</p>
            </div>
            <input type="checkbox" v-model="config.auto_select_all" />
          </div>

          <div class="form-item switch-item">
            <div>
              <span class="switch-title">🚀 替换后自动回车发送</span>
              <p class="switch-sub">替换完立刻按 Enter 发出；关闭时留在输入框供你确认。</p>
            </div>
            <input type="checkbox" v-model="config.auto_send" />
          </div>

          <div class="form-item switch-item">
            <div>
              <span class="switch-title">🔤 保留结尾标点符号</span>
              <p class="switch-sub">开启时保留译文末尾句号/感叹号；关闭时自动去除句末标点，更契合游戏聊天随性习惯。</p>
            </div>
            <input type="checkbox" v-model="config.keep_trailing_punct" />
          </div>

          <div class="form-item switch-item">
            <div>
              <span class="switch-title">⚙️ 开机自启动 (后台常驻)</span>
              <p class="switch-sub">开机自动在后台运行微语，随时随地打字按热键即变身（点右上角 X 同样最小化到托盘）。</p>
            </div>
            <input type="checkbox" v-model="config.autostart" />
          </div>
        </div>
      </section>
    </main>

    <!-- 底部通知栏 -->
    <footer class="footer-tip">
      💡 极致工作流：在游戏聊天框、Discord、微信打出中文，直接按 <strong>{{ formatHotkey(config.hotkey) }}</strong>，文字瞬间原地变身为地道外语！
    </footer>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch, computed, nextTick } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

interface TranslationConfig {
  provider: string;
  api_base: string;
  api_key: string;
  model: string;
  target_lang: string;
  is_game_mode: boolean;
  hotkey: string;
  auto_select_all: boolean;
  auto_send: boolean;
  keep_trailing_punct: boolean;
  autostart: boolean;
}

const config = ref<TranslationConfig>({
  provider: 'deepseek',
  api_base: 'https://api.deepseek.com/v1',
  api_key: '',
  model: 'deepseek-chat',
  target_lang: 'English',
  is_game_mode: true,
  hotkey: 'Alt+Backquote',
  auto_select_all: true,
  auto_send: false,
  keep_trailing_punct: false,
  autostart: false,
});

// 每个服务商各自的 Base / Key / 模型，切换回来时原样恢复，不再被覆盖清空
interface ProviderDraft {
  api_base: string;
  api_key: string;
  model: string;
}

const PROVIDER_PRESETS: Record<string, { api_base: string; model: string; label: string }> = {
  deepseek: { api_base: 'https://api.deepseek.com/v1', model: 'deepseek-chat', label: 'DeepSeek 官方' },
  siliconflow: { api_base: 'https://api.siliconflow.cn/v1', model: 'deepseek-ai/DeepSeek-V3', label: '硅基流动' },
  openai: { api_base: 'https://api.openai.com/v1', model: 'gpt-4o-mini', label: 'OpenAI' },
  ollama: { api_base: 'http://localhost:11434/v1', model: 'qwen2.5:7b', label: '本地 Ollama' },
  free: { api_base: '', model: '', label: '内置免费通道' },
};

const isFreeChannel = computed(() => config.value.provider === 'free');
// 免费通道不填任何东西；Ollama 只需要地址和模型
const needsKey = computed(
  () => config.value.provider !== 'free' && config.value.provider !== 'ollama'
);
const providerPreset = computed(() => PROVIDER_PRESETS[config.value.provider] ?? PROVIDER_PRESETS.deepseek);

const logs = ref<string[]>([]);
const logBodyRef = ref<HTMLElement | null>(null);

const showApiKey = ref(false);
const testInput = ref('');
const testOutput = ref('');
// 记录这条结果是按哪个目标语言翻出来的，避免切了语言之后标签跟着变、看起来像数据错乱
const resultLang = ref('English');
const translating = ref(false);
const countdown = ref(3);
const countdownActive = ref(false);
const testing = ref(false);
const connResult = ref('');
const connOk = ref(false);

const DRAFT_KEY = 'gamechat_provider_drafts';

const readDrafts = (): Record<string, ProviderDraft> => {
  try {
    return JSON.parse(localStorage.getItem(DRAFT_KEY) || '{}');
  } catch {
    return {};
  }
};

const writeDrafts = (drafts: Record<string, ProviderDraft>) => {
  try {
    localStorage.setItem(DRAFT_KEY, JSON.stringify(drafts));
  } catch {}
};

// 切换服务商：先把当前填的存进草稿，再恢复目标服务商上次填的内容
const switchProvider = (next: string, prev: string) => {
  if (!next || next === prev) return;

  const drafts = readDrafts();
  if (prev) {
    drafts[prev] = {
      api_base: config.value.api_base,
      api_key: config.value.api_key,
      model: config.value.model,
    };
  }

  const saved = drafts[next];
  if (saved) {
    // 恢复到上次的填写状态（Key 不会再被清空）
    config.value.api_base = saved.api_base;
    config.value.api_key = saved.api_key;
    config.value.model = saved.model;
  } else {
    const preset = PROVIDER_PRESETS[next];
    if (preset && next !== 'free') {
      config.value.api_base = preset.api_base;
      config.value.model = preset.model;
    }
    // Key 保持不变：切到免费再切回来，原来填的 Key 还在
  }

  writeDrafts(drafts);
  // 换了引擎，上一次的连通性结果就不作数了
  connResult.value = '';
  connOk.value = false;
};

const startInPlaceTest = async () => {
  countdown.value = 3;
  countdownActive.value = true;
  await invoke('save_config', { newConfig: config.value });
  await invoke('test_in_place_after_delay', { delaySecs: 3 });

  const timer = setInterval(() => {
    countdown.value -= 1;
    if (countdown.value <= 0) {
      clearInterval(timer);
      countdownActive.value = false;
    }
  }, 1000);
};

const DEFAULT_HOTKEY = 'Alt+Backquote';
const recording = ref(false);

const MODIFIER_CODES = new Set([
  'ControlLeft', 'ControlRight', 'AltLeft', 'AltRight',
  'ShiftLeft', 'ShiftRight', 'MetaLeft', 'MetaRight',
]);
// 这些组合不能绑：程序内部要用它们做全选/复制/粘贴
const RESERVED_COMBOS = new Set(['Ctrl+KeyA', 'Ctrl+KeyC', 'Ctrl+KeyV', 'Ctrl+KeyX']);

const formatHotkey = (hk: string) => {
  if (!hk) return 'Alt + ~';
  return hk
    .replace(/Backquote/g, '~')
    .replace(/Key([A-Z0-9])/g, '$1')
    .replace(/Digit(\d)/g, '$1')
    .replace(/Super/g, 'Win')
    .replace(/Control/g, 'Ctrl')
    .replace(/\+/g, ' + ');
};

const startRecording = () => {
  recording.value = true;
};

const stopRecording = () => {
  recording.value = false;
};

const applyHotkey = async (combo: string) => {
  const prev = config.value.hotkey;
  config.value.hotkey = combo;
  try {
    await invoke('update_hotkey', { newHotkey: combo });
    recording.value = false;
    (document.activeElement as HTMLElement | null)?.blur();
  } catch (e: any) {
    config.value.hotkey = prev;
    // 后端未改动，回读一次确保界面与实际一致
    try {
      const saved = await invoke<TranslationConfig>('get_config');
      config.value.hotkey = saved.hotkey;
    } catch {}
    alert(`❌ 这个组合键注册不了（可能被系统或别的软件占用）：\n${e}`);
  }
};

const onHotkeyKeydown = async (e: KeyboardEvent) => {
  e.preventDefault();
  e.stopPropagation();

  if (e.key === 'Escape') {
    recording.value = false;
    (document.activeElement as HTMLElement | null)?.blur();
    return;
  }
  // 只按了修饰键，继续等真正的键
  if (MODIFIER_CODES.has(e.code) || !e.code) return;

  const parts: string[] = [];
  if (e.ctrlKey) parts.push('Ctrl');
  if (e.altKey) parts.push('Alt');
  if (e.shiftKey) parts.push('Shift');
  if (e.metaKey) parts.push('Super');

  const isFnKey = /^F\d{1,2}$/.test(e.code);
  if (parts.length === 0 && !isFnKey) {
    alert('请至少带一个 Ctrl / Alt / Shift 修饰键（或使用 F1~F12），否则会和正常打字冲突。');
    return;
  }

  const combo = [...parts, e.code].join('+');
  if (RESERVED_COMBOS.has(combo)) {
    alert(`${formatHotkey(combo)} 是程序自动替换时内部要用的组合键，请换一个。`);
    return;
  }
  await applyHotkey(combo);
};

const resetHotkey = () => applyHotkey(DEFAULT_HOTKEY);

// 日志有新增就滚到底，保证最新一步（替换完成/报错）始终可见
watch(
  () => logs.value.length,
  async () => {
    await nextTick();
    const el = logBodyRef.value;
    if (el) el.scrollTop = el.scrollHeight;
  }
);

const theme = ref<'dark' | 'light'>('dark');

const toggleTheme = () => {
  theme.value = theme.value === 'dark' ? 'light' : 'dark';
  localStorage.setItem('gamechat_theme_mode', theme.value);
};

// 老配置里没有 provider 字段时，是否由前端反推出来（需要补写一次磁盘）
let providerInferred = false;

onMounted(async () => {
  const savedTheme = localStorage.getItem('gamechat_theme_mode') as 'dark' | 'light' | null;
  if (savedTheme) {
    theme.value = savedTheme;
  }

  try {
    const saved = await invoke<TranslationConfig>('get_config');
    if (saved) {
      config.value = saved;
      // 老配置文件没有 provider 字段（后端会回 "auto"），按 api_base / Key 反推一次
      if (!PROVIDER_PRESETS[saved.provider]) {
        providerInferred = true;
        if (saved.api_base.includes('siliconflow')) config.value.provider = 'siliconflow';
        else if (saved.api_base.includes('openai')) config.value.provider = 'openai';
        else if (saved.api_base.includes('11434')) config.value.provider = 'ollama';
        else if (saved.api_key.trim() === '') config.value.provider = 'free';
        else config.value.provider = 'deepseek';
      }
      // 把当前填写内容存成该服务商的草稿，下次切回来能原样恢复
      const drafts = readDrafts();
      drafts[config.value.provider] = {
        api_base: config.value.api_base,
        api_key: config.value.api_key,
        model: config.value.model,
      };
      writeDrafts(drafts);
    }
  } catch (e) {
    console.error(e);
  }

  // 监听后端推送的实时转换执行链路日志
  // 改为「按时间顺序追加 + 自动滚到底」：一次触发有 6~8 行，
  // 以前是插到最前面且只留 25 行，导致最关键的最后一句（替换完成）永远被挤到看不见的地方。
  await listen<string>('translation_log', (event) => {
    const time = new Date().toLocaleTimeString();
    logs.value.push(`[${time}] ${event.payload}`);
    if (logs.value.length > 300) logs.value.splice(0, logs.value.length - 300);
  });

  // 加载期间不让 watch 触发保存
  configLoaded = true;

  // 首次渲染后把日志滚到底（虽然初始为空，但热更新/重挂载时可能有历史）
  await nextTick();
  if (logBodyRef.value) logBodyRef.value.scrollTop = logBodyRef.value.scrollHeight;


  // 老配置没有 provider 字段时是上面反推出来的，这里静默补一次落盘，
  // 否则要等你下次动一下配置，后端才知道自己走的是哪条通道。
  if (providerInferred) {
    invoke('save_config', { newConfig: config.value }).catch((e) => console.error(e));
  }
});

const saveState = ref<'idle' | 'saving' | 'saved' | 'error'>('idle');
let syncTimer: ReturnType<typeof setTimeout> | null = null;

// 立即落盘：清掉待执行的防抖任务，马上写一次并更新状态提示
const flushSave = async () => {
  if (syncTimer) {
    clearTimeout(syncTimer);
    syncTimer = null;
  }
  saveState.value = 'saving';
  try {
    await invoke('save_config', { newConfig: config.value });
    saveState.value = 'saved';
    setTimeout(() => {
      if (saveState.value === 'saved') saveState.value = 'idle';
    }, 1600);
  } catch (e) {
    console.error('配置保存失败', e);
    saveState.value = 'error';
  }
};

// 光标离开整个配置区时立刻落盘（在区内各字段之间切换不触发），
// 保证"填完 Key 直接按热键测试"时后端一定已经拿到最新值。
const onFormFocusOut = (e: FocusEvent) => {
  const next = e.relatedTarget as Node | null;
  if (next && (e.currentTarget as HTMLElement).contains(next)) return;
  flushSave();
};

// 兜底：任何输入框/下拉框的改动都会在 300ms 后自动同步，
// 防止"光标没离开就直接按热键"导致读到旧配置。
let configLoaded = false;
watch(
  config,
  (val) => {
    if (!configLoaded) return; // 初次加载配置不算"用户修改"，不触发保存
    if (syncTimer) clearTimeout(syncTimer);
    syncTimer = setTimeout(() => {
      saveState.value = 'saving';
      invoke('save_config', { newConfig: val })
        .then(() => {
          saveState.value = 'saved';
          setTimeout(() => {
            if (saveState.value === 'saved') saveState.value = 'idle';
          }, 1600);
        })
        .catch((e) => {
          console.error('配置同步失败', e);
          saveState.value = 'error';
        });
    }, 300);
  },
  { deep: true }
);

// 切换服务商时保留各自填过的内容（不能再用"重新赋值"的写法，
// 那会把用户手填的 Base / 自定义模型直接覆盖掉，切回去就没了）
watch(
  () => config.value.provider,
  (next, prev) => {
    if (!configLoaded) return;
    switchProvider(next, prev);
  }
);

const setPreset = (text: string) => {
  testInput.value = text;
};

// 切换翻译风格：只改 config.is_game_mode，后端据此选不同的 system prompt
const setMode = (gameMode: boolean) => {
  config.value.is_game_mode = gameMode;
  // 换风格等于换译文口径，旧的测试结果不再代表当前设置
  connResult.value = '';
};

const doTestTranslate = async () => {
  if (!testInput.value.trim()) return;
  translating.value = true;
  const langAtRequest = config.value.target_lang;
  try {
    await invoke('save_config', { newConfig: config.value });
    const res = await invoke<string>('translate', { text: testInput.value });
    testOutput.value = res;
    resultLang.value = langAtRequest;
  } catch (e: any) {
    testOutput.value = `[错误] ${e}`;
    resultLang.value = langAtRequest;
  } finally {
    translating.value = false;
  }
};



const testConnection = async () => {
  testing.value = true;
  connResult.value = '';
  try {
    // 先确保后端拿到当前填写的配置，再发起真实请求
    await invoke('save_config', { newConfig: config.value });
    const t0 = performance.now();
    const res = await invoke<string>('test_connection', { text: testInput.value.trim() || '你好，能收到吗？' });
    const ms = Math.round(performance.now() - t0);
    connOk.value = true;
    connResult.value = `✓ ${config.value.model} 连通，${ms}ms → “${res}”`;
  } catch (e: any) {
    connOk.value = false;
    connResult.value = `✗ ${e}`;
  } finally {
    testing.value = false;
  }
};

const copyResult = () => {
  navigator.clipboard.writeText(testOutput.value);
  alert('已复制到剪贴板！');
};
</script>

<style scoped>
/* ================= 全局主题变量系统 ================= */
.main-dashboard {
  --bg-app: #0b0f19;
  --bg-card: #111827;
  --border-card: rgba(255, 255, 255, 0.08);
  --bg-input: #1f2937;
  --border-input: rgba(255, 255, 255, 0.12);
  --text-main: #f1f5f9;
  --text-heading: #e2e8f0;
  --text-sub: #64748b;
  --text-muted: #94a3b8;
  --chip-bg: rgba(255, 255, 255, 0.05);
  --chip-border: rgba(255, 255, 255, 0.1);
  --chip-hover-bg: rgba(56, 189, 248, 0.2);
  --chip-hover-color: #38bdf8;
  --status-pill-bg: rgba(255, 255, 255, 0.05);
  --status-pill-border: rgba(255, 255, 255, 0.08);
  --kbd-bg: rgba(255, 255, 255, 0.1);
  --ghost-btn-bg: rgba(255, 255, 255, 0.08);
  --ghost-btn-color: #cbd5e1;
  --switch-bg: rgba(255, 255, 255, 0.03);
  --switch-border: rgba(255, 255, 255, 0.1);
  --divider: rgba(255, 255, 255, 0.08);
  --result-bg: #1e293b;
  --card-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);

  /* 日志控制台（原来是写死的深色，浅色模式下会突出一块黑框） */
  --log-bg: #090d16;
  --log-header-bg: rgba(30, 41, 59, 0.85);
  --log-border: rgba(59, 130, 246, 0.25);
  --log-divider: rgba(255, 255, 255, 0.05);
  --log-title: #93c5fd;
  --log-action: #94a3b8;
  --log-action-bg: rgba(255, 255, 255, 0.05);
  --log-text: #38bdf8;
  --log-text-new: #34d399;
  --log-empty: #64748b;
  --log-shadow: 0 4px 12px rgba(0, 0, 0, 0.25);

  width: 100vw;
  height: 100vh;
  box-sizing: border-box;
  background: var(--bg-app);
  color: var(--text-main);
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  display: flex;
  flex-direction: column;
  padding: 18px 24px;
  overflow: hidden;
  transition: background-color 0.25s ease, color 0.25s ease;
}

/* ================= 优雅淡色系主题 (Light Theme) ================= */
.main-dashboard.theme-light {
  --bg-app: #f1f5f9;
  --bg-card: #ffffff;
  --border-card: #e2e8f0;
  --bg-input: #f8fafc;
  --border-input: #cbd5e1;
  --text-main: #0f172a;
  --text-heading: #1e293b;
  --text-sub: #64748b;
  --text-muted: #475569;
  --chip-bg: #f8fafc;
  --chip-border: #e2e8f0;
  --chip-hover-bg: #e0f2fe;
  --chip-hover-color: #0284c7;
  --status-pill-bg: #e2e8f0;
  --status-pill-border: #cbd5e1;
  --kbd-bg: #cbd5e1;
  --ghost-btn-bg: #e2e8f0;
  --ghost-btn-color: #334155;
  --switch-bg: #f8fafc;
  --switch-border: #e2e8f0;
  --divider: #e2e8f0;
  --result-bg: #f8fafc;
  --card-shadow: 0 4px 20px rgba(0, 0, 0, 0.05);

  /* 浅色模式下的日志面板：淡底深字，不再是一块黑框 */
  --log-bg: #f8fafc;
  --log-header-bg: #eef2f7;
  --log-border: #dbe3ec;
  --log-divider: #e2e8f0;
  --log-title: #0369a1;
  --log-action: #64748b;
  --log-action-bg: #e8eef5;
  --log-text: #1e293b;
  --log-text-new: #047857;
  --log-empty: #94a3b8;
  --log-shadow: 0 1px 3px rgba(15, 23, 42, 0.06);
}

.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-bottom: 14px;
  border-bottom: 1px solid var(--divider);
  flex-shrink: 0;
}

.brand {
  display: flex;
  align-items: center;
  gap: 12px;
}

.logo-badge {
  font-size: 24px;
  width: 44px;
  height: 44px;
  background: linear-gradient(135deg, #0ea5e9, #3b82f6);
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 0 15px rgba(14, 165, 233, 0.35);
  color: white;
}

.brand-title-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.version-tag {
  font-size: 10px;
  font-weight: 700;
  padding: 1px 6px;
  border-radius: 4px;
  background: rgba(14, 165, 233, 0.15);
  color: #0284c7;
  border: 1px solid rgba(14, 165, 233, 0.35);
  letter-spacing: 0.2px;
}

.brand h1 {
  font-size: 18px;
  margin: 0;
  font-weight: 700;
  letter-spacing: -0.3px;
  color: var(--text-main);
}

.brand .sub {
  font-size: 12px;
  color: var(--text-sub);
  margin: 2px 0 0 0;
}

.header-status {
  display: flex;
  align-items: center;
  gap: 10px;
}

/* 主题切换按钮 */
.theme-btn {
  background: var(--ghost-btn-bg);
  color: var(--ghost-btn-color);
  border: 1px solid var(--border-card);
  padding: 6px 12px;
  border-radius: 20px;
  font-size: 12px;
  font-weight: 600;
  display: flex;
  align-items: center;
  gap: 4px;
  transition: all 0.2s;
}

.theme-btn:hover {
  filter: brightness(1.1);
  transform: translateY(-1px);
}

.status-pill {
  display: flex;
  align-items: center;
  gap: 6px;
  background: var(--status-pill-bg);
  border: 1px solid var(--status-pill-border);
  padding: 5px 10px;
  border-radius: 20px;
  font-size: 11px;
  color: var(--text-muted);
}

.dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
}

.dot.online {
  background: #10b981;
  box-shadow: 0 0 6px #10b981;
}

kbd {
  background: var(--kbd-bg);
  padding: 1px 5px;
  border-radius: 4px;
  color: var(--text-main);
}

button {
  border: none;
  cursor: pointer;
  font-size: 12px;
  font-weight: 600;
  padding: 6px 14px;
  border-radius: 6px;
  transition: all 0.2s;
}

.ghost-btn {
  background: var(--ghost-btn-bg);
  color: var(--ghost-btn-color);
}

.ghost-btn:hover {
  filter: brightness(1.15);
}

.primary-btn {
  background: #0284c7;
  color: white;
}

.primary-btn:hover {
  background: #0369a1;
}

.content-grid {
  flex: 1;
  display: grid;
  grid-template-columns: 1fr 1fr;
  /* 关键：行高必须被容器约束住，否则行高由卡片内容自动撑开，
     卡片内的 flex:1（日志面板）就拿不到"剩余高度"，内容超出后被外层直接裁掉。 */
  grid-template-rows: minmax(0, 1fr);
  gap: 18px;
  margin-top: 16px;
  min-height: 0;
}

.card {
  background: var(--bg-card);
  border: 1px solid var(--border-card);
  border-radius: 10px;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  box-shadow: var(--card-shadow);
  transition: background-color 0.25s ease, border-color 0.25s ease;
  min-height: 0;
  overflow-y: auto;
}

/* 配置项变多后两列高度不再对等，各自内部滚动，避免整页跟着跳 */
.card::-webkit-scrollbar {
  width: 6px;
}

.card::-webkit-scrollbar-thumb {
  background: var(--border-input);
  border-radius: 3px;
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-shrink: 0;
}

.group-title {
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.4px;
  color: var(--text-sub);
  text-transform: uppercase;
  padding-top: 2px;
  border-top: 1px dashed var(--divider);
  margin-top: 2px;
}

.group-title:first-child {
  border-top: none;
  margin-top: 0;
  padding-top: 0;
}

/* 免费通道 / 本地模型这类「不用填」的说明条 */
.channel-note {
  display: flex;
  flex-direction: column;
  gap: 3px;
  padding: 9px 11px;
  border-radius: 8px;
  border: 1px dashed var(--border-input);
  background: var(--switch-bg);
}

.channel-note.free {
  border-color: rgba(16, 185, 129, 0.4);
  background: rgba(16, 185, 129, 0.08);
}

.channel-note.neutral {
  border-color: rgba(56, 189, 248, 0.35);
  background: rgba(56, 189, 248, 0.07);
}

.note-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-heading);
}

.note-sub {
  font-size: 10px;
  line-height: 1.5;
  color: var(--text-sub);
}

.note-sub code {
  background: var(--kbd-bg);
  padding: 0 3px;
  border-radius: 3px;
}

.stale-tip {
  color: #f59e0b;
  font-weight: 600;
}

.card-header h2 {
  font-size: 14px;
  font-weight: 600;
  margin: 0;
  color: var(--text-heading);
}

.badge {
  font-size: 11px;
  padding: 3px 8px;
  border-radius: 12px;
  background: var(--chip-bg);
  color: var(--text-muted);
}

.badge.game {
  background: rgba(16, 185, 129, 0.15);
  color: #10b981;
  border: 1px solid rgba(16, 185, 129, 0.3);
}

.quick-preset-chips {
  display: flex;
  gap: 6px;
  margin: 6px 0;
}

.chip {
  font-size: 11px;
  background: var(--chip-bg);
  border: 1px solid var(--chip-border);
  padding: 3px 8px;
  border-radius: 12px;
  cursor: pointer;
  color: var(--text-muted);
  transition: all 0.2s;
}

.chip:hover {
  background: var(--chip-hover-bg);
  color: var(--chip-hover-color);
}

textarea, input[type="text"], input[type="password"], select {
  width: 100%;
  box-sizing: border-box;
  background: var(--bg-input);
  border: 1px solid var(--border-input);
  border-radius: 6px;
  padding: 8px 10px;
  color: var(--text-main);
  font-size: 13px;
  outline: none;
  transition: border-color 0.2s;
}

textarea:focus, input:focus, select:focus {
  border-color: #38bdf8;
}

.hotkey-box {
  background: rgba(56, 189, 248, 0.08);
  border: 1px solid rgba(56, 189, 248, 0.25);
  border-radius: 8px;
  padding: 10px;
}

.hotkey-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.hotkey-recorder {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 34px;
  padding: 6px 12px;
  box-sizing: border-box;
  background: var(--bg-input);
  border: 1px dashed var(--border-input);
  border-radius: 6px;
  cursor: pointer;
  outline: none;
  transition: all 0.2s;
  user-select: none;
}

.hotkey-recorder:hover {
  border-color: #38bdf8;
}

.hotkey-recorder:focus,
.hotkey-recorder.recording {
  border-color: #38bdf8;
  border-style: solid;
  background: rgba(56, 189, 248, 0.12);
  box-shadow: 0 0 0 3px rgba(56, 189, 248, 0.15);
}

.rec-value {
  font-size: 13px;
  font-weight: 700;
  letter-spacing: 0.5px;
  color: var(--text-main);
  background: transparent;
  padding: 0;
}

.rec-hint {
  font-size: 12px;
  font-weight: 600;
  color: #0284c7;
  animation: rec-blink 1.1s ease-in-out infinite;
}

@keyframes rec-blink {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.35; }
}

.reset-hk {
  background: var(--ghost-btn-bg);
  color: var(--ghost-btn-color);
  border: 1px solid var(--border-input);
  padding: 6px 10px;
  font-size: 11px;
  white-space: nowrap;
}

.reset-hk:hover {
  filter: brightness(1.15);
}

.action-row {
  display: flex;
  gap: 8px;
}

.test-countdown-box {
  background: rgba(16, 185, 129, 0.08);
  border: 1px dashed rgba(16, 185, 129, 0.35);
  border-radius: 8px;
  padding: 10px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.countdown-btn {
  background: #10b981;
  color: white;
  padding: 8px 12px;
  font-size: 13px;
  font-weight: 600;
  border-radius: 6px;
  transition: all 0.2s;
}

.countdown-btn:hover:not(:disabled) {
  background: #059669;
}

.countdown-btn:disabled {
  background: #94a3b8;
  cursor: not-allowed;
}

.accent-btn {
  background: #0284c7;
  color: white;
}

.accent-btn:hover {
  background: #0369a1;
}

.secondary-btn {
  background: var(--ghost-btn-bg);
  color: var(--ghost-btn-color);
}

.result-box {
  background: var(--result-bg);
  border: 1px solid rgba(56, 189, 248, 0.3);
  border-radius: 6px;
  padding: 10px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.result-header {
  display: flex;
  justify-content: space-between;
  font-size: 11px;
  color: var(--text-muted);
}

.copy-hint {
  color: #0284c7;
  cursor: pointer;
  font-weight: 600;
}

.result-content {
  color: #0284c7;
  font-size: 14px;
  font-weight: 600;
}

.form-body {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.form-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.form-item label {
  font-size: 11px;
  color: var(--text-muted);
}

.password-input {
  display: flex;
  position: relative;
}

.icon-toggle {
  position: absolute;
  right: 6px;
  top: 50%;
  transform: translateY(-50%);
  background: transparent;
  padding: 2px 4px;
  color: var(--text-muted);
}

.field-hint {
  font-size: 10px;
  color: var(--text-sub);
  line-height: 1.4;
}

.form-row-2 {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}

.switch-item {
  flex-direction: row;
  align-items: center;
  justify-content: space-between;
  background: var(--switch-bg);
  padding: 8px;
  border-radius: 6px;
  border: 1px dashed var(--switch-border);
}

.switch-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-heading);
}

/* 两档模式切换（游戏报点 / 口语聊天） */
.mode-switch {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 6px;
  padding: 3px;
  background: var(--switch-bg);
  border: 1px solid var(--switch-border);
  border-radius: 8px;
}

.mode-btn {
  background: transparent;
  color: var(--text-muted);
  border: 1px solid transparent;
  padding: 7px 8px;
  font-size: 12px;
  font-weight: 600;
  border-radius: 6px;
  transition: all 0.18s;
}

.mode-btn:hover:not(.active) {
  color: var(--text-main);
  background: var(--chip-bg);
}

.mode-btn.active {
  background: #0284c7;
  color: #fff;
  border-color: #0284c7;
  box-shadow: 0 2px 8px rgba(2, 132, 199, 0.3);
}

.switch-sub {
  font-size: 10px;
  color: var(--text-sub);
  margin: 2px 0 0 0;
}

.save-btn {
  background: #10b981;
  color: white;
  padding: 4px 10px;
  font-size: 11px;
}

.header-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.save-state {
  font-size: 10px;
  padding: 2px 7px;
  border-radius: 10px;
  white-space: nowrap;
  transition: all 0.2s;
}

.save-state.saving {
  background: rgba(148, 163, 184, 0.18);
  color: var(--text-muted);
}

.save-state.saved {
  background: rgba(16, 185, 129, 0.15);
  color: #10b981;
  border: 1px solid rgba(16, 185, 129, 0.3);
}

.save-state.error {
  background: rgba(239, 68, 68, 0.15);
  color: #ef4444;
  border: 1px solid rgba(239, 68, 68, 0.3);
}

.conn-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 4px;
  min-height: 20px;
}

.test-conn-btn {
  background: var(--ghost-btn-bg);
  color: var(--ghost-btn-color);
  border: 1px solid var(--border-input);
  padding: 3px 10px;
  font-size: 10px;
  white-space: nowrap;
}

.test-conn-btn:hover:not(:disabled) {
  filter: brightness(1.15);
}

.test-conn-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.conn-result {
  font-size: 10px;
  line-height: 1.4;
  color: #ef4444;
  word-break: break-all;
}

.conn-result.ok {
  color: #10b981;
}

.footer-tip {
  margin-top: 14px;
  font-size: 11px;
  color: var(--text-sub);
  text-align: center;
  padding-top: 10px;
  border-top: 1px solid var(--divider);
  flex-shrink: 0;
}

/* 实时监控日志面板样式（配色跟随主题，浅色模式下不给黑框） */
/* 面板吃掉左卡剩余高度，日志在内部滚动 —— 底部不再是"被窗口裁掉"的状态 */
.log-console-box {
  margin-top: 14px;
  flex: 1 1 0;
  min-height: 110px;
  display: flex;
  flex-direction: column;
  background: var(--log-bg);
  border: 1px solid var(--log-border);
  border-radius: 8px;
  overflow: hidden;
  box-shadow: var(--log-shadow);
  transition: background-color 0.25s ease, border-color 0.25s ease;
}

/* 左卡里除日志外的元素都不许被压缩 */
.test-card > *:not(.log-console-box) {
  flex-shrink: 0;
}

.log-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 12px;
  background: var(--log-header-bg);
  border-bottom: 1px solid var(--log-divider);
  font-size: 11px;
  font-weight: 600;
  color: var(--log-title);
}

.clear-log-btn {
  font-size: 10px;
  color: var(--log-action);
  cursor: pointer;
  padding: 2px 6px;
  border-radius: 4px;
  background: var(--log-action-bg);
  transition: all 0.2s;
}

.clear-log-btn:hover {
  color: #f87171;
  background: rgba(239, 68, 68, 0.15);
}

.log-body {
  padding: 8px 12px;
  flex: 1 1 auto;
  min-height: 0;
  overflow-y: auto;
  font-family: 'Cascadia Code', 'Fira Code', 'Consolas', monospace;
  font-size: 11px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.log-body::-webkit-scrollbar {
  width: 6px;
}

.log-body::-webkit-scrollbar-thumb {
  background: var(--border-input);
  border-radius: 3px;
}

.log-empty {
  color: var(--log-empty);
  font-style: italic;
  font-size: 11px;
  padding: 6px 0;
  text-align: center;
  margin: auto 0;
}

.log-line {
  color: var(--log-text);
  line-height: 1.4;
  word-break: break-all;
  white-space: pre-wrap;
}

/* 最新一条在底部，所以高亮最后一行（原来的 first-child 是给倒序用的） */
.log-line:last-child {
  color: var(--log-text-new);
  font-weight: 600;
}
</style>
