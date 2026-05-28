# NCdesktop 20 步开发框架宪章

> NoteCapt Desktop — 多模态知识采集终端桌面控制中枢
> 版本：1.0 | 生效日期：2026-03-25
> 上游文档：《整体 PRD》《NCdesktop 软件 PRD》《项目开发宪章》《Liquid Glass UI 设计宪章》《Omni/Arca 软件开发宪章》《SpecKit 宪章模板》《桌面端时间轴窗口 UI 描述》

---

## 宪章导言

本宪章将 NCdesktop 的完整开发过程拆解为 **20 个严格有序的步骤（Step）**，每个步骤定义了：

- **目标**：该步骤需要达成的状态
- **输入**：开始前必须就绪的前置产出物
- **产出物**：步骤完成后的交付文件/代码
- **涉及目录**：变更的文件路径
- **验收标准**：可测量的完成判定条件
- **质量门禁**：进入下一步前必须通过的检查项

### 开发阶段总览

```
┌─────────────────────────────────────────────────────────────────────┐
│                    NCdesktop 20 步开发路线图                          │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ▸ 第一阶段：基建筑基（Step 1-4）                                    │
│    搭环境 → 创脚手架 → 建设计系统 → 铺布局骨架                       │
│                                                                     │
│  ▸ 第二阶段：数据地基（Step 5-8）                                    │
│    类型系统 → 数据库层 → 状态管理 → IPC 通信桥                       │
│                                                                     │
│  ▸ 第三阶段：核心 UI（Step 9-11）                                    │
│    侧边栏导航 → 知识库管理 → 素材预览与 Inspector                    │
│                                                                     │
│  ▸ 第四阶段：核心引擎（Step 12-14）                                  │
│    TF 卡同步引擎 → 音频处理核心 → 波形渲染引擎                       │
│                                                                     │
│  ▸ 第五阶段：灵魂功能（Step 15-17）  ★ 产品核心差异化                │
│    时空记忆轴 → 关键帧锚定 → Magic Moment 双向联动                   │
│                                                                     │
│  ▸ 第六阶段：增值功能（Step 18-19）                                  │
│    全局悬浮窗 → LLM Bridge                                          │
│                                                                     │
│  ▸ 第七阶段：收尾发布（Step 20）                                     │
│    全局搜索 + 构建打包 + 发布                                        │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### 步骤依赖关系图

```
Step 1 ──▶ Step 2 ──▶ Step 3 ──▶ Step 4
                                     │
                    ┌────────────────┤
                    ▼                ▼
                 Step 5          Step 9
                    │                │
                    ▼                ▼
                 Step 6          Step 10
                    │                │
                    ▼                ▼
                 Step 7          Step 11
                    │
                    ▼
                 Step 8
                    │
           ┌───────┼───────┐
           ▼       ▼       ▼
       Step 12  Step 13  Step 18
           │       │       │
           │       ▼       │
           │    Step 14    │
           │       │       │
           └───┬───┘       │
               ▼           │
            Step 15        │
               │           │
               ▼           │
            Step 16        │
               │           │
               ▼           │
            Step 17        │
               │           │
               ▼           ▼
            Step 19     Step 19
               │           │
               └─────┬─────┘
                     ▼
                  Step 20
```

---

## 第一阶段 · 基建筑基

---

### Step 1：开发环境初始化

| 属性 | 说明 |
|------|------|
| **目标** | 搭建完整的本地开发工具链，确保 Node.js、Rust、Tauri CLI 全部就绪 |
| **输入** | macOS 26 主机 + Homebrew 4.4.20 + Xcode CLI |
| **预计耗时** | 30 分钟 |

#### 1.1 执行清单

```bash
# 1. 安装 Node.js 版本管理器
brew install fnm

# 2. 配置 shell
echo 'eval "$(fnm env --use-on-cd --shell zsh)"' >> ~/.zshrc
source ~/.zshrc

# 3. 安装 Node.js LTS
fnm install --lts
fnm use lts-latest

# 4. 启用 corepack（内置 pnpm 支持）
corepack enable

# 5. 安装 Rust 工具链
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# 6. 安装 Tauri CLI
cargo install tauri-cli

# 7. 验证所有工具
node --version      # >= 22.x
pnpm --version      # >= 9.x
rustc --version     # >= 1.77
cargo tauri --version  # >= 2.x
```

#### 1.2 产出物

| 文件 | 说明 |
|------|------|
| 无代码产出 | 环境就绪即可 |

#### 1.3 验收标准

- [ ] `node --version` 输出 >= 22.x
- [ ] `pnpm --version` 输出 >= 9.x
- [ ] `rustc --version` 输出 >= 1.77
- [ ] `cargo tauri --version` 输出 >= 2.x
- [ ] Xcode Command Line Tools 已安装

---

### Step 2：Tauri v2 项目脚手架创建

| 属性 | 说明 |
|------|------|
| **目标** | 创建 Tauri v2 + React 19 + TypeScript + Vite 6 项目骨架 |
| **输入** | Step 1 就绪的开发环境 |
| **预计耗时** | 45 分钟 |

#### 2.1 执行清单

```bash
# 1. 在项目目录下初始化 Tauri 项目
pnpm create tauri-app NCdesktop --template react-ts

# 2. 安装核心前端依赖
pnpm add react@19 react-dom@19
pnpm add -D typescript@5 @types/react @types/react-dom

# 3. 安装 Tailwind CSS v4
pnpm add -D tailwindcss @tailwindcss/vite

# 4. 安装状态管理与工具库
pnpm add zustand lucide-react @tanstack/react-virtual react-router

# 5. 安装 Tauri 前端 API
pnpm add @tauri-apps/api @tauri-apps/plugin-shell

# 6. 安装开发工具
pnpm add -D eslint prettier eslint-config-prettier

# 7. 安装 Tauri 插件（Rust 端在 Cargo.toml 中添加）
# tauri-plugin-sql、tauri-plugin-fs、tauri-plugin-notification
# tauri-plugin-global-shortcut、tauri-plugin-liquid-glass
```

#### 2.2 产出物

| 文件 | 说明 |
|------|------|
| `package.json` | 前端依赖与脚本定义 |
| `pnpm-lock.yaml` | 锁文件 |
| `tsconfig.json` | TypeScript 严格模式配置 |
| `vite.config.ts` | Vite 6 + Tailwind CSS 构建配置 |
| `src-tauri/Cargo.toml` | Rust 依赖（含所有 Tauri 插件） |
| `src-tauri/tauri.conf.json` | Tauri 窗口与权限配置 |
| `src-tauri/capabilities/` | Tauri 权限声明 |
| `.gitignore` | Git 忽略规则 |
| `.env.example` | 环境变量模板（含 OPENAI_API_KEY 占位） |

#### 2.3 `tauri.conf.json` 核心配置

```json
{
  "productName": "NoteCapt",
  "version": "0.1.0",
  "identifier": "com.notecapt.desktop",
  "app": {
    "windows": [
      {
        "title": "NoteCapt",
        "width": 1200,
        "height": 800,
        "minWidth": 800,
        "minHeight": 600,
        "transparent": true,
        "decorations": true,
        "titleBarStyle": "Overlay"
      }
    ]
  },
  "plugins": {
    "liquid-glass": {
      "cornerRadius": 12,
      "tintColor": "#1F456E10"
    }
  }
}
```

#### 2.4 `package.json` Scripts

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

#### 2.5 验收标准

- [ ] `pnpm tauri:dev` 成功启动开发窗口
- [ ] 窗口标题显示 "NoteCapt"
- [ ] TypeScript 严格模式已启用（`"strict": true`）
- [ ] Vite HMR 热更新正常工作
- [ ] Rust 编译无 warning
- [ ] Git 仓库已初始化，`.gitignore` 包含 `node_modules/`、`target/`、`.env`

---

### Step 3：Liquid Glass 设计系统基础

| 属性 | 说明 |
|------|------|
| **目标** | 建立完整的 CSS 设计令牌系统（Design Tokens），实现 Liquid Glass 材质基础类 |
| **输入** | Step 2 的项目骨架 + 《Liquid Glass UI 设计宪章》 |
| **预计耗时** | 2 小时 |

#### 3.1 执行清单

| 序号 | 任务 | 目标文件 |
|------|------|----------|
| 3.1.1 | 创建全局 CSS 设计令牌（颜色、间距、圆角、模糊、阴影、排版、动画） | `src/styles/globals.css` |
| 3.1.2 | 创建 Liquid Glass 材质层级样式（L1-L5 五层玻璃） | `src/styles/glass.css` |
| 3.1.3 | 创建标准玻璃组件 CSS（panel、sidebar、toolbar、card、popover） | `src/styles/glass.css` |
| 3.1.4 | 创建暗色模式令牌覆盖 | `src/styles/globals.css` |
| 3.1.5 | 创建无障碍回退样式（降低透明度、减少动效） | `src/styles/glass.css` |
| 3.1.6 | 配置 Tailwind CSS 扩展（品牌色、玻璃色、自定义圆角等） | `tailwind.config.ts` |
| 3.1.7 | 设置窗口透明基础（html/body/root 透明） | `src/styles/globals.css` |

#### 3.2 设计令牌清单

必须包含的令牌类别（参照《Liquid Glass UI 设计宪章》第二章）：

| 类别 | 令牌数量 | 关键变量 |
|------|---------|---------|
| 模糊值 | 6 | `--glass-blur-xs` 到 `--glass-blur-2xl` |
| 背景色（亮/暗） | 5×2 | `--glass-bg-ultra-thin` 到 `--glass-bg-ultra-thick` |
| 边框色 | 4 | `--glass-border-subtle` 到 `--glass-border-accent-top` |
| 阴影 | 5 | `--glass-shadow-sm` 到 `--glass-shadow-inset` |
| 圆角 | 7 | `--radius-xs` 到 `--radius-full` |
| 间距 | 11 | `--space-1` 到 `--space-16`（4px 基准网格） |
| 品牌色 | 6 | `--brand-navy` 系列 + `--brand-gold` 系列 |
| 语义色 | 6 | `--color-primary` 到 `--color-info` |
| 文本色 | 5 | `--text-primary` 到 `--text-on-glass-secondary` |
| 排版 | 10+ | 字号阶梯 + 行高 + 字间距 |
| 动画 | 5 | 缓动函数 + 时长 |

#### 3.3 验收标准

- [ ] 所有设计令牌变量已定义，可通过 `var(--token-name)` 引用
- [ ] 亮色/暗色模式令牌完整，`[data-theme="dark"]` 覆盖生效
- [ ] `.glass-panel`、`.glass-sidebar`、`.glass-toolbar` 等基础材质类可用
- [ ] Tailwind 扩展配置包含 `brand.navy`、`brand.gold`、`glass.white` 等自定义色
- [ ] `@media (prefers-reduced-transparency)` 回退样式生效
- [ ] `@media (prefers-reduced-motion)` 动效禁用生效
- [ ] 4px 基准网格间距系统一致

---

### Step 4：应用窗口与三栏布局骨架

| 属性 | 说明 |
|------|------|
| **目标** | 实现 macOS 原生窗口 + 三栏响应式布局骨架（侧边栏 + 内容区 + Inspector） |
| **输入** | Step 3 的设计系统 + 《UI 描述》中的布局规格 |
| **预计耗时** | 3 小时 |

#### 4.1 执行清单

| 序号 | 任务 | 目标文件 |
|------|------|----------|
| 4.1.1 | 实现标题栏拖拽区域 + 窗口控件兼容 | `src/components/layout/TitleBar.tsx` |
| 4.1.2 | 实现三栏主布局容器（Sidebar + Content + Inspector） | `src/components/layout/AppLayout.tsx` |
| 4.1.3 | 实现侧边栏容器（Liquid Glass Sidebar 材质） | `src/components/layout/Sidebar.tsx` |
| 4.1.4 | 实现内容区容器（上下分割：预览区 + 时间轴区） | `src/components/layout/ContentArea.tsx` |
| 4.1.5 | 实现 Inspector 侧边面板容器（可折叠） | `src/components/layout/Inspector.tsx` |
| 4.1.6 | 实现面板分隔线拖拽调节宽度 | `src/hooks/useResizable.ts` |
| 4.1.7 | 实现响应式布局策略（≥1200px 三栏 → 700-899px 两栏 → <700px 单栏） | `src/components/layout/AppLayout.tsx` |
| 4.1.8 | 配置应用根组件和入口 | `src/App.tsx` + `src/main.tsx` |

#### 4.2 布局尺寸规格

| 区域 | 默认宽度 | 可调范围 | 材质层级 |
|------|---------|---------|---------|
| 侧边栏 | 220px | 160-300px（可拖拽） | L2 Thin (glass-sidebar) |
| 内容区 | flex 剩余空间 | — | 纯净背景（无玻璃） |
| Inspector | 320px | 可折叠至 0px | L2 Thin |
| 标题栏 | 100% × 52px | 固定 | L3 Regular (glass-toolbar) |

#### 4.3 验收标准

- [ ] macOS 标题栏红绿灯按钮正确显示（`titleBarStyle: "Overlay"`）
- [ ] 标题栏区域可拖拽移动窗口（`-webkit-app-region: drag`）
- [ ] 三栏布局在 1200px+ 窗口下正确渲染
- [ ] 侧边栏宽度可通过拖拽分隔线调节
- [ ] Inspector 面板可折叠/展开
- [ ] 窗口缩小到 700px 时自动切换为两栏布局
- [ ] Liquid Glass 材质效果在侧边栏和标题栏生效（半透明 + 模糊）
- [ ] 所有布局间距遵循 4px 基准网格

---

## 第二阶段 · 数据地基

---

### Step 5：TypeScript 类型系统

| 属性 | 说明 |
|------|------|
| **目标** | 定义全部核心数据模型的 TypeScript 类型，作为前端类型安全的基础 |
| **输入** | 《NCdesktop 软件 PRD》第三章完整数据模型 |
| **预计耗时** | 2 小时 |

#### 5.1 执行清单

| 序号 | 任务 | 目标文件 |
|------|------|----------|
| 5.1.1 | 定义知识库 Library 类型 | `src/types/library.ts` |
| 5.1.2 | 定义项目 Project + ProjectSource + ProjectMetadata 类型 | `src/types/project.ts` |
| 5.1.3 | 定义时间轴 Timeline + AudioTrack + Transcription + TranscriptionSegment 类型 | `src/types/timeline.ts` |
| 5.1.4 | 定义关键帧 Keyframe + 标记 Marker 类型 | `src/types/timeline.ts` |
| 5.1.5 | 定义素材 Asset + AssetType + AssetSource + AIAnalysis 类型 | `src/types/asset.ts` |
| 5.1.6 | 定义标签 Tag + 笔记 Note 类型 | `src/types/common.ts` |
| 5.1.7 | 定义导出配置 ExportConfig 类型 | `src/types/export.ts` |
| 5.1.8 | 定义应用设置 AppSettings + LLMTarget 类型 | `src/types/settings.ts` |
| 5.1.9 | 定义 TF 卡同步相关类型（TFCardManifest、SessionData） | `src/types/sync.ts` |
| 5.1.10 | 定义 LLM 相关类型（ChatMessage、LLMConfig、LLMRequestLog） | `src/types/llm.ts` |
| 5.1.11 | 创建统一导出入口 | `src/types/index.ts` |

#### 5.2 类型完整性清单

以下实体 **必须** 有对应 TypeScript interface/type 定义：

```
Library, Project, ProjectSource, ProjectMetadata,
Timeline, AudioTrack, Transcription, TranscriptionSegment,
Keyframe, Marker, Asset, AssetType, AssetSource, AIAnalysis,
Tag, Note, ExportConfig, AppSettings, LLMTarget,
ChatMessage, LLMConfig, LLMRequestLog
```

#### 5.3 验收标准

- [ ] 所有类型与《软件 PRD》第三章数据模型一一对应
- [ ] `pnpm check`（tsc --noEmit）零错误
- [ ] 无 `any` 类型出现
- [ ] 所有 interface 属性有明确类型声明
- [ ] 联合类型（AssetType、ProjectSource 等）使用字面量类型

---

### Step 6：SQLite 数据库层（Rust 端）

| 属性 | 说明 |
|------|------|
| **目标** | 实现 Rust 端 SQLite 数据库初始化、表结构创建、基础 CRUD 操作 |
| **输入** | Step 5 的类型定义 → 对应 Rust 数据模型 |
| **预计耗时** | 4 小时 |

#### 6.1 执行清单

| 序号 | 任务 | 目标文件 |
|------|------|----------|
| 6.1.1 | 配置 rusqlite/sea-orm 依赖 | `src-tauri/Cargo.toml` |
| 6.1.2 | 实现数据库初始化与迁移 | `src-tauri/src/db/mod.rs` |
| 6.1.3 | 创建表结构（libraries、projects、timelines、audio_tracks、keyframes、markers、assets、tags、notes） | `src-tauri/src/db/migrations.rs` |
| 6.1.4 | 创建 FTS5 虚拟表（用于全文搜索） | `src-tauri/src/db/migrations.rs` |
| 6.1.5 | 实现 Rust 数据模型（对应前端类型） | `src-tauri/src/models/` |
| 6.1.6 | 实现 Library CRUD | `src-tauri/src/db/library.rs` |
| 6.1.7 | 实现 Project CRUD | `src-tauri/src/db/project.rs` |
| 6.1.8 | 实现 Asset CRUD | `src-tauri/src/db/asset.rs` |
| 6.1.9 | 实现 Timeline + AudioTrack + Keyframe CRUD | `src-tauri/src/db/timeline.rs` |
| 6.1.10 | 实现 Tag / Note CRUD | `src-tauri/src/db/tag.rs` + `note.rs` |
| 6.1.11 | 实现 AppSettings 读写 | `src-tauri/src/db/settings.rs` |

#### 6.2 核心表结构概览

```sql
-- 核心表（共 9 张）
CREATE TABLE libraries (...);
CREATE TABLE projects (...);
CREATE TABLE timelines (...);
CREATE TABLE audio_tracks (...);
CREATE TABLE keyframes (...);
CREATE TABLE markers (...);
CREATE TABLE assets (...);
CREATE TABLE tags (...);
CREATE TABLE notes (...);

-- FTS5 虚拟表（全文搜索）
CREATE VIRTUAL TABLE search_index USING fts5(
  title, content, ocr_text, transcription_text,
  content='assets', content_rowid='rowid'
);
```

#### 6.3 验收标准

- [ ] `cargo test` 全部通过
- [ ] 应用启动时自动创建/迁移数据库文件（`~/.notecapt/data.db`）
- [ ] 所有 CRUD 操作返回 `Result<T, E>` 类型
- [ ] FTS5 虚拟表创建成功，支持中英文分词搜索
- [ ] 数据库文件大小在空库状态 < 100KB
- [ ] 单元测试覆盖所有 CRUD 函数

---

### Step 7：Zustand 状态管理层

| 属性 | 说明 |
|------|------|
| **目标** | 按功能域创建 Zustand store，建立前端状态管理体系 |
| **输入** | Step 5 的类型定义 |
| **预计耗时** | 3 小时 |

#### 7.1 执行清单

| 序号 | 任务 | 目标文件 |
|------|------|----------|
| 7.1.1 | 创建 Library Store（知识库列表、当前知识库） | `src/stores/libraryStore.ts` |
| 7.1.2 | 创建 Project Store（项目列表、当前项目、筛选/排序） | `src/stores/projectStore.ts` |
| 7.1.3 | 创建 Timeline Store（时间轴状态、播放控制、缩放级别） | `src/stores/timelineStore.ts` |
| 7.1.4 | 创建 Asset Store（素材列表、当前选中素材） | `src/stores/assetStore.ts` |
| 7.1.5 | 创建 UI Store（主题、侧边栏宽度、Inspector 展开状态、视图模式） | `src/stores/uiStore.ts` |
| 7.1.6 | 创建 Sync Store（TF 卡连接状态、同步进度） | `src/stores/syncStore.ts` |
| 7.1.7 | 创建 Search Store（搜索关键词、结果、过滤器） | `src/stores/searchStore.ts` |
| 7.1.8 | 创建 LLM Store（对话历史、流式内容、配置状态） | `src/stores/llmStore.ts` |
| 7.1.9 | 创建 Settings Store（应用设置读写） | `src/stores/settingsStore.ts` |

#### 7.2 Store 设计规则

```typescript
// 每个 Store 遵循统一结构
interface XXXStore {
  // 数据状态
  items: Item[];
  activeItemId: string | null;
  isLoading: boolean;
  error: string | null;
  
  // 同步 Action
  setActiveItem: (id: string) => void;
  
  // 异步 Action（内部调用 Tauri IPC）
  fetchItems: () => Promise<void>;
  createItem: (data: CreateItemData) => Promise<void>;
  updateItem: (id: string, data: Partial<Item>) => Promise<void>;
  deleteItem: (id: string) => Promise<void>;
}
```

#### 7.3 验收标准

- [ ] 9 个 Store 全部创建，类型安全（零 `any`）
- [ ] 所有异步 Action 内包含 `isLoading` 和 `error` 状态管理
- [ ] 使用 selector 避免无关组件重渲染
- [ ] 不使用 localStorage 持久化，数据通过 Tauri 后端存储
- [ ] `pnpm check` 零错误

---

### Step 8：Tauri IPC 通信桥

| 属性 | 说明 |
|------|------|
| **目标** | 打通前端 Zustand Store 与 Rust 后端 SQLite 的 IPC 命令通道 |
| **输入** | Step 6 的 Rust 数据库层 + Step 7 的 Zustand Store |
| **预计耗时** | 4 小时 |

#### 8.1 执行清单

| 序号 | 任务 | 目标文件 |
|------|------|----------|
| 8.1.1 | 注册 Tauri 命令：Library CRUD（get_libraries, create_library） | `src-tauri/src/commands/library.rs` |
| 8.1.2 | 注册 Tauri 命令：Project CRUD（get_projects, create_project, update_project, delete_project） | `src-tauri/src/commands/project.rs` |
| 8.1.3 | 注册 Tauri 命令：Asset CRUD | `src-tauri/src/commands/asset.rs` |
| 8.1.4 | 注册 Tauri 命令：Timeline + Keyframe 操作 | `src-tauri/src/commands/timeline.rs` |
| 8.1.5 | 注册 Tauri 命令：Tag / Note 操作 | `src-tauri/src/commands/tag.rs` + `note.rs` |
| 8.1.6 | 注册 Tauri 命令：Settings 读写 | `src-tauri/src/commands/settings.rs` |
| 8.1.7 | 在 Rust main.rs 中统一注册所有命令 | `src-tauri/src/main.rs` |
| 8.1.8 | 在 capabilities 中声明所需权限 | `src-tauri/capabilities/default.json` |
| 8.1.9 | 前端创建 IPC 调用封装层（统一 try/catch + 错误处理） | `src/lib/tauri-commands.ts` |
| 8.1.10 | 连接 Zustand Store Actions 到 IPC 命令 | 各 Store 文件 |

#### 8.2 IPC 命令命名规范

```
Rust 端: snake_case → get_all_projects, create_project, update_project
前端调用: invoke<ReturnType>("command_name", { param })
```

#### 8.3 验收标准

- [ ] 所有 IPC 命令返回 `Result<T, String>` 类型
- [ ] 前端 `invoke` 调用全部用 try/catch 包裹
- [ ] 前端可成功调用后端创建/读取/更新/删除 Project
- [ ] 数据从前端 → Rust → SQLite → Rust → 前端 完整闭环
- [ ] Tauri capabilities 权限声明最小化
- [ ] 错误信息对用户友好（非 Rust 原始错误信息）

---

## 第三阶段 · 核心 UI

---

### Step 9：侧边栏导航

| 属性 | 说明 |
|------|------|
| **目标** | 实现完整的 Knowledge Library 侧边栏导航树 |
| **输入** | Step 4 的布局骨架 + Step 7 的 Store + 《UI 描述》第二节 |
| **预计耗时** | 4 小时 |

#### 9.1 执行清单

| 序号 | 任务 | 目标文件 |
|------|------|----------|
| 9.1.1 | 实现侧边栏顶部品牌标识（"NCdesktop" + "KNOWLEDGE LIBRARY"） | `src/components/layout/Sidebar.tsx` |
| 9.1.2 | 实现导航项组件（图标 + 文字 + 选中态 + 金色指示条） | `src/components/layout/SidebarItem.tsx` |
| 9.1.3 | 实现搜索入口（Search） | `src/components/layout/Sidebar.tsx` |
| 9.1.4 | 实现最近导入（Recent） | `src/components/layout/Sidebar.tsx` |
| 9.1.5 | 实现项目树（Projects，可展开/折叠，含子项 Timeline） | `src/components/features/ProjectTree.tsx` |
| 9.1.6 | 实现标签树（Tags，显示使用次数） | `src/components/features/TagTree.tsx` |
| 9.1.7 | 实现收藏（Starred）入口 | `src/components/layout/Sidebar.tsx` |
| 9.1.8 | 实现底部状态栏（Settings + TF Card 连接状态） | `src/components/layout/SidebarFooter.tsx` |
| 9.1.9 | 连接侧边栏到 projectStore 和 uiStore | 各组件 |

#### 9.2 视觉规格

| 元素 | 规格 |
|------|------|
| 选中项背景 | `rgba(31, 69, 110, 0.12)` |
| 左侧金色指示条 | 2.5px 宽，`#FFC000` |
| 导航图标 | Lucide React，品牌金色 `#FFC000` |
| 字号 | 13px（Footnote 级） |
| 项间距 | 8px（--space-2） |
| 材质 | glass-sidebar（L2 Thin，blur 12px） |

#### 9.3 验收标准

- [ ] 侧边栏显示完整导航树（Search、Recent、Projects、Tags、Settings）
- [ ] 项目树可展开/折叠，选中高亮正确
- [ ] 标签显示使用次数
- [ ] 底部显示 TF 卡连接状态（Connected / Disconnected）
- [ ] 选中项有金色左侧指示条 + 背景高亮
- [ ] hover 态有微妙的背景色变化
- [ ] 键盘导航（Tab + Enter）可操作

---

### Step 10：知识库管理页面

| 属性 | 说明 |
|------|------|
| **目标** | 实现知识库首页的项目列表展示（卡片视图 + 列表视图） |
| **输入** | Step 8 的 IPC 通道 + Step 9 的侧边栏 + 《软件 PRD》F8 |
| **预计耗时** | 4 小时 |

#### 10.1 执行清单

| 序号 | 任务 | 目标文件 |
|------|------|----------|
| 10.1.1 | 实现项目卡片组件（封面缩略图 + 名称 + 日期 + 素材统计 + 标签） | `src/components/features/ProjectCard.tsx` |
| 10.1.2 | 实现项目列表行组件 | `src/components/features/ProjectListItem.tsx` |
| 10.1.3 | 实现卡片/列表视图切换 | `src/components/features/ProjectListView.tsx` |
| 10.1.4 | 实现虚拟列表渲染（@tanstack/react-virtual） | `src/components/features/ProjectListView.tsx` |
| 10.1.5 | 实现工具栏（新建项目 + 搜索框 + 排序 + 视图切换） | `src/components/layout/Toolbar.tsx` |
| 10.1.6 | 实现空状态页面（欢迎页 + 引导） | `src/components/features/EmptyState.tsx` |
| 10.1.7 | 连接到 projectStore 数据 | 各组件 |

#### 10.2 验收标准

- [ ] 卡片视图正确显示项目信息（封面、名称、日期、统计、标签）
- [ ] 列表视图正确显示（图标、名称、时长、素材数、日期）
- [ ] 视图切换动效平滑（300ms，ease-out-expo）
- [ ] 虚拟列表在 1000+ 项目时仍 60fps 滚动
- [ ] 空状态页在无项目时显示引导信息
- [ ] 工具栏搜索框可实时过滤项目
- [ ] 新建项目按钮功能正常

---

### Step 11：素材预览与 Inspector 面板

| 属性 | 说明 |
|------|------|
| **目标** | 实现素材大图预览区域和右侧 Inspector 属性面板 |
| **输入** | Step 4 的布局骨架 + 《UI 描述》第三、四节 |
| **预计耗时** | 4 小时 |

#### 11.1 执行清单

| 序号 | 任务 | 目标文件 |
|------|------|----------|
| 11.1.1 | 实现素材预览面板（Asset Preview Panel：大图/文本显示） | `src/components/features/AssetPreview.tsx` |
| 11.1.2 | 实现照片查看器（缩放、平移） | `src/components/features/PhotoViewer.tsx` |
| 11.1.3 | 实现扫描文本查看器 | `src/components/features/ScanTextViewer.tsx` |
| 11.1.4 | 实现 Inspector 素材详情区（缩略图 + 元数据） | `src/components/layout/InspectorDetails.tsx` |
| 11.1.5 | 实现 Inspector AI 分析区（Summary + OCR Text） | `src/components/layout/InspectorAI.tsx` |
| 11.1.6 | 实现 Inspector 建议标签区（Suggested Tags） | `src/components/layout/InspectorTags.tsx` |
| 11.1.7 | 实现标签编辑（添加/删除标签） | `src/components/common/TagEditor.tsx` |
| 11.1.8 | 连接到 assetStore | 各组件 |

#### 11.2 验收标准

- [ ] 照片素材可大图预览，支持缩放手势
- [ ] 扫描文本素材正确渲染文本内容
- [ ] Inspector 显示完整元数据（采集时间、来源、AI 分析）
- [ ] 建议标签可点击接受/拒绝
- [ ] 用户可手动添加/删除标签
- [ ] Inspector 面板可折叠/展开
- [ ] 选中不同素材时 Inspector 平滑切换

---

## 第四阶段 · 核心引擎

---

### Step 12：TF 卡同步引擎

| 属性 | 说明 |
|------|------|
| **目标** | 实现 TF 卡检测、会话解析、文件导入、数据库写入的完整同步流程 |
| **输入** | Step 6 的数据库层 + 《软件 PRD》F7 + TF 卡目录结构规范 |
| **预计耗时** | 6 小时 |

#### 12.1 执行清单

| 序号 | 任务 | 目标文件 |
|------|------|----------|
| 12.1.1 | 实现 TF 卡/可移动存储卷检测（监听 macOS 卷挂载事件） | `src-tauri/src/sync/detector.rs` |
| 12.1.2 | 实现 `.arca` 目录识别与 manifest.json 解析 | `src-tauri/src/sync/manifest.rs` |
| 12.1.3 | 实现会话（Session）解析：遍历 sessions/ 目录 | `src-tauri/src/sync/session_parser.rs` |
| 12.1.4 | 实现文件复制（音频/照片/扫描文本 → 本地存储） | `src-tauri/src/sync/file_copier.rs` |
| 12.1.5 | 实现元数据解析（.meta.json → AI 分析结果 + 时间戳） | `src-tauri/src/sync/meta_parser.rs` |
| 12.1.6 | 实现时间轴自动构建（基于时间戳将关键帧锚定到音频时间轴） | `src-tauri/src/sync/timeline_builder.rs` |
| 12.1.7 | 实现同步状态管理（sync_state.json 读写，增量同步） | `src-tauri/src/sync/state.rs` |
| 12.1.8 | 实现导入进度 Tauri Event 推送 | `src-tauri/src/sync/progress.rs` |
| 12.1.9 | 注册 IPC 命令：scan_tf_card、import_sessions、get_sync_status | `src-tauri/src/commands/sync.rs` |
| 12.1.10 | 前端实现导入预览对话框 | `src/components/features/ImportPreview.tsx` |
| 12.1.11 | 前端实现导入进度指示器 | `src/components/features/ImportProgress.tsx` |
| 12.1.12 | 连接到 syncStore | 各组件 |

#### 12.2 验收标准

- [ ] 插入 TF 卡后 < 5 秒检测到（支持模拟数据测试）
- [ ] manifest.json 正确解析，显示会话预览
- [ ] 文件复制速度 > 50MB/s（Rust 并行复制）
- [ ] meta.json 中的 AI 标签和时间戳正确导入
- [ ] 时间轴自动构建：关键帧按时间戳锚定到音频位置
- [ ] 增量同步：已导入的会话不重复导入
- [ ] 导入进度实时推送到前端

---

### Step 13：音频处理核心（Rust 端）

| 属性 | 说明 |
|------|------|
| **目标** | 实现 Rust 端音频解码、波形数据生成、时间戳服务 |
| **输入** | Step 6 的数据库层 + 《软件 PRD》F5 音频规格 |
| **预计耗时** | 5 小时 |

#### 13.1 执行清单

| 序号 | 任务 | 目标文件 |
|------|------|----------|
| 13.1.1 | 添加音频解码 Rust 依赖（symphonia / rodio） | `src-tauri/Cargo.toml` |
| 13.1.2 | 实现音频文件解码（m4a、wav、mp3、aac → PCM 数据） | `src-tauri/src/audio/decoder.rs` |
| 13.1.3 | 实现波形数据生成算法（PCM → 缩减采样 → 波形峰值数组） | `src-tauri/src/audio/waveform.rs` |
| 13.1.4 | 实现波形数据缓存（预渲染存储到文件） | `src-tauri/src/audio/waveform.rs` |
| 13.1.5 | 实现时间戳服务（毫秒精度的时间点计算） | `src-tauri/src/audio/timestamp.rs` |
| 13.1.6 | 实现音频元信息提取（时长、采样率、声道数） | `src-tauri/src/audio/metadata.rs` |
| 13.1.7 | 注册 IPC 命令：get_waveform_data、get_audio_metadata | `src-tauri/src/commands/audio.rs` |
| 13.1.8 | 实现波形数据异步生成 + Event 进度推送 | `src-tauri/src/audio/waveform.rs` |

#### 13.2 验收标准

- [ ] 支持 m4a、wav、mp3、aac 四种格式解码
- [ ] 90 分钟音频波形生成 < 2 分钟（后台异步）
- [ ] 波形数据文件大小合理（90min 音频 < 5MB）
- [ ] 时间戳精度 < 10ms
- [ ] `cargo test` 音频模块全部通过
- [ ] 波形数据缓存有效，二次访问直接读取

---

### Step 14：前端波形渲染引擎

| 属性 | 说明 |
|------|------|
| **目标** | 实现基于 Canvas 的音频波形渲染、播放控制、时间轴交互 |
| **输入** | Step 13 的波形数据 + 《软件 PRD》F5.5 波形规格 |
| **预计耗时** | 6 小时 |

#### 14.1 执行清单

| 序号 | 任务 | 目标文件 |
|------|------|----------|
| 14.1.1 | 实现 Canvas 波形渲染器（双色：海军蓝未播放 + 金色已播放） | `src/components/features/timeline/WaveformRenderer.tsx` |
| 14.1.2 | 实现播放头组件（2px 金色竖线 + 顶部圆形指示器） | `src/components/features/timeline/Playhead.tsx` |
| 14.1.3 | 实现时间刻度尺（自适应密度） | `src/components/features/timeline/TimeRuler.tsx` |
| 14.1.4 | 实现播放控制栏（⏮ ◀◀ ▶/⏸ ▶▶ ⏭ + 速度 + 音量） | `src/components/features/timeline/PlaybackControls.tsx` |
| 14.1.5 | 实现音频播放 Hook（Web Audio API 封装） | `src/hooks/useAudioPlayer.ts` |
| 14.1.6 | 实现波形水平拖拽滚动（惯性滑动） | `src/hooks/useTimelineDrag.ts` |
| 14.1.7 | 实现波形缩放（滚轮/双指捏合，1秒/px ↔ 60秒/px） | `src/hooks/useTimelineZoom.ts` |
| 14.1.8 | 实现时间范围选区（拖拽高亮 + 淡蓝半透明覆盖） | `src/components/features/timeline/SelectionOverlay.tsx` |
| 14.1.9 | 实现键盘快捷键（Space 播放/暂停、← → 快退快进、Home/End） | `src/hooks/useTimelineShortcuts.ts` |
| 14.1.10 | 连接到 timelineStore | 各组件 |

#### 14.2 波形视觉规格（参照《软件 PRD》F5.5）

| 属性 | 值 |
|------|------|
| 波形高度 | 64px（可折叠到 32px） |
| 未播放波形色 | `#1F456E` alpha 0.6 |
| 已播放波形色 | `#FFC000` 渐变填充（`#FFD54F → #FFC000`） |
| 播放头 | 2px 宽，`#FFC000`，顶部 8px 圆形 |
| 选区高亮 | `rgba(31, 69, 110, 0.10)` |
| 时间刻度 | 底部灰色文字，12px |
| 背景 | 深海军蓝（透明度低） |

#### 14.3 验收标准

- [ ] 波形渲染 60fps 无卡顿
- [ ] 播放时播放头平滑移动
- [ ] 点击波形任意位置，播放头精准跳转
- [ ] 水平拖拽带惯性滑动效果
- [ ] 缩放流畅（1秒/px ↔ 60秒/px 之间连续缩放）
- [ ] 所有快捷键生效（Space、方向键、Home/End）
- [ ] 播放速度切换正常（0.5x - 2.0x）
- [ ] 音量控制正常

---

## 第五阶段 · 灵魂功能 ★

> 这是 NCdesktop 的**核心产品差异化**所在，投入最多设计和测试资源。

---

### Step 15：时空记忆轴 — 关键帧轨道

| 属性 | 说明 |
|------|------|
| **目标** | 在波形上方实现关键帧缩略图轨道，将素材按时间戳精准悬挂在时间轴上 |
| **输入** | Step 14 的波形渲染器 + 《UI 描述》第三节 3.4 |
| **预计耗时** | 5 小时 |

#### 15.1 执行清单

| 序号 | 任务 | 目标文件 |
|------|------|----------|
| 15.1.1 | 实现关键帧缩略图组件（48×48px 默认，hover 放大到 120×120px） | `src/components/features/timeline/KeyframeThumb.tsx` |
| 15.1.2 | 实现关键帧轨道容器（与波形 X 轴对齐） | `src/components/features/timeline/KeyframeTrack.tsx` |
| 15.1.3 | 实现关键帧连接线（缩略图底部到波形对应位置的虚线） | `src/components/features/timeline/KeyframeConnector.tsx` |
| 15.1.4 | 实现关键帧时间分组（按时间点聚合，如 09:05、09:14、09:32） | `src/components/features/timeline/KeyframeGroup.tsx` |
| 15.1.5 | 实现关键帧拖拽调整锚定时间（水平拖拽） | `src/hooks/useKeyframeDrag.ts` |
| 15.1.6 | 实现关键帧右键菜单（查看详情 / 编辑备注 / 取消锚定 / 删除） | `src/components/features/timeline/KeyframeContextMenu.tsx` |
| 15.1.7 | 实现手动添加关键帧（从素材列表拖拽到时间轴） | `src/hooks/useKeyframeDrop.ts` |
| 15.1.8 | 实现时间轴整体组合组件 | `src/components/features/timeline/TimelineView.tsx` |

#### 15.2 验收标准

- [ ] 关键帧按时间戳精准对齐到波形 X 轴位置
- [ ] 缩略图 hover 平滑放大到 120×120px（duration-fast）
- [ ] 关键帧可拖拽调整锚定时间
- [ ] 同一时间段的素材水平排列
- [ ] 右键菜单功能完整
- [ ] 关键帧与波形同步缩放
- [ ] 手动拖拽素材到时间轴可创建新关键帧

---

### Step 16：AI 转录面板

| 属性 | 说明 |
|------|------|
| **目标** | 实现与时间轴联动的 AI 转录文本面板 |
| **输入** | Step 14 的波形播放器 + 《软件 PRD》F5.6 |
| **预计耗时** | 4 小时 |

#### 16.1 执行清单

| 序号 | 任务 | 目标文件 |
|------|------|----------|
| 16.1.1 | 实现转录面板容器 | `src/components/features/timeline/TranscriptionPanel.tsx` |
| 16.1.2 | 实现转录句子组件（时间戳 + 文本 + 说话人标识） | `src/components/features/timeline/TranscriptionSegment.tsx` |
| 16.1.3 | 实现实时高亮（播放时当前句子品牌金色下划线） | `src/hooks/useTranscriptionSync.ts` |
| 16.1.4 | 实现点击跳转（点击转录句子 → 播放头跳转） | `src/components/features/timeline/TranscriptionSegment.tsx` |
| 16.1.5 | 实现关键词搜索（搜索匹配处标黄 + 逐个跳转） | `src/components/features/timeline/TranscriptionSearch.tsx` |
| 16.1.6 | 实现手动修正（双击文本可编辑修正转录错误） | `src/components/features/timeline/TranscriptionSegment.tsx` |
| 16.1.7 | 实现转录文本导出（TXT / SRT / Markdown） | `src/lib/export/transcription-export.ts` |
| 16.1.8 | 连接到 timelineStore 播放状态 | 各组件 |

#### 16.2 验收标准

- [ ] 播放时当前句子高亮，自动滚动跟随
- [ ] 点击任意句子，播放头精准跳转
- [ ] 搜索关键词正确标黄匹配处
- [ ] 双击可编辑修正文本
- [ ] 导出 SRT 格式时间戳正确

---

### Step 17：Magic Moment 双向联动

| 属性 | 说明 |
|------|------|
| **目标** | 实现"由图寻音"和"随音现图"的核心创新交互 |
| **输入** | Step 15 的关键帧轨道 + Step 14 的播放器 + 《软件 PRD》F5.3 |
| **预计耗时** | 5 小时 |

#### 17.1 执行清单

| 序号 | 任务 | 目标文件 |
|------|------|----------|
| 17.1.1 | 实现"由图寻音"逻辑：点击关键帧 → 播放头滑动到时间点 → Pre-roll 倒退 5 秒 → 自动播放 | `src/hooks/useMagicMoment.ts` |
| 17.1.2 | 实现"由图寻音"动效：播放头平滑动画滑动（duration-normal, ease-out-expo） | `src/hooks/useMagicMoment.ts` |
| 17.1.3 | 实现"由图寻音"素材预览联动：上方内容区自动切换到该素材大图 | `src/hooks/useMagicMoment.ts` |
| 17.1.4 | 实现"随音现图"逻辑：播放到关键帧锚定时间 → 缩略图放大高亮 | `src/hooks/useMagicMoment.ts` |
| 17.1.5 | 实现"随音现图"动效：缩略图 48px → 80px + Liquid Glass 边框高光脉冲 | `src/components/features/timeline/KeyframeThumb.tsx` |
| 17.1.6 | 实现"随音现图"内容区联动：自动切换到该素材预览 | `src/hooks/useMagicMoment.ts` |
| 17.1.7 | 实现 Magic Moment 指示器（播放头顶部圆形金色标记） | `src/components/features/timeline/Playhead.tsx` |
| 17.1.8 | 实现 Pre-roll 秒数可配置（默认 5 秒，设置中可调） | `src/stores/settingsStore.ts` |

#### 17.2 验收标准

- [ ] 由图寻音：点击照片到音频开始播放 < 500ms
- [ ] 由图寻音：播放头滑动动效流畅自然
- [ ] 随音现图：经过关键帧时缩略图自动高亮放大
- [ ] 随音现图：内容区自动切换到对应素材
- [ ] Magic Moment 金色指示器在当前关键帧位置显示
- [ ] Pre-roll 倒退秒数可在设置中配置
- [ ] 音画同步精度 ≤ ±100ms
- [ ] 所有动效符合 Liquid Glass 动效规范（缓动函数、时长）
- [ ] 极端情况：3 小时录音 + 200 张图片仍可流畅操作

---

## 第六阶段 · 增值功能

---

### Step 18：全局悬浮窗 (Global Dropzone)

| 属性 | 说明 |
|------|------|
| **目标** | 实现常驻桌面的半透明悬浮窗口，支持拖拽入文件自动 AI 分类 |
| **输入** | Step 8 的 IPC 通道 + 《软件 PRD》F4 |
| **预计耗时** | 6 小时 |

#### 18.1 执行清单

| 序号 | 任务 | 目标文件 |
|------|------|----------|
| 18.1.1 | 创建悬浮窗 Tauri 窗口（系统浮动面板级别，始终在最上方） | `src-tauri/src/commands/dropzone.rs` |
| 18.1.2 | 实现悬浮窗状态机（Hidden → Idle → Attract → Processing → Complete） | `src/stores/dropzoneStore.ts` |
| 18.1.3 | 实现待机状态 UI（NoteCapt Logo，Liquid Glass L3 材质） | `src/components/features/dropzone/DropzoneIdle.tsx` |
| 18.1.4 | 实现吸引状态（检测到拖拽时放大 1.3x + 品牌金色边缘辉光） | `src/components/features/dropzone/DropzoneAttract.tsx` |
| 18.1.5 | 实现文件拖放接收（支持 PDF/图片/音频/Markdown/文件夹/链接/文本） | `src/components/features/dropzone/DropHandler.tsx` |
| 18.1.6 | 实现处理状态（AI 分类进度动画） | `src/components/features/dropzone/DropzoneProcessing.tsx` |
| 18.1.7 | 实现完成状态（✓ 动效，2 秒后回到 Idle） | `src/components/features/dropzone/DropzoneComplete.tsx` |
| 18.1.8 | 实现展开状态（点击显示最近 5 条导入记录） | `src/components/features/dropzone/DropzoneExpanded.tsx` |
| 18.1.9 | 实现右键菜单（打开主窗口 / 设置 / 暂停自动分类 / 隐藏） | `src/components/features/dropzone/DropzoneMenu.tsx` |
| 18.1.10 | 实现悬浮窗拖拽移动（可在桌面任意位置放置） | `src/hooks/useDropzoneDrag.ts` |
| 18.1.11 | 实现全局快捷键 `⌘⇧D` 显示/隐藏 | `src-tauri/src/commands/shortcuts.rs` |
| 18.1.12 | 实现 AI 自动分类逻辑（文件类型识别 → 内容提取 → 主题分析 → 匹配/创建项目 → 打标） | `src-tauri/src/commands/ai_classify.rs` |

#### 18.2 验收标准

- [ ] 悬浮窗始终在所有窗口之上
- [ ] 拖拽文件靠近时自动放大 + 金色辉光
- [ ] 支持 PDF、图片、音频、Markdown、文件夹拖入
- [ ] AI 分类完成后文件正确归入对应项目
- [ ] 单击展开最近导入记录
- [ ] 双击打开主窗口
- [ ] `⌘⇧D` 全局快捷键正常工作
- [ ] 多文件批量拖入显示进度

---

### Step 19：LLM Bridge（OpenAI 大模型集成）

| 属性 | 说明 |
|------|------|
| **目标** | 实现结构化 Markdown 导出 + OpenAI API 直连（Rust Proxy 架构） |
| **输入** | Step 8 的 IPC 通道 + 《软件 PRD》F6 + 《Omni/Arca 宪章》5.8 |
| **预计耗时** | 6 小时 |

#### 19.1 执行清单

| 序号 | 任务 | 目标文件 |
|------|------|----------|
| 19.1.1 | 配置 async-openai Rust 依赖 | `src-tauri/Cargo.toml` |
| 19.1.2 | 实现 OpenAI Client 初始化（从环境变量读取 API Key 和 Base URL） | `src-tauri/src/llm/client.rs` |
| 19.1.3 | 实现结构化 Markdown 组装器（时间轴数据 → Markdown 导出格式） | `src-tauri/src/commands/export.rs` |
| 19.1.4 | 实现 Chat Completions 封装（同步 + Streaming） | `src-tauri/src/llm/chat.rs` |
| 19.1.5 | 实现 Streaming SSE → Tauri Event 推送（打字机效果） | `src-tauri/src/llm/chat.rs` |
| 19.1.6 | 实现智能摘要命令（llm_summarize） | `src-tauri/src/commands/llm.rs` |
| 19.1.7 | 实现 AI 分类命令（llm_classify） | `src-tauri/src/commands/llm.rs` |
| 19.1.8 | 实现 Prompt 模板管理（集中化、版本化） | `src-tauri/src/llm/prompts.rs` |
| 19.1.9 | 实现错误处理与重试（401/429/500 处理 + 指数退避） | `src-tauri/src/llm/retry.rs` |
| 19.1.10 | 前端实现导出预览面板（Markdown 实时渲染 + 内容勾选） | `src/components/features/bridge/ExportPanel.tsx` |
| 19.1.11 | 前端实现 LLM 目标选择器（NotebookLM / ChatGPT / Claude / 剪贴板） | `src/components/features/bridge/TargetSelector.tsx` |
| 19.1.12 | 前端实现 Streaming 消费 Hook | `src/lib/ai/useLLMStream.ts` |
| 19.1.13 | 前端实现 LLM 设置表单（API Key 配置引导） | `src/components/features/bridge/LLMSettingsForm.tsx` |
| 19.1.14 | 实现离线降级策略（无网络时仅支持本地 Markdown 导出） | 全链路 |

#### 19.2 安全红线

- API Key 仅存在于 Rust 进程内存，从 `OPENAI_API_KEY` 环境变量读取
- 前端**永远不直接**调用 OpenAI API
- 发送至 OpenAI 的数据**不包含原始媒体文件**（仅文本/摘要）
- 日志中 API Key 最多显示前 8 位 `sk-xxxxx...`
- `.env` 已加入 `.gitignore`

#### 19.3 验收标准

- [ ] API Key 未配置时，LLM 功能优雅灰置，不影响核心功能
- [ ] 结构化 Markdown 导出格式正确（含时间线、转录、OCR、标签）
- [ ] Streaming 打字机效果流畅无闪烁
- [ ] 导出预览面板可勾选/取消内容类型
- [ ] NotebookLM 目标：导出 .md 文件 + 自动打开浏览器
- [ ] 剪贴板目标：复制到剪贴板 + Toast 提示
- [ ] 网络断开时自动降级为离线导出模式
- [ ] API 错误（401/429/500）有用户友好提示
- [ ] 指数退避重试策略正确（1s/2s/4s，最多 3 次）

---

## 第七阶段 · 收尾发布

---

### Step 20：全局搜索 + 构建打包 + 发布

| 属性 | 说明 |
|------|------|
| **目标** | 实现 FTS5 全文搜索，完成应用优化、构建打包、发布准备 |
| **输入** | Step 1-19 全部完成 |
| **预计耗时** | 8 小时 |

#### 20.1 全局搜索

| 序号 | 任务 | 目标文件 |
|------|------|----------|
| 20.1.1 | 实现搜索 IPC 命令（基于 SQLite FTS5） | `src-tauri/src/commands/search.rs` |
| 20.1.2 | 搜索范围：项目名称、AI 标签、音频转录、OCR 文字、扫描笔文本、笔记、文件名 | `src-tauri/src/db/search.rs` |
| 20.1.3 | 实现全局搜索面板 UI（搜索框 + 结果列表 + 高亮关键词） | `src/components/features/SearchPanel.tsx` |
| 20.1.4 | 实现搜索结果点击跳转（跳转到对应项目/时间轴位置） | `src/components/features/SearchResultItem.tsx` |
| 20.1.5 | 实现全局搜索快捷键 `⌘K` | `src/hooks/useGlobalShortcuts.ts` |

#### 20.2 应用优化

| 序号 | 任务 | 说明 |
|------|------|------|
| 20.2.1 | React 组件按需加载 | `React.lazy` + `Suspense` |
| 20.2.2 | 虚拟列表确认（大列表场景） | `@tanstack/react-virtual` |
| 20.2.3 | 图片/大文件延迟加载 | `IntersectionObserver` |
| 20.2.4 | CSS 动画性能确认 | 仅使用 `transform` + `opacity` |
| 20.2.5 | `will-change` 标记频繁动画元素 | 时间轴组件 |
| 20.2.6 | `content-visibility: auto` 滚动优化 | 长列表区域 |

#### 20.3 设置面板

| 序号 | 任务 | 目标文件 |
|------|------|----------|
| 20.3.1 | 实现设置 Sheet 面板 | `src/components/features/SettingsPanel.tsx` |
| 20.3.2 | 外观设置（主题切换、侧边栏宽度） | 同上 |
| 20.3.3 | TF 卡设置（自动导入、导入后删除原文件） | 同上 |
| 20.3.4 | 悬浮窗设置（启用/禁用、位置、大小、自动分类） | 同上 |
| 20.3.5 | 音频设置（默认播放速度、Pre-roll 秒数、波形颜色） | 同上 |
| 20.3.6 | AI/LLM 设置（API Key 配置、模型选择、转录语言） | 同上 |
| 20.3.7 | 隐私设置（数据存储路径、分析开关） | 同上 |

#### 20.4 构建打包

```bash
# 1. 代码检查
pnpm lint && pnpm check

# 2. 构建生产包
pnpm tauri:build

# 3. 产出物
#    → NCdesktop.dmg (macOS 安装包)
#    → NoteCapt.app (应用程序包)

# 4. 可选：Apple Notarization 公证
```

#### 20.5 最终验收标准

**性能指标：**

| 指标 | 目标值 | 测试方法 |
|------|--------|---------|
| 冷启动时间 | < 500ms | 计时器测量 |
| 内存占用（空闲） | < 80MB | Activity Monitor |
| 打包体积 | < 15MB | 文件大小 |
| 搜索响应 | < 200ms | 控制台计时 |
| 动画帧率 | 60fps | Performance Monitor |
| 音画同步精度 | ≤ ±100ms | 手动验证 |

**功能清单：**

- [ ] TF 卡插入 → 检测 → 导入 → 时间轴浏览 → 完整闭环
- [ ] 由图寻音 / 随音现图 双向联动正常
- [ ] 全局悬浮窗拖拽入文件 → AI 分类归档
- [ ] LLM 导出 → 结构化 Markdown 正确
- [ ] 全局搜索 → 跨项目定位
- [ ] 亮色/暗色/跟随系统 三种主题正常
- [ ] 键盘快捷键完整可用
- [ ] 无障碍：VoiceOver 可操作、焦点指示器可见
- [ ] 降低透明度设置下回退方案生效
- [ ] 减少动效设置下动画禁用

**安全清单：**

- [ ] 默认离线：核心功能无需网络
- [ ] API Key 不在前端代码中出现
- [ ] .env 不在 Git 仓库中
- [ ] 所有外部链接在系统浏览器打开
- [ ] Tauri capabilities 权限最小化

---

## 附录 A · 步骤-文件映射总表

| Step | 阶段 | 核心产出文件 |
|------|------|-------------|
| 1 | 基建 | （环境就绪） |
| 2 | 基建 | `package.json` `Cargo.toml` `tauri.conf.json` |
| 3 | 基建 | `src/styles/globals.css` `src/styles/glass.css` `tailwind.config.ts` |
| 4 | 基建 | `src/components/layout/*` `src/App.tsx` |
| 5 | 数据 | `src/types/*.ts` |
| 6 | 数据 | `src-tauri/src/db/*` `src-tauri/src/models/*` |
| 7 | 数据 | `src/stores/*.ts` |
| 8 | 数据 | `src-tauri/src/commands/*` `src/lib/tauri-commands.ts` |
| 9 | UI | `src/components/layout/Sidebar*.tsx` `src/components/features/ProjectTree.tsx` |
| 10 | UI | `src/components/features/ProjectCard.tsx` `ProjectListView.tsx` `Toolbar.tsx` |
| 11 | UI | `src/components/features/AssetPreview.tsx` `src/components/layout/Inspector*.tsx` |
| 12 | 引擎 | `src-tauri/src/sync/*` `src/components/features/Import*.tsx` |
| 13 | 引擎 | `src-tauri/src/audio/*` |
| 14 | 引擎 | `src/components/features/timeline/Waveform*.tsx` `PlaybackControls.tsx` |
| 15 | 灵魂★ | `src/components/features/timeline/Keyframe*.tsx` `TimelineView.tsx` |
| 16 | 灵魂★ | `src/components/features/timeline/Transcription*.tsx` |
| 17 | 灵魂★ | `src/hooks/useMagicMoment.ts` |
| 18 | 增值 | `src/components/features/dropzone/*` |
| 19 | 增值 | `src-tauri/src/llm/*` `src/components/features/bridge/*` |
| 20 | 发布 | `src/components/features/SearchPanel.tsx` `SettingsPanel.tsx` |

---

## 附录 B · 版本阶段与步骤对应

| 版本 | 阶段 | 包含步骤 | 交付功能 |
|------|------|---------|---------|
| **v0.1 MVP** | Phase 1 | Step 1-17 | TF 卡导入 → 时间轴浏览 → Magic Moment → Markdown 导出 |
| **v0.5 完善** | Phase 2 | Step 18-19 | 全局悬浮窗 + AI 分类 + LLM API 直连 |
| **v1.0 正式发布** | Phase 3 | Step 20 | 全局搜索 + 设置面板 + 性能优化 + 打包发布 |

---

## 附录 C · 关联宪章索引

| 宪章文件 | 对应步骤 | 关键引用内容 |
|---------|---------|-------------|
| 项目开发宪章 | 全部 | 技术栈、目录结构、编码规范、构建流程 |
| Liquid Glass UI 设计宪章 | Step 3, 4, 9-11, 15, 17, 18 | 材质系统、颜色、排版、动效、组件规范 |
| 软件 PRD | Step 5-20 | 数据模型、功能规格、交互规格、性能指标 |
| Omni/Arca 软件开发宪章 | Step 6, 12, 19 | 六阶段流程、隐私红线、OpenAI 集成规范 |
| 桌面端 UI 描述 | Step 9, 11, 14, 15 | 界面布局、颜色使用、组件细节 |
| SpecKit 宪章模板 | 全部 | 开发方法论框架 |

---

## 附录 D · 此宪章的使用说明

1. **严格按步骤顺序执行**：每个 Step 的"输入"栏标明了前置依赖，不可跳步
2. **每步完成后检查验收标准**：全部打 ✅ 方可进入下一步
3. **每步开始前回顾对应宪章**：参照附录 C 的关联宪章索引
4. **保持中文注释和文档一致性**
5. **遇到不确定的设计决策**：优先参考《软件 PRD》，其次参考《Liquid Glass UI 设计宪章》
6. **性能始终是红线**：每完成一个阶段，回归测试性能指标

---

> **宪章版本记录**
>
> | 版本 | 日期 | 变更说明 | 作者 |
> |------|------|----------|------|
> | 1.0 | 2026-03-25 | 初始版本：20 步开发框架宪章创建 | 钟嘉澄 |
