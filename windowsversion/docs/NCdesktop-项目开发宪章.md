# NCdesktop 项目开发宪章

> NoteCapt Desktop — macOS 原生桌面应用
> 版本：1.0 | 生效日期：2026-03-25

---

## 第一章 · 项目概述

### 1.1 产品定位

NCdesktop（NoteCapt Desktop）是一款面向 macOS 平台的原生桌面应用程序。产品名称 **NoteCapt** 暗示其核心功能围绕"笔记捕获"（Note + Capture），旨在为用户提供高效、优雅的信息捕获与笔记管理体验。

### 1.2 品牌标识

| 元素 | 值 |
|------|------|
| 产品名称 | NoteCapt |
| 品牌主色 | 海军蓝 `#1F456E` |
| 品牌强调色 | 金色 `#FFC000`（渐变：`#FFD54F` → `#FFC000`） |
| Logo 字体 | Arial / Helvetica / sans-serif, Bold |
| Logo 图形 | 文档边框 + 箭头/菱形播放符号 |

### 1.3 目标平台

- **主平台**: macOS 26 (Tahoe) 及以上
- **CPU 架构**: Apple Silicon (arm64) 优先，兼容 Intel (x86_64)
- **最低系统要求**: macOS 14 Sonoma（向下兼容目标）

---

## 第二章 · 开发环境

### 2.1 主机环境（当前）

| 项目 | 状态 |
|------|------|
| 操作系统 | macOS 26.3.1 (Build 25D2128) |
| 架构 | arm64 (Apple Silicon) |
| Shell | zsh |
| 包管理器 | Homebrew 4.4.20 |
| Xcode CLI | 已安装（`/Applications/Xcode.app/Contents/Developer`） |
| Node.js | **未安装 — 需要初始化** |

### 2.2 环境初始化步骤

```bash
# 1. 安装 Node.js 版本管理器
brew install fnm

# 2. 配置 shell（添加到 ~/.zshrc）
echo 'eval "$(fnm env --use-on-cd --shell zsh)"' >> ~/.zshrc
source ~/.zshrc

# 3. 安装 Node.js LTS
fnm install --lts
fnm use lts-latest

# 4. 启用 corepack（内置 pnpm 支持）
corepack enable

# 5. 安装 Rust 工具链（Tauri 必需）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2.3 IDE 配置

- **编辑器**: Cursor（基于 VS Code）
- **必备扩展**: Tauri、rust-analyzer、ESLint、Prettier、Tailwind CSS IntelliSense
- **格式化**: 保存时自动格式化，使用 Prettier
- **语言**: 所有代码注释和 commit 信息使用中文

---

## 第三章 · 技术架构

### 3.1 框架选型：Tauri v2

**选择 Tauri 而非 Electron 的理由：**

| 维度 | Tauri v2 | Electron |
|------|----------|----------|
| 冷启动 | < 0.5s | 1-2s |
| 内存占用（空闲） | 20-80MB | 100-300MB |
| 打包体积 | 3-10MB | 50-150MB |
| CPU 占用（空闲） | < 1% | 1-5% |
| 原生感 | 使用 macOS WKWebView，天然原生 | 内嵌 Chromium |
| 安全性 | Rust 后端 + 权限隔离 | Node.js 完全权限 |
| Liquid Glass 支持 | `tauri-plugin-liquid-glass` 原生支持 | 需第三方 `electron-liquid-glass` |

### 3.2 技术栈全景

```
┌─────────────────────────────────────────────────────────┐
│                    NCdesktop 架构                        │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌─── 前端层 (WebView) ─────────────────────────────┐  │
│  │  框架：React 19 + TypeScript 5.x                  │  │
│  │  构建：Vite 6                                     │  │
│  │  样式：Tailwind CSS v4 + CSS 自定义属性            │  │
│  │  状态：Zustand（轻量状态管理）                     │  │
│  │  路由：React Router v7（如需多页面）               │  │
│  │  图标：Lucide React                               │  │
│  └──────────────────────────────────────────────────┘  │
│                        ↕ IPC 通信                       │
│  ┌─── 后端层 (Rust Core) ───────────────────────────┐  │
│  │  框架：Tauri v2                                    │  │
│  │  数据库：SQLite（via rusqlite / sea-orm）          │  │
│  │  文件系统：Tauri fs 插件                           │  │
│  │  窗口效果：tauri-plugin-liquid-glass               │  │
│  │  系统集成：macOS 通知、菜单栏、全局快捷键          │  │
│  └──────────────────────────────────────────────────┘  │
│                        ↕                                │
│  ┌─── 系统层 (macOS) ──────────────────────────────┐  │
│  │  WKWebView · NSGlassEffectView · NSVisualEffect  │  │
│  │  Spotlight 集成 · 文件系统 · 通知中心              │  │
│  └──────────────────────────────────────────────────┘  │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### 3.3 项目目录结构规范

```
NCdesktop/
├── src/                          # 前端源码（React + TypeScript）
│   ├── assets/                   # 静态资源（图片、字体、SVG）
│   ├── components/               # UI 组件
│   │   ├── common/               # 通用组件（Button、Input、Modal 等）
│   │   ├── layout/               # 布局组件（Sidebar、Toolbar、TitleBar）
│   │   └── features/             # 功能组件（NoteEditor、NoteList 等）
│   ├── hooks/                    # 自定义 React Hooks
│   ├── stores/                   # Zustand 状态管理
│   ├── styles/                   # 全局样式 + Tailwind 配置
│   │   ├── globals.css           # 全局 CSS + 设计令牌
│   │   └── glass.css             # 毛玻璃效果专用样式
│   ├── lib/                      # 工具函数和辅助模块
│   ├── types/                    # TypeScript 类型定义
│   ├── App.tsx                   # 根组件
│   └── main.tsx                  # 入口文件
├── src-tauri/                    # Tauri 后端（Rust）
│   ├── src/
│   │   ├── main.rs               # Rust 入口
│   │   ├── commands/             # Tauri IPC 命令
│   │   ├── models/               # 数据模型
│   │   ├── db/                   # 数据库操作
│   │   └── utils/                # Rust 工具函数
│   ├── Cargo.toml                # Rust 依赖
│   ├── tauri.conf.json           # Tauri 配置
│   ├── capabilities/             # Tauri 权限配置
│   └── icons/                    # 应用图标
├── public/                       # 公共静态资源
├── .cursor/
│   └── rules/                    # Cursor AI 规则文件
├── package.json
├── pnpm-lock.yaml
├── tsconfig.json
├── vite.config.ts
├── tailwind.config.ts
├── postcss.config.js
└── README.md
```

---

## 第四章 · 编码规范

### 4.1 语言与命名

| 范畴 | 规则 |
|------|------|
| 前端语言 | TypeScript（严格模式，禁止 `any`） |
| 后端语言 | Rust（stable 通道） |
| 组件命名 | PascalCase（`NoteEditor.tsx`） |
| 函数/变量 | camelCase（`getNoteById`） |
| CSS 类名 | kebab-case 或 Tailwind 实用类 |
| Rust 函数 | snake_case（`get_note_by_id`） |
| 文件名 | 组件用 PascalCase，其余用 kebab-case |
| 常量 | UPPER_SNAKE_CASE（`MAX_NOTE_LENGTH`） |

### 4.2 TypeScript 规范

```typescript
// 优先使用 interface 定义对象类型
interface Note {
  id: string;
  title: string;
  content: string;
  createdAt: Date;
  updatedAt: Date;
  tags: string[];
}

// 使用 type 定义联合类型和工具类型
type NoteStatus = "draft" | "published" | "archived";
type NoteWithStatus = Note & { status: NoteStatus };
```

**关键规则：**
- 所有函数必须有明确的返回类型声明
- 禁止使用 `any`，必要时使用 `unknown` 并进行类型收窄
- 组件 Props 必须定义 interface，以 `Props` 后缀命名
- 异步操作统一使用 `async/await`
- 错误处理使用 try/catch 并提供用户友好的错误信息

### 4.3 React 组件规范

```typescript
// 组件定义标准模式
interface NoteCardProps {
  note: Note;
  onSelect: (id: string) => void;
  isActive?: boolean;
}

export function NoteCard({ note, onSelect, isActive = false }: NoteCardProps) {
  // hooks 在最顶部
  // 事件处理函数
  // 条件判断和计算
  // return JSX
}
```

**关键规则：**
- 使用函数声明式组件，不使用 `React.FC`
- 每个组件一个文件，文件名与组件名一致
- 将复杂逻辑抽取到自定义 hooks
- 组件内部按照 hooks → handlers → computed → JSX 的顺序组织
- 使用 `memo` 仅当性能分析表明需要时

### 4.4 Tauri IPC 通信规范

```rust
// Rust 端命令定义
#[tauri::command]
async fn get_notes(db: State<'_, Database>) -> Result<Vec<Note>, String> {
    db.get_all_notes()
        .map_err(|e| e.to_string())
}
```

```typescript
// 前端调用方式
import { invoke } from "@tauri-apps/api/core";

async function fetchNotes(): Promise<Note[]> {
  return invoke<Note[]>("get_notes");
}
```

**关键规则：**
- 所有 IPC 命令必须返回 `Result` 类型
- 前端 `invoke` 调用必须用 try/catch 包裹
- 大量数据传输使用 Tauri 事件系统而非命令
- 敏感操作必须在 `capabilities/` 中声明权限

### 4.5 状态管理规范

```typescript
// Zustand store 标准模式
interface NoteStore {
  notes: Note[];
  activeNoteId: string | null;
  isLoading: boolean;
  fetchNotes: () => Promise<void>;
  setActiveNote: (id: string) => void;
  createNote: (title: string) => Promise<void>;
}

export const useNoteStore = create<NoteStore>((set, get) => ({
  notes: [],
  activeNoteId: null,
  isLoading: false,
  
  fetchNotes: async () => {
    set({ isLoading: true });
    const notes = await invoke<Note[]>("get_notes");
    set({ notes, isLoading: false });
  },
  
  setActiveNote: (id) => set({ activeNoteId: id }),
  
  createNote: async (title) => {
    const note = await invoke<Note>("create_note", { title });
    set((state) => ({ notes: [...state.notes, note] }));
  },
}));
```

**关键规则：**
- 按功能域拆分 store（noteStore、settingsStore、uiStore）
- 异步操作在 store action 中执行，组件只调用 action
- 使用 selector 避免不必要的重渲染
- 持久化数据使用 Tauri 后端存储，不使用 localStorage

---

## 第五章 · 构建与发布

### 5.1 NPM Scripts

```json
{
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview",
    "tauri": "tauri",
    "tauri:dev": "tauri dev",
    "tauri:build": "tauri build",
    "lint": "eslint src --ext .ts,.tsx",
    "format": "prettier --write src",
    "check": "tsc --noEmit"
  }
}
```

### 5.2 开发流程

```
开发者日常工作流:

1. pnpm tauri:dev        → 启动开发服务器 + Tauri 窗口
2. 编写/修改代码          → Vite HMR 自动热更新前端
3. 修改 Rust 代码         → Tauri 自动重新编译后端
4. pnpm lint && pnpm check → 提交前检查
5. git commit             → 提交变更
```

### 5.3 构建与分发

```
pnpm tauri:build

输出:
  → .dmg 安装包（macOS 标准分发格式）
  → .app 应用程序包
  → 可选：通过 Apple Notarization 公证
```

### 5.4 版本管理

- 遵循 [Semantic Versioning](https://semver.org/)（语义化版本）
- `MAJOR.MINOR.PATCH`（如 `1.2.3`）
- package.json 和 tauri.conf.json 中版本号保持同步

---

## 第六章 · 核心功能模块

### 6.1 模块划分

```
┌────────────────────────────────────────────────┐
│                 NCdesktop 功能模块               │
├────────────────────────────────────────────────┤
│                                                │
│  📝 笔记核心          🗂 组织管理               │
│  ├─ 富文本编辑        ├─ 文件夹/分类            │
│  ├─ Markdown 支持     ├─ 标签系统               │
│  ├─ 代码块高亮        ├─ 搜索与过滤             │
│  └─ 实时预览          └─ 排序与归档             │
│                                                │
│  🎨 界面交互          ⚙️ 系统集成               │
│  ├─ Liquid Glass UI   ├─ 全局快捷键捕获         │
│  ├─ 侧边栏导航        ├─ 菜单栏快捷操作         │
│  ├─ 工具栏             ├─ Spotlight 搜索集成     │
│  └─ 暗色/亮色主题     └─ 文件系统读写           │
│                                                │
│  💾 数据管理          🔄 同步扩展（未来）        │
│  ├─ SQLite 本地存储   ├─ iCloud 同步            │
│  ├─ 自动保存          ├─ 导出（PDF/MD/HTML）    │
│  └─ 数据备份          └─ 分享功能               │
│                                                │
└────────────────────────────────────────────────┘
```

### 6.2 数据模型

```typescript
interface Note {
  id: string;                // UUID
  title: string;
  content: string;           // Markdown/富文本内容
  folderId: string | null;
  tags: string[];
  isPinned: boolean;
  isArchived: boolean;
  createdAt: string;         // ISO 8601
  updatedAt: string;         // ISO 8601
}

interface Folder {
  id: string;
  name: string;
  parentId: string | null;   // 支持嵌套
  icon: string;              // 图标标识
  sortOrder: number;
}

interface Tag {
  id: string;
  name: string;
  color: string;             // 标签颜色
}

interface AppSettings {
  theme: "light" | "dark" | "system";
  fontSize: number;
  fontFamily: string;
  sidebarWidth: number;
  editorWidth: number;
  autoSaveInterval: number;  // 毫秒
}
```

---

## 第七章 · 性能与安全

### 7.1 性能目标

| 指标 | 目标值 |
|------|--------|
| 冷启动时间 | < 500ms |
| 内存占用（空闲） | < 80MB |
| 打包体积 | < 15MB |
| 笔记切换延迟 | < 100ms |
| 搜索响应 | < 200ms（10,000 条笔记内） |
| 渲染帧率 | 60fps（动画/滚动） |

### 7.2 性能策略

- 虚拟列表渲染大量笔记（使用 `@tanstack/react-virtual`）
- 搜索使用 Rust 端 SQLite FTS5 全文检索
- 图片和大文件延迟加载
- React 组件按需加载（`React.lazy`）
- CSS 动画优先使用 `transform` 和 `opacity`（GPU 加速）

### 7.3 安全原则

- Tauri 权限最小化：只声明必要的 capabilities
- 用户数据本地加密存储
- 不收集用户隐私数据
- 所有外部链接在系统浏览器中打开
- IPC 命令参数验证和类型检查

---

## 第八章 · Git 与协作

### 8.1 分支策略

```
main        ← 稳定发布版本
├── dev     ← 开发主线
│   ├── feat/note-editor    ← 功能分支
│   ├── fix/sidebar-crash   ← 修复分支
│   └── chore/update-deps   ← 维护分支
```

### 8.2 Commit 消息规范

```
<type>(<scope>): <description>

type:
  feat     新功能
  fix      修复 bug
  refactor 重构
  style    样式调整
  docs     文档更新
  chore    构建/工具变更
  perf     性能优化
  test     测试

scope: 模块名（editor、sidebar、store、tauri 等）

示例:
  feat(editor): 添加 Markdown 实时预览功能
  fix(sidebar): 修复文件夹拖拽排序异常
  perf(search): 使用 FTS5 优化全文搜索速度
```

### 8.3 .gitignore 核心规则

```
node_modules/
dist/
target/
*.DS_Store
.env
*.local
src-tauri/target/
src-tauri/WixTools/
src-tauri/gen/
```

---

## 附录 A · 关键依赖参考

### 前端依赖

| 包名 | 用途 |
|------|------|
| react + react-dom | UI 框架 |
| @tauri-apps/api | Tauri 前端 API |
| zustand | 状态管理 |
| tailwindcss | 原子化 CSS |
| lucide-react | 图标库 |
| @tanstack/react-virtual | 虚拟列表 |
| react-router | 路由（如需） |

### Tauri 插件

| 插件 | 用途 |
|------|------|
| tauri-plugin-liquid-glass | macOS Liquid Glass 窗口效果 |
| tauri-plugin-sql | SQLite 数据库 |
| tauri-plugin-fs | 文件系统访问 |
| tauri-plugin-shell | 外部命令调用 |
| tauri-plugin-notification | 系统通知 |
| tauri-plugin-global-shortcut | 全局快捷键 |

---

## 附录 B · 此宪章的使用说明

本宪章是 NCdesktop 项目的权威开发指导文件。所有代码生成、架构决策、技术选型都必须以本宪章为准则。AI 助手在参与本项目开发时，应：

1. **严格遵循** 本宪章定义的技术栈和目录结构
2. **使用** 规定的命名规范和编码风格
3. **参考** 数据模型设计来生成类型定义
4. **依照** 品牌标识中的颜色值进行 UI 开发
5. **遵守** 性能目标，避免引入性能退化
6. **保持** 中文注释和文档的一致性
