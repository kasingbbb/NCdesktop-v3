# NCdesktop 开发过程文档 — Step 1-4（基建筑基阶段）

> 记录人：AI Agent（Claude）
> 执行日期：2026-03-25
> **最后更新：2026-03-26**（见文末「迭代更新记录」）
> 适配宪章：《（cursor）NCdesktop 20 步开发框架宪章》
> 状态：✅ Step 1-4 已完成；**后续 UI/数据迭代已合入主线**（见迭代记录）

---

## 执行摘要

Step 1-4 为 NCdesktop 项目的「基建筑基」阶段，在本次开发中全部串行完成。所有产出物保存在 `项目启动/NCdesktop/` 目录下。

| Step | 名称 | 状态 | 耗时估算 |
|------|------|------|---------|
| 1 | 开发环境初始化 | ✅ 完成 | ~5 分钟 |
| 2 | Tauri v2 项目脚手架创建 | ✅ 完成 | ~8 分钟 |
| 3 | Liquid Glass 设计系统基础 | ✅ 完成 | ~10 分钟 |
| 4 | 应用窗口与三栏布局骨架 | ✅ 完成 | ~15 分钟 |

---

## Step 1：开发环境初始化

### 执行内容

1. **环境检测**：Homebrew 4.4.20 ✅、Xcode CLI ✅，其余工具均未安装
2. **安装 fnm**：通过 `brew install fnm` 安装 Node 版本管理器 v1.39.0
3. **安装 Node.js**：通过 fnm 安装 Node.js v24.14.1 LTS
4. **启用 pnpm**：通过 corepack enable 启用 pnpm 10.33.0
5. **安装 Rust**：通过 rustup 安装 Rust 1.94.0 (stable-aarch64-apple-darwin)
6. **安装 Tauri CLI**：通过 `cargo install tauri-cli` 安装 v2.10.1
7. **持久化配置**：将 fnm 配置写入 `~/.zshrc`

### 验收结果

| 检查项 | 结果 |
|--------|------|
| `node --version` >= 22.x | ✅ v24.14.1 |
| `pnpm --version` >= 9.x | ✅ v10.33.0 |
| `rustc --version` >= 1.77 | ✅ v1.94.0 |
| `cargo tauri --version` >= 2.x | ✅ v2.10.1 |
| Xcode CLI 已安装 | ✅ |

### 注意事项

- Rust 安装耗时较长（编译 tauri-cli 约 3 分钟）
- fnm 配置需要写入 `~/.zshrc` 才能在新 shell 中持久化

---

## Step 2：Tauri v2 项目脚手架创建

### 执行内容

1. **创建 Vite 项目**：`pnpm create vite NCdesktop --template react-ts`
2. **初始化 Tauri**：安装 `@tauri-apps/cli` 并运行 `pnpm tauri init`
3. **安装核心依赖**：
   - 前端运行时：`@tauri-apps/api`, `zustand`, `lucide-react`, `@tanstack/react-virtual`, `react-router`
   - Tauri 插件前端：`@tauri-apps/plugin-shell`
   - 构建工具：`tailwindcss`, `@tailwindcss/vite`
4. **配置 tauri.conf.json**：
   - `productName`: "NoteCapt"
   - `identifier`: "com.notecapt.desktop"
   - 窗口：1200×800，最小 800×600
   - `transparent: true`，`titleBarStyle: "Overlay"`
5. **配置 Cargo.toml**：添加 `tauri-plugin-shell` 依赖
6. **配置 vite.config.ts**：集成 Tailwind CSS v4 插件
7. **配置 .gitignore**：覆盖 node_modules、target、.env 等
8. **创建 .env.example**：OpenAI API 环境变量模板
9. **更新 package.json scripts**：添加 `tauri:dev`、`tauri:build`、`check` 脚本

### 产出文件清单

```
项目启动/NCdesktop/
├── package.json              # 前端依赖与脚本
├── pnpm-lock.yaml            # 依赖锁文件
├── tsconfig.json             # TypeScript 配置（strict: true）
├── tsconfig.app.json         # 应用级 TS 配置
├── tsconfig.node.json        # Node 端 TS 配置
├── vite.config.ts            # Vite 6 + Tailwind CSS
├── .gitignore                # Git 忽略规则
├── .env.example              # 环境变量模板
├── src-tauri/
│   ├── Cargo.toml            # Rust 依赖
│   ├── tauri.conf.json       # Tauri 窗口与权限配置
│   ├── capabilities/default.json  # 权限声明
│   ├── src/main.rs           # Rust 入口
│   ├── src/lib.rs            # Tauri Builder 配置
│   ├── build.rs              # 构建脚本
│   └── icons/                # 应用图标
└── node_modules/             # 依赖（不提交）
```

### 实际依赖版本

| 包 | 版本 |
|----|------|
| React | 19.2.4 |
| TypeScript | 5.9.3 |
| Vite | 8.0.2 |
| Tailwind CSS | 4.2.2 |
| Tauri CLI | 2.10.1 |
| Tauri (Rust) | 2.10.3 |
| Zustand | 5.0.12 |
| Lucide React | 1.6.0 |
| React Router | 7.13.2 |

### 验收结果

| 检查项 | 结果 |
|--------|------|
| TypeScript strict: true | ✅ |
| Vite dev server 启动成功 | ✅ (http://localhost:5173) |
| .gitignore 包含必要规则 | ✅ |

---

## Step 3：Liquid Glass 设计系统基础

### 执行内容

1. **创建 globals.css**：完整设计令牌系统
   - 模糊值令牌：6 个（`--glass-blur-xs` ~ `--glass-blur-2xl`）
   - 背景色令牌：5×2 个（亮色+暗色模式）
   - 边框色令牌：4×2 个
   - 阴影令牌：5 个
   - 圆角令牌：7 个
   - 间距令牌：11 个（4px 基准网格）
   - 品牌色令牌：6 个（navy 3 + gold 3）
   - 语义色令牌：6 个
   - 文本色令牌：5×2 个
   - 排版令牌：字体栈、8 级字号、行高、字间距
   - 动画令牌：5 个时长 + 4 个缓动函数
   - 暗色模式覆盖（`[data-theme="dark"]` + `prefers-color-scheme: dark`）
   - 窗口透明基础（html/body/#root transparent）
   - 全局滚动条美化

2. **创建 glass.css**：Liquid Glass 材质层级 + 组件
   - L2 `.glass-sidebar`（blur 12px, thin 背景）
   - L3 `.glass-panel`（blur 16px, regular 背景）
   - L3 `.glass-toolbar`（blur 24px, regular 背景）
   - L4 `.glass-card-elevated`（blur 24px, thick 背景）
   - L5 `.glass-popover`（blur 32px, ultra-thick 背景）
   - `.glass-interactive`：hover/active 交互动效
   - `.btn-glass` / `.btn-glass-accent`：玻璃按钮
   - `.input-glass`：玻璃输入框
   - `.sidebar-item`：侧边栏列表项（含选中态金色指示条）
   - `.text-on-glass`：玻璃上文字增强
   - 焦点指示器（无障碍）
   - 模态框入场动画
   - 标题栏拖拽区域
   - `prefers-reduced-transparency` 回退
   - `prefers-reduced-motion` 回退

3. **更新 main.tsx**：引入 globals.css 和 glass.css

### 产出文件

```
src/styles/globals.css    # 设计令牌（~150 行）
src/styles/glass.css      # 玻璃材质系统（~250 行）
```

### 验收结果

| 检查项 | 结果 |
|--------|------|
| 所有令牌变量已定义 | ✅ |
| 暗色模式覆盖生效 | ✅ |
| 基础材质类可用 | ✅ |
| 降低透明度回退 | ✅ |
| 减少动效回退 | ✅ |
| 4px 基准网格一致 | ✅ |

---

## Step 4：应用窗口与三栏布局骨架

### 执行内容

1. **TitleBar.tsx**：标题栏组件
   - 高度 52px，`glass-toolbar` 材质
   - `-webkit-app-region: drag` 拖拽移动窗口
   - macOS 红绿灯按钮区域留白 78px
   - 居中显示应用名称

2. **Sidebar.tsx**：侧边栏组件
   - `glass-sidebar` L2 材质
   - 品牌标识区（NCdesktop + KNOWLEDGE LIBRARY）
   - 导航项：Search、Recent（选中态）、Starred
   - 项目树、标签树（含使用次数）
   - 底部状态栏（Settings、TF Card Connected）
   - Lucide React 图标，品牌金色

3. **ContentArea.tsx**：内容区组件
   - 上半部分：素材预览面板占位
   - 下半部分：时间轴区域占位（180px 高）
   - Flex 布局自适应

4. **Inspector.tsx**：Inspector 侧边面板
   - 可折叠（通过 `isOpen` 控制）
   - 素材详情区 + AI 分析区 + 建议标签区
   - 关闭按钮

5. **ResizeHandle.tsx**：面板分隔线
   - 4px 宽拖拽区域
   - hover 高亮反馈
   - 拖拽中金色指示线

6. **useResizable.ts**：面板宽度拖拽 Hook
   - 支持最小/最大宽度约束
   - 支持左右方向
   - mousedown/mousemove/mouseup 事件管理
   - 拖拽时禁止文本选择

7. **AppLayout.tsx**：主布局容器
   - 响应式三模式：
     - ≥1200px → 三栏（Sidebar + Content + Inspector）
     - 700-1199px → 两栏（Sidebar + Content）
     - <700px → 单栏（Content）
   - Sidebar 宽度可拖拽调节（160-300px，默认 220px）
   - Inspector 可折叠；**第三栏宽度可拖拽**（260–960px，默认 320px）— 详见「迭代更新记录」

8. **App.tsx**：根组件
   - 仅渲染 `<AppLayout />`

### 产出文件

```
src/App.tsx
src/hooks/useResizable.ts
src/components/layout/TitleBar.tsx
src/components/layout/Sidebar.tsx
src/components/layout/ContentArea.tsx
src/components/layout/Inspector.tsx
src/components/layout/ResizeHandle.tsx
src/components/layout/AppLayout.tsx
```

### 验收结果

| 检查项 | 结果 |
|--------|------|
| `tsc --noEmit` 零错误 | ✅ |
| Vite dev server 正常渲染 HTML | ✅ |
| 标题栏 52px + 拖拽区域 | ✅ |
| 三栏布局组件齐全 | ✅ |
| Sidebar 可拖拽调节宽度 | ✅ |
| Inspector 可折叠 | ✅ |
| 响应式断点逻辑 | ✅ |
| Liquid Glass 材质已应用 | ✅ |
| 间距遵循 4px 网格 | ✅ |

---

## 迭代更新记录（2026-03-26）

以下为本阶段在 Step 1-4 基线之上合入的功能与数据层变更，便于评审与接续开发对照。

### 1. 布局与右栏

| 项 | 说明 |
|----|------|
| **Inspector 与全局状态统一** | `AppLayout` 中右栏显隐与 `useUIStore.inspectorOpen` 一致，避免快捷键与界面不同步。 |
| **第三栏可拖拽加宽** | 在「中间内容区」与「Inspector/时间流」之间增加 `ResizeHandle` + `useResizable`（`direction: "left"`），宽度约 260–960px；`ContentArea` 增加 `min-w-0` 避免挤压失效。 |
| **右栏双模式** | `uiStore.rightPanelMode`：`inspector` \| `timeline-flow`。Inspector 为素材详情；时间流为演示用瀑布布局（`TimelineFlowView` + `demo-timeline-data.ts`）。右下角胶囊切换。 |
| **类型** | `src/types/ui.ts` 增加 `RightPanelMode`。 |

### 2. 悬浮窗（Dropzone）

| 项 | 说明 |
|----|------|
| **外观** | 外层留白 + 内层大圆角卡片（约 28px）、渐变底与描边；顶栏拖动区与右下角缩放区对齐圆角。 |
| **响应式** | 主区域 `flex-1 min-h-0`；展开列表现可滚动；中央 Drop 按钮尺寸 `clamp` 随窗口变化。 |
| **说明** | 未启用 Rust 侧 `WebviewWindowBuilder::transparent`（默认 macOS 构建需 `macos-private-api`）；窗口背景为实色时由前端铺底。 |

### 3. 中间栏：访达式视图 + 原件/工作区双栏

| 项 | 说明 |
|----|------|
| **工具栏** | 进入具体项目后：图标/列表切换绑定 **`assetStore.viewMode`**（与项目列表的 `projectStore.viewMode` 分离）。 |
| **双栏主界面** | 左栏 **「导入原件」**：展示 `originalName`（或回退 `name`）、按 **`imported_at` 新→旧** 排序；`sourceData` 有值时 tooltip 显示原件路径。右栏 **「工作区」**：当前副本 **`name`**、AI 标签、`organized/<分类>/` 路径推断的主题目录、与左栏 **同一 `assetId` 联动选中**。 |
| **列表/图标** | 两栏同时遵循当前视图模式（列表或图标网格）。 |

### 4. 数据层：原件名与标签映射

| 项 | 说明 |
|----|------|
| **迁移 V2** | `assets.original_name`：`ALTER TABLE` + 旧数据用 `name` 回填。 |
| **导入语义** | 悬浮窗导入已使用 **`fs::copy`** 至 `app_data/assets/<projectId>/`，**不修改用户磁盘上的源文件**；`source_data` 存原件路径字符串。AI 整理仅 `update_name_and_path` 更新副本的 `name` 与 `file_path`，**不覆盖 `original_name`**。 |
| **Rust** | `models::Asset` 增加 `original_name`（`#[serde(default)]` 兼容旧 JSON）；`get_by_project` 排序改为 **`ORDER BY imported_at DESC`**；新增命令 **`get_project_asset_tag_map`** → `HashMap<assetId, tagNames[]>`。 |
| **前端** | `Asset` 增加 `originalName`、`sourceData`；`assetStore` 在 `fetchAssets` / `fetchAssetsByTag` 后并行拉取标签映射 **`assetTagNamesById`**。 |

### 5. 演示与静态资源

| 文件 | 用途 |
|------|------|
| `src/lib/demo-timeline-data.ts` | 时间流右栏演示数据（录音 / 图片 / 文档锚点与 Time Tag）。 |
| `public/demo/timeline-flow-seed.json` | 精简 JSON 样例，可供后续导入或对齐工具使用。 |

### 6. 开发时注意

- **重启开发进程**：`pnpm tauri:dev`；若 **5173 端口占用**需先结束旧 Vite 再启动。以 **Tauri 弹出窗口** 为准验证（浏览器单独打开 localhost 时 `invoke`/`listen` 会报错属预期）。
- **数据库**：本地 `notecapt.db` 会在启动时自动执行迁移；若需从零验证 V2，可备份后删除库文件再启动。

---

## 项目文件结构总览（含迭代后）

```
项目启动/NCdesktop/
├── public/
│   └── demo/
│       └── timeline-flow-seed.json   # 时间流演示种子（可选）
├── src/
│   ├── components/
│   │   ├── features/
│   │   │   ├── AssetListView.tsx     # 项目内：原件/工作区双栏 + 访达式列表/图标
│   │   │   ├── timeline-flow/
│   │   │   │   └── TimelineFlowView.tsx
│   │   │   └── dropzone/             # 悬浮窗各状态组件
│   │   └── layout/
│   │       ├── AppLayout.tsx         # 含侧栏与第三栏拖拽、Inspector 显隐
│   │       ├── Inspector.tsx         # Inspector / 时间流 + 右下角模式切换
│   │       ├── ContentArea.tsx
│   │       ├── Toolbar.tsx
│   │       └── ResizeHandle.tsx
│   ├── hooks/
│   │   ├── useResizable.ts
│   │   └── useProjectWorkspaceSync.ts
│   ├── lib/
│   │   ├── tauri-commands.ts         # 含 getProjectAssetTagMap 等
│   │   └── demo-timeline-data.ts
│   ├── stores/
│   │   ├── uiStore.ts                # inspectorOpen、rightPanelMode、…
│   │   └── assetStore.ts             # assets、assetTagNamesById、viewMode
│   ├── types/
│   │   ├── asset.ts                  # originalName、sourceData
│   │   └── ui.ts                     # RightPanelMode、AssetViewMode、…
│   └── …
├── src-tauri/
│   ├── src/
│   │   ├── db/
│   │   │   ├── migration.rs          # V2 original_name
│   │   │   └── asset.rs              # insert/select、get_tag_names_by_project
│   │   ├── models/asset.rs
│   │   ├── commands/
│   │   │   ├── asset.rs              # get_project_asset_tag_map
│   │   │   └── dropzone.rs           # import 复制与 AI 后台任务
│   │   └── lib.rs                    # invoke 注册
│   └── …
└── …
```

---

## 汇合点 C1 准备状态（修订）

| 契约 | 文件 | 状态 |
|------|------|------|
| TypeScript 类型契约 | `src/types/*.ts` | 🔄 持续演进（已含 Asset 扩展、RightPanelMode） |
| IPC 命令签名契约 | `src/lib/tauri-commands.ts` | 🔄 已含 `getProjectAssetTagMap` 等 |
| Zustand store 接口 | `src/stores/*.ts` | 🔄 已含 `assetTagNamesById`、`rightPanelMode` |
| Tauri Event 命名契约 | `notecapt/import-drop-finished`、`notecapt/dropzone-ai-finished` | ✅ 沿用 |

**下一步建议（接续并行阶段）**：在《并行开发文档》框架下继续 W1/W2 任务；时间流与真实素材锚点、Inspector 与双栏的深层联动等，以 PRD 与代码中 `TODO`/迭代记录为准。

---

## 后续 AI 阅读指引

如果你是接续开发的 AI Agent，请按以下顺序阅读：

1. **项目宪章**：`.cursor/rules/project-development-charter.mdc`
2. **20 步开发框架**：`（cursor）NCdesktop 20 步开发框架宪章.md`
3. **并行开发文档**：`（cursor）并行开发文档.md`
4. **本文档**：`项目启动/开发过程文档-Step1-4.md`（Step 1-4 基线 + **「迭代更新记录（2026-03-26）」**）
5. **设计宪章**：`Liquid-Glass-UI-设计宪章.md`（UI 开发参考）
6. **软件 PRD**：`NCdesktop-软件PRD.md`（功能规格和数据模型权威定义）

**当前项目代码路径**：`项目启动/NCdesktop/`

**并行阶段提示**：汇合点 C1 的契约状态见本文档 **「汇合点 C1 准备状态（修订）」**；数据与 IPC 已在迭代中部分落地，接续开发请以代码与迭代记录为准。

---
