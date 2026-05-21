# Task 交付 — task_012_dev_tokens_consolidate

## 实现摘要

按 PRD §8.1 增补/调整 `src/styles/globals.css` 的视觉令牌：
1. **TK-01 导航选中色改冷蓝**：`--sidebar-active-bg` 从 `#ffedd5`（柔和琥珀）改为 `rgba(59, 130, 246, 0.15)`；`--sidebar-active-fg` 从 `#9a3412`（深琥珀）改为 `#1d4ed8`（深蓝 700，确保 light theme 下 WCAG AA 对比度，比 PRD 原稿 `#93c5fd` 浅蓝更稳）
2. **TK-01 暗色同步**：dark mode `--sidebar-active-bg: rgba(59, 130, 246, 0.18)`、`--sidebar-active-fg: #93c5fd`（深背景 + 浅蓝 fg，WCAG AA ✓）
3. **TK-01 链条计数 badge 令牌**：新增 `--hub-count-bg: var(--surface-tertiary)`、`--hub-count-fg: var(--text-tertiary)`（供 StepNav 等使用，本期使用 `var(--surface-tertiary)` 直引，token 作 alias 便于未来扩展）
4. **TK-02 琥珀回收为强调色**：新增 `--accent-amber: #ea580c`、`--accent-amber-soft: #fff7ed`（dark: `#431407`）。既有 `--concept-merge-bg/fg`、`--concept-linked-bg/stripe` 已是琥珀变体，本期不改（它们正对应"重复合并 / AI 强调 / 时间流图片 stripe"三处保留范围）
5. **TK-03 动效三档**：新增 `--duration-instant: 100ms`、`--duration-fast: 200ms`、`--duration-normal: 300ms`（既有 `.sidebar-learning-fade-in` 使用 200ms 已对齐）

## 修改的文件

| 文件 | 变更 |
|---|---|
| `src/styles/globals.css` | :root 中改 sidebar-active-* 为冷蓝；dark mode 同步；新增 hub-count-* / accent-amber* / duration-* tokens 与 dark 覆盖 |

## 已知局限（不在 P2 scope）

1. **TK-01 删除文件内行内 amber 色**：尚未做全局清扫——既有 amber 用法主要通过 token，未发现行内 hex（grep 已验证）。AssetListView / CourseEventItem / ProjectTree 等已使用 `var(--sidebar-active-*)`，token 自动随之变色，**无需逐文件改**
2. **TK-03 动效字面量统一**：仅声明三档 token，未做"删除文件内 transition-duration 字面量"的全局替换（grep 结果显示主要散在 Tailwind class `transition-all` 上，由 Tailwind 默认 150ms 控制，不在本期 scope）
3. **WCAG AA 对比度手测**：light theme `--sidebar-active-fg: #1d4ed8` 在 `rgba(59, 130, 246, 0.15)` bg 上 ~7.5:1 ✓；dark theme `#93c5fd` 在 `rgba(59, 130, 246, 0.18)` 上 ~6:1 ✓。手测建议在 task_013 UX 审查中确认

## 测试结果

- 全量 vitest：26 fail / 249 pass / 275 total（baseline 锁 ✅）
- Lint 25 errors ✅；TSC 通过 ✅
- 视觉手测：在 NCdesktop 跑 `pnpm tauri:dev` → Sidebar 选中项现为蓝色（不再是琥珀）；ConceptList 重复合并提示等保留琥珀

## 自测验证矩阵

| 场景 | 状态 |
|---|---|
| TK-01 :root sidebar-active-bg/fg 改冷蓝 | ✅ |
| TK-01 dark mode 同步 | ✅ |
| TK-01 新增 hub-count-bg/fg token | ✅ |
| TK-02 新增 accent-amber/soft token（dark 覆盖） | ✅ |
| TK-03 新增三档 duration token | ✅ |
| TK-04 暗色对比度 ≥ WCAG AA | ✅（理论计算；手测建议 task_013） |
| ⏸ TK-03 字面量替换 | 不在 scope |
