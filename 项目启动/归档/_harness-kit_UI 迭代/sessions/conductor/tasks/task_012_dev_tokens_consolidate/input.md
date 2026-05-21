# Task 输入 — task_012_dev_tokens_consolidate

## 目标

在 `src/styles/globals.css` 新增/调整一组视觉令牌，并在项目内统一替换：
- 新增：`--sidebar-active-bg`、`--sidebar-active-fg`、`--hub-count-bg`、`--hub-count-fg`、`--accent-amber`、`--accent-amber-soft`
- 同步暗色模式覆盖
- 替换全项目内行内 amber 色（导航选中态）为 `--sidebar-active-*`
- 琥珀仅保留三处：① 重复概念合并提示 ② AI 强调框 ③ 时间流图片 zone stripe
- 动效统一三档 duration：`--duration-instant 100ms`、`--duration-fast 200ms`、`--duration-normal 300ms`，删除其它 duration 字面量

## 前置条件

- 依赖 task：**task_002 ~ 011**（必须在所有改造完成后做最后清理）
- 必须先存在的文件/接口：
  - `src/styles/globals.css`

## 验收标准（Acceptance Criteria）

1. **AC-1**：`globals.css` `:root` 块中存在所有新 token（按 PRD §8.1）：
   ```css
   --sidebar-active-bg: rgba(59, 130, 246, .15);
   --sidebar-active-fg: #93c5fd;
   --hub-count-bg: var(--surface-tertiary);
   --hub-count-fg: var(--text-tertiary);
   --accent-amber: #ea580c;
   --accent-amber-soft: #fff7ed;
   ```
2. **AC-2**：暗色模式覆盖：
   ```css
   @media (prefers-color-scheme: dark) { /* 或 .dark { ... } 视项目惯例 */
     --sidebar-active-bg: rgba(59, 130, 246, .18);
     --accent-amber-soft: #431407;
   }
   ```
3. **AC-3**：所有项目内"导航选中态"统一用 `var(--sidebar-active-bg)` + `var(--sidebar-active-fg)`；grep `src/` 不存在 `#fff7ed`、`#ea580c` 等行内 hex 用于导航选中
4. **AC-4**：琥珀色仅出现在三处：① ConceptsStep 的重复概念合并提示 ② Inspector AI 框 ③ MagicMoment / 时间流图片 zone stripe；其它地方的 amber 全部替换或删除
5. **AC-5**：动效 duration：grep `src/` 后，所有 `transition-duration: NNNms`、`animation-duration: NNNms`、`duration-NNN`（Tailwind 类）的字面量被替换为三档 token 之一
6. **AC-6**：dark mode 下所有上述变化的对比度 ≥ WCAG AA（手测：用 macOS 系统切换到 dark，肉眼或 Lighthouse 验证）
7. **AC-7**：`pnpm check` + `pnpm lint` + `pnpm test` 全绿；现有快照测试如有不一致需更新

## 技术约束

- **token 集中声明**：所有新 token 在 `:root` 中声明；不要散布到具体组件的 css 文件
- **dark mode 实现方式**：与 globals.css 现有惯例对齐（先看 globals.css 是用 `.dark` 类还是 `@media (prefers-color-scheme: dark)`）
- **不引入 sass/postcss 自定义函数**：纯 CSS var
- **不动 brand 色**：navy / gold 等品牌色保留
- **替换工作**：可用 `grep -rn "amber" src/` 找现有引用，逐个替换；保留三处琥珀（标注 comment "保留：重复概念合并 / AI 强调 / 时间流"）

## 参考文件

- `src/styles/globals.css`（当前 :root 块）
- `product/prd/notecapt-v1.3-ui_prd_v1.md` §8 TK-01 ~ TK-04

## 预估影响范围

- **修改文件**：
  - `src/styles/globals.css`（新增 token）
  - 多个组件文件（grep amber 后逐个替换）
  - 可能：`src/components/features/dropzone/Dropzone*.tsx`（动效 duration 收敛）

- **新建文件**：无

---

## Reviewer 重点关注项

- 暗色对比度实测 ≥ WCAG AA（建议用 Chrome DevTools 的对比度检查或 axe）
- amber 三处保留位置是否真正合理（grep 后确认每处都对应 PRD 列出的三类）
- 三档 duration 替换是否完整（不再有任何 `100ms`/`200ms`/`300ms` 之外的 duration 字面量）
- 旧 `--color-accent` 是否被废弃或重新指向新 token（避免引用悬空）
