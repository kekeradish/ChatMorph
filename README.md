# 微语 (ChatMorph) - 原生聊天翻译器

> 极速 · 原生 · 零封号 · 打完字直接按键原地变身外语

[![Release](https://img.shields.io/github/v/release/kekeradish/ChatMorph?label=最新测试版&color=brightgreen)](https://github.com/kekeradish/ChatMorph/releases/tag/v0.1.0-beta)
[![Download](https://img.shields.io/badge/立即下载-v0.1.0测试版-blue)](https://github.com/kekeradish/ChatMorph/releases/tag/v0.1.0-beta)

基于 **Tauri v2 + Rust + Vue 3 (TypeScript)** 打造的极轻量、毫秒级响应、零封号风险的桌面端聊天翻译工具。

专为 **PC 竞技网游（LoL、CS2、Apex、Valorant、DOTA2、瓦罗兰特等）** 与 **桌面社交软件（微信、QQ、Discord、Telegram、Twitter/X 等）** 量身定制。

---

## ✨ 核心特性

1. **原地无感变身（In-Place Morphing）**：
   - 打完中文后，**无需手动划选或全选文本**，直接按下默认热键 `Alt + ~`（支持自定义为 `Alt + A` 等）；
   - 底层原生按键流水线自动秒级提取输入框文本、调用翻译引擎，并在当前输入框瞬间覆盖更新为地道外语！
2. **绝对安全零封号（Non-invasive & Anti-Cheat Safe）**：
   - 100% 外部原生模拟交互，绝不向游戏注入 DLL，不读取游戏内存，彻底杜绝 VAC / EAC / Vanguard / BattlEye 反作弊封禁风险。
3. **极速时钟与静音体验**：
   - 优化 Windows 硬件消息事件队列，按键时延压缩至 8~10ms，全流程响应仅需 70~90ms；
   - 移除所有系统提示蜂鸣音，安静无打扰。
4. **淡色系与深色系双主题自由切换**：
   - 顶部状态栏一键在 **🌙 深色系**（黑金科技电竞感）与 **☀️ 淡色系**（雅致清爽米白纸面感）之间随心切换，支持本地持久化记忆。
5. **竞技网游黑话与缩写专属增强（Gaming Slang Engine）**：
   - 内置游戏报点与电竞俚语 Prompt（如：开大->`Ult`，抓人->`Gank`，守高地->`Defend HG`，别送->`Stop feeding`，发把大狙->`Drop me an AWP`，残血->`Low HP/one shot`）。
6. **多翻译引擎自由配置**：
   - 支持 DeepSeek、硅基流动 (SiliconFlow)、OpenAI、本地 Ollama 离线大模型；
   - 内置免费免 Key 应急降级通道，开箱即用。

---

## ⌨️ 常用快捷键与操作

| 动作 | 默认热键 / 操作 | 说明 |
| :--- | :--- | :--- |
| **原地变身** | **`Alt + ~`** | 在微信/游戏/网页输入框打完字后直接按，自动变身为地道外语 |
| **主题切换** | 点击右上角按钮 | 实时在深色系与淡色系之间无缝切换 |
| **模式切换** | 点击模式卡片 | 自由切换「🎮 游戏报点模式」与「💬 口语聊天模式」 |

---

## 🚀 启动与构建

在项目根目录下：

### 1. 启动桌面端调试
```bash
npm run tauri dev
```

### 2. 打包生成 Release 单文件安装包
```bash
npm run tauri build
```

---

## 🛠️ 技术架构

- **桌面宿主**：[Tauri v2](https://v2.tauri.app/) (Rust 2021)
- **前端界面**：Vue 3 + Vite + TypeScript + Lucide Icons
- **Windows 底层原生支持**：`windows-sys` (UIA 自动化探查 + SendInput 硬件按键模拟流水线)
