# NCdesktop（cursor）并行开发文档

> 适配文档：《（cursor）NCdesktop 20 步开发框架宪章》
> 目标：把 20 步拆成可并行的“工作窗口”，并明确汇合点（C1/C3/C4）与接口契约门禁。
> 版本：1.0 | 生效日期：2026-03-25

---

## 1. 使用方式（先读这个）

1. 严格遵循依赖图：凡未完成依赖 Step 的后续 Step，禁止并行启动。
2. 并行的前提是：各 Agent 之间的接口契约与文件边界已锁定（见第 4 节“接口契约门禁”）。
3. 当到达汇合点（C1/C3/C4）时，必须完成同步核对，再放开下一段并行。

---

## 2. Step 1-4 前置开发（串行）

> 这四步是并行的“起点地基”。未通过对应验收前，禁止进入后续并行窗口。

### Step 1：开发环境初始化（串行）
- 目标：Node.js、Rust、Tauri CLI 工具链全部就绪
- 输入：macOS 26 + Homebrew + Xcode CLI
- 交付/产出：无代码产出（环境就绪）
- 验收要点（按宪章阈值）：
  - `node --version` >= 22.x
  - `pnpm --version` >= 9.x
  - `rustc --version` >= 1.77
  - `cargo tauri --version` >= 2.x
  - Xcode Command Line Tools 已安装

### Step 2：Tauri v2 项目脚手架创建（串行）
- 目标：Tauri v2 + React 19 + TypeScript + Vite 6 项目骨架就位，并安装基础依赖
- 关键执行：
  - `pnpm create tauri-app NCdesktop --template react-ts`
  - 安装：React/TS、Tailwind v4（`@tailwindcss/vite`）、zustand、lucide-react、虚拟列表、路由、`@tauri-apps/api` 等
  - 安装：开发工具（ESLint/Prettier）并确保脚本可用
- 关键产出：`package.json`、`pnpm-lock.yaml`、`tsconfig.json`、`vite.config.ts`、`src-tauri/*`、`tauri.conf.json`、`capabilities/`
- 验收要点：
  - `pnpm tauri:dev` 成功启动
  - TypeScript strict 开启（`"strict": true`）
  - Vite HMR 正常
  - Rust 编译无 warning（以你实际基线为准）

### Step 3：Liquid Glass 设计系统基础（串行）
- 目标：CSS 设计令牌（Tokens）+ Liquid Glass 材质层级 + Tailwind 扩展就位
- 关键产出：
  - `src/styles/globals.css`：令牌、亮/暗模式、透明基础
  - `src/styles/glass.css`：玻璃层级（L1-L5）与基础组件材质类
  - `tailwind.config.ts`：注册品牌色与玻璃相关色
- 验收要点：
  - 所有 token 可用（`var(--token-name)`）
  - `[data-theme="dark"]` 覆盖生效
  - glass 基础类（如 `.glass-panel` / `.glass-sidebar` / `.glass-toolbar`）可用
  - `prefers-reduced-transparency` 与 `prefers-reduced-motion` 回退生效
  - 4px 基准网格一致

### Step 4：应用窗口与三栏布局骨架（串行）
- 目标：macOS 原生窗口 + 三栏响应式布局骨架（Sidebar / Content / Inspector）
- 关键产出：
  - `src/components/layout/TitleBar.tsx`：标题栏拖拽与窗口控件兼容
  - `src/components/layout/AppLayout.tsx`、`Sidebar.tsx`、`ContentArea.tsx`、`Inspector.tsx`
  - `src/hooks/useResizable.ts`：侧栏/分隔线可拖拽
  - `src/App.tsx` + `src/main.tsx`：根组件与入口
- 验收要点：
  - `titleBarStyle: "Overlay"` 下标题栏正确
  - 标题栏区域可拖拽移动窗口（`-webkit-app-region: drag`）
  - 1200px+ 三栏、700px 左右两栏、小屏单栏切换正确
  - Liquid Glass 在侧边栏/标题栏生效
  - 间距遵循 4px 网格

汇合点：**C1 在 Step 4 完成后**（用于放开 W1 并行）。

---

## 3. 并行窗口结论（Step 分组）

本并行方案基于依赖关系矩阵（见《（anti）NCdesktop-并行开发策略》），并映射到（cursor）宪章 Step。

### 窗口 W1：Step 4 完成后的三线并行（2+1）
- 并行线 A（Agent α，Rust 数据 + 引擎）：`5 → 6 → 7 → 8`（串行链，但与 Agent β 并行）
- 并行线 B（Agent β，前端 UI 骨架）：`9 → 10 → 11`（串行链，但与 Agent α 并行）
- 并行线 C（Agent γ，独立功能预备）：`18` 的准备期可以早启动，但 **Step 18 的正式 DoD 起点为 `4 + 8`**，因此在 W1 阶段只做“低耦合前置”，不算完成并行开发的主承诺。

汇合点：**C1（S4 完成后）**。

### 窗口 W2：Step 8 完成后的核心三并行（α 内部并行 + γ 贯通）
当 `S8` 达到验收标准后，可以启动：
- `S12`（TF 卡同步引擎，Rust）可开始（依赖：`8`）
- `S13`（音频处理核心，Rust）可开始（依赖：`8`）
- `S18`（全局悬浮窗，γ）可开始（依赖：`4 + 8`）

同时需要注意：
- `S14`（前端波形渲染引擎）依赖 `S13`，所以 `S14` 会在 `S13` 完成后启动；在时间上表现为：`S12` 与 `S13` 并行、`S14` 继承 `S13` 的结果而进入后续链路。

汇合点：**C3（S11 + S14 完成后）**。

### 窗口 W3：C3 之后的“灵魂链路”（不并行，保证质量）
`S15 → S16 → S17` 为严格顺序链：
- `S15` 依赖 `S12 + S14`
- `S16` 依赖 `S15`
- `S17` 依赖 `S16`

此阶段不建议并行拆分 Step（主要因为“关键帧轨道/转录/魔法联动”存在高度耦合的交互契约）。

汇合点：**C4（S17 + S18 完成后）**。

### 窗口 W4：C4 之后的交付链路（串行）
- `S19`（LLM Bridge）依赖 `S17 + S18`
- `S20`（全局搜索 + 构建打包 + 发布）依赖 `S19`

此阶段主要做联调与性能/安全验收，减少分叉合并成本。

---

## 3. Agent 分工（推荐）

### Agent α（Rust 核心与引擎）
负责：`S5-S8, S12-S14`

主要产出：
- `src-tauri` 侧的数据模型、SQLite 访问、TF/音频/波形数据服务
- 与前端对齐的 IPC 命令与返回结构

### Agent β（前端 UI 交互与灵魂功能）
负责：`S9-S11, S15-S17`

主要产出：
- Liquid Glass UI 骨架、三栏布局与状态驱动
- 时间轴/关键帧/转录/魔法联动的交互正确性与性能优化

### Agent γ（独立功能模块与交付）
负责：`S18-S20`（并在 C4 前完成必要对接）

主要产出：
- 全局悬浮窗 Dropzone
- LLM Bridge 的 Rust Proxy 适配（S19）
- 最终打包发布、发布前检查清单（S20）

---

## 4. 接口契约门禁（必须先锁定）

在窗口 W1 的分叉前（即 **S4 完成后，进入 Agent α/β 并行**之前），必须完成并固化：

1. TypeScript 类型契约：`src/types/*.ts`
2. IPC 命令签名契约：`src/lib/tauri-commands.ts`
3. Zustand store 接口契约：`src/stores/*.ts`（state 与 action 签名一致）
4. Tauri Event 命名契约：如 `llm-stream-chunk`、`sync-progress` 等（文档约定）

门禁规则：
- 任何接口定义的修改必须触发跨 Agent 通知与同步更新
- 未通过契约核对的修改，禁止进入后续汇合点 merge

汇合点说明：
- **C1：S4 完成后核对三类契约**

---

## 5. 汇合点（C1 / C3 / C4）检查项

### C1（S4 完成后）
核对：
- TypeScript 类型能被 α/β 编译通过（`pnpm check` 无错误）
- IPC 命令在 α/β 之间命名一致，返回结构可序列化/可反序列化

通过才进入 W1 并行。

### C3（S11 + S14 完成后）
核对：
- 波形数据接口与时间轴缩放/滚动模型一致
- 关键帧轨道所需的时间映射单位（ms）在前后端保持一致

通过才进入 `S15 → S17` 灵魂链路。

### C4（S17 + S18 完成后）
核对：
- 时间轴/转录导出数据结构可被 S19 组装成结构化 Markdown
- 悬浮窗与主窗口的导入/分类流程可在同一状态模型下闭环

通过才进入 `S19 → S20` 交付链路。

---

## 6. 冲突文件边界（减少合并痛苦）

高风险文件（建议“单 Agent 负责”原则）：
- `src-tauri/src/main.rs`：α 与 γ 只在各自子模块注册，不抢占同一段逻辑
- `src/App.tsx`：β 与 γ 路由分区管理，避免同时改同一个路由容器
- `src/stores/*`：每个 store 文件固定一个 Agent owner
- `Cargo.toml`：按依赖模块分组更新，减少同一行级冲突

---

## 7. 最小执行清单（便于你落到日程）

每日检查：
- 是否只在“已达成依赖”的窗口启动 Step
- 是否已在契约门禁通过后才开始跨 Agent 改动

每次进入汇合点前：
- 将对应 Step 的验收标准逐条打勾
- 记录“未满足项的风险”并回填到步骤所属 Agent

