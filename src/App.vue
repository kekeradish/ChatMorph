<template>
  <div class="app-root">
    <MainView />
  </div>
</template>

<script setup lang="ts">
// 现在只有主窗口一个界面。
// QuickInputView / HudOverlayView 两个组件的文件仍保留在 components 下，
// 但它们对应的窗口已经从 tauri.conf.json 删掉了：
//   输入条 —— 后端命令从未注册，整条链路是断的；
//   HUD 浮层 —— 里面 3 条是写死的假数据，没有 OCR 数据源。
// 以后要恢复某个浮层：在 tauri.conf.json 加回窗口，并在这里按 ?win=xxx 加回分支即可。
import MainView from './components/MainView.vue';
</script>

<style>
/* 全局基础重置 */
/* 关键：卡片用 grid 拉伸高度时，如果还是 content-box，padding 会额外加到高度上，
   卡片实际渲染比网格行高出 32px，底部内容会被裁掉（日志面板"看不到底"就是这个原因）。 */
*, *::before, *::after {
  box-sizing: border-box;
}

html, body {
  margin: 0;
  padding: 0;
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: transparent;
  -webkit-font-smoothing: antialiased;
}

#app, .app-root {
  width: 100%;
  height: 100%;
}
</style>