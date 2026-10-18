<template>
  <div class="hud-container" :class="{ 'is-locked': isLocked }">
    <!-- 顶部拖拽栏与锁定控制 -->
    <div class="hud-header" data-tauri-drag-region>
      <div class="hud-title" data-tauri-drag-region>
        <span class="pulse-dot"></span>
        <span>游戏聊天翻译 HUD</span>
      </div>
      <div class="hud-actions">
        <button
          class="lock-btn"
          :class="{ active: isLocked }"
          :title="isLocked ? '已开启鼠标穿透 (点击解除锁定)' : '点击开启鼠标穿透 (不挡游戏操作)'"
          @click="toggleClickThrough"
        >
          {{ isLocked ? '🔒 穿透模式' : '🔓 拖拽调整' }}
        </button>
      </div>
    </div>

    <!-- 消息滚动列表 -->
    <div class="message-stream">
      <div v-if="messages.length === 0" class="empty-tip">
        等待游戏聊天框新消息滚动或手动输入测试...
      </div>
      <div
        v-for="(msg, index) in messages"
        :key="index"
        class="chat-bubble"
      >
        <div class="bubble-meta">
          <span class="user-badge">{{ msg.sender || 'Player' }}</span>
          <span class="time-stamp">{{ msg.time }}</span>
        </div>
        <div class="orig-text">{{ msg.original }}</div>
        <div class="trans-text">{{ msg.translated }}</div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';

interface ChatItem {
  sender?: string;
  original: string;
  translated: string;
  time: string;
}

const isLocked = ref(false);

const messages = ref<ChatItem[]>([
  {
    sender: 'Teammate',
    original: 'Rush B don\'t stop, drop me an AWP!',
    translated: '快冲 B 点别停，给我扔一把大狙！',
    time: '21:30',
  },
  {
    sender: 'Opponent',
    original: 'mid diff, ff15 pls',
    translated: '中单差距太大了，15投了吧',
    time: '21:31',
  },
  {
    sender: 'Support',
    original: 'Baron is up, defend base first',
    translated: '大龙刷新了，先守住高地基座',
    time: '21:32',
  },
]);

const toggleClickThrough = async () => {
  isLocked.value = !isLocked.value;
  try {
    await invoke('set_hud_click_through', { enable: isLocked.value });
  } catch (e) {
    console.error('切换穿透失败', e);
  }
};
</script>

<style scoped>
.hud-container {
  width: 100vw;
  height: 100vh;
  box-sizing: border-box;
  padding: 8px;
  background: rgba(10, 15, 26, 0.75);
  backdrop-filter: blur(12px);
  border: 1px solid rgba(56, 189, 248, 0.3);
  border-radius: 10px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  color: #f1f5f9;
}

.hud-container.is-locked {
  border-color: rgba(16, 185, 129, 0.4);
  background: rgba(10, 15, 26, 0.55);
}

.hud-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-bottom: 6px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  cursor: grab;
  user-select: none;
}

.hud-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  font-weight: 600;
  color: #94a3b8;
  letter-spacing: 0.5px;
}

.pulse-dot {
  width: 7px;
  height: 7px;
  background: #10b981;
  border-radius: 50%;
  box-shadow: 0 0 6px #10b981;
}

.lock-btn {
  background: rgba(255, 255, 255, 0.1);
  border: 1px solid rgba(255, 255, 255, 0.15);
  color: #cbd5e1;
  font-size: 10px;
  padding: 2px 8px;
  border-radius: 4px;
  cursor: pointer;
  transition: all 0.2s;
}

.lock-btn.active {
  background: rgba(16, 185, 129, 0.25);
  border-color: rgba(16, 185, 129, 0.5);
  color: #34d399;
}

.message-stream {
  flex: 1;
  overflow-y: auto;
  margin-top: 6px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.empty-tip {
  color: #64748b;
  font-size: 11px;
  text-align: center;
  margin-top: 30px;
}

.chat-bubble {
  background: rgba(15, 23, 42, 0.65);
  border-left: 3px solid #38bdf8;
  border-radius: 4px 6px 6px 4px;
  padding: 5px 8px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.bubble-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 10px;
}

.user-badge {
  color: #a5b4fc;
  font-weight: 600;
}

.time-stamp {
  color: #475569;
}

.orig-text {
  font-size: 12px;
  color: #cbd5e1;
  word-break: break-all;
}

.trans-text {
  font-size: 12px;
  color: #38bdf8;
  font-weight: 500;
  word-break: break-all;
}
</style>
