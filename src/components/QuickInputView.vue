<template>
  <div class="quick-bar-wrapper" @keydown.esc="closeQuickBar">
    <div class="quick-card">
      <div class="input-row">
        <span class="prefix-tag" :class="{ 'game-tag': config.is_game_mode }">
          {{ config.is_game_mode ? '🎮 游戏' : '💬 社交' }} → {{ config.target_lang }}
        </span>
        <input
          ref="inputRef"
          v-model="sourceText"
          type="text"
          placeholder="输入中文 (按 Enter 自动翻译并发送到游戏/聊天窗口)..."
          @input="handleInput"
          @keydown.enter.exact.prevent="handleSubmit(true)"
          @keydown.shift.enter.exact.prevent="handleSubmit(false)"
          autofocus
        />
        <div v-if="loading" class="spinner"></div>
      </div>

      <!-- 实时翻译预览 -->
      <div class="preview-row" v-if="translatedText || loading">
        <span class="preview-label">翻译预览:</span>
        <span class="preview-text" :class="{ 'loading-pulse': loading }">
          {{ translatedText || '正在翻译中...' }}
        </span>
      </div>

      <!-- 底部快捷键提示 -->
      <div class="hints-row">
        <span><kbd>Enter</kbd> 注入并发送</span>
        <span><kbd>Shift+Enter</kbd> 仅填入不发送</span>
        <span><kbd>Esc</kbd> 取消并隐藏</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, nextTick } from 'vue';
import { invoke } from '@tauri-apps/api/core';

interface TranslationConfig {
  api_base: String;
  api_key: String;
  model: String;
  target_lang: String;
  is_game_mode: boolean;
}

const inputRef = ref<HTMLInputElement | null>(null);
const sourceText = ref('');
const translatedText = ref('');
const loading = ref(false);
const config = ref<TranslationConfig>({
  api_base: '',
  api_key: '',
  model: '',
  target_lang: 'English',
  is_game_mode: true,
});

let debounceTimer: any = null;

onMounted(async () => {
  try {
    config.value = await invoke('get_config');
  } catch (e) {
    console.error(e);
  }
  nextTick(() => {
    inputRef.value?.focus();
  });
});

const handleInput = () => {
  clearTimeout(debounceTimer);
  if (!sourceText.value.trim()) {
    translatedText.value = '';
    loading.value = false;
    return;
  }

  loading.value = true;
  debounceTimer = setTimeout(async () => {
    try {
      const res = await invoke<string>('translate', { text: sourceText.value });
      translatedText.value = res;
    } catch (e: any) {
      translatedText.value = `[翻译失败: ${e}]`;
    } finally {
      loading.value = false;
    }
  }, 280);
};

const handleSubmit = async (autoEnter: boolean) => {
  let textToSend = translatedText.value.trim();
  if (!textToSend && sourceText.value.trim()) {
    // 如果还没翻译完就按了回车，先同步等待翻译结果
    loading.value = true;
    try {
      textToSend = await invoke<string>('translate', { text: sourceText.value });
    } catch (e) {
      textToSend = sourceText.value;
    } finally {
      loading.value = false;
    }
  }

  if (textToSend) {
    await invoke('inject_and_send', {
      text: textToSend,
      autoEnter: autoEnter,
    });
    sourceText.value = '';
    translatedText.value = '';
  } else {
    closeQuickBar();
  }
};

const closeQuickBar = async () => {
  await invoke('toggle_quick_input');
};
</script>

<style scoped>
.quick-bar-wrapper {
  width: 100vw;
  height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  user-select: none;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  overflow: hidden;
}

.quick-card {
  width: 95%;
  background: rgba(18, 20, 29, 0.92);
  backdrop-filter: blur(20px);
  border: 1px solid rgba(80, 160, 255, 0.35);
  box-shadow: 0 12px 35px rgba(0, 0, 0, 0.65), 0 0 20px rgba(59, 130, 246, 0.2);
  border-radius: 12px;
  padding: 10px 14px;
  display: flex;
  flex-direction: column;
  gap: 6px;
  box-sizing: border-box;
}

.input-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.prefix-tag {
  background: rgba(255, 255, 255, 0.1);
  color: #94a3b8;
  font-size: 11px;
  font-weight: 600;
  padding: 4px 8px;
  border-radius: 6px;
  white-space: nowrap;
}

.prefix-tag.game-tag {
  background: rgba(16, 185, 129, 0.2);
  color: #34d399;
  border: 1px solid rgba(16, 185, 129, 0.4);
}

input {
  flex: 1;
  background: transparent;
  border: none;
  outline: none;
  color: #f8fafc;
  font-size: 15px;
  font-weight: 500;
}

input::placeholder {
  color: #64748b;
  font-size: 13px;
}

.spinner {
  width: 14px;
  height: 14px;
  border: 2px solid rgba(255, 255, 255, 0.15);
  border-top-color: #38bdf8;
  border-radius: 50%;
  animation: spin 0.7s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.preview-row {
  display: flex;
  align-items: center;
  gap: 8px;
  background: rgba(30, 41, 59, 0.6);
  padding: 6px 10px;
  border-radius: 6px;
  font-size: 13px;
  border-left: 3px solid #38bdf8;
}

.preview-label {
  color: #94a3b8;
  font-size: 11px;
  font-weight: 600;
}

.preview-text {
  color: #38bdf8;
  font-weight: 500;
}

.loading-pulse {
  opacity: 0.6;
}

.hints-row {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  color: #64748b;
  font-size: 10px;
}

kbd {
  background: rgba(255, 255, 255, 0.1);
  padding: 1px 4px;
  border-radius: 3px;
  color: #cbd5e1;
  font-family: inherit;
}
</style>
