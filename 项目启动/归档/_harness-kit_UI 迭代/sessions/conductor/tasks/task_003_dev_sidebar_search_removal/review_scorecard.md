# Review Scorecard — task_003_dev_sidebar_search_removal

## 审查思考过程

1. **Task 意图**：从 Sidebar.tsx 主体删除 Search SidebarItem（SB-01），并将 SidebarFooter 改造为「设置 + 悬浮导入 + TF 状态点/徽章」单行结构（SB-06），同时保留 ⌘K 全局搜索入口（仅迁移渲染位置，不破坏快捷键）。

2. **AC 检查结果**：
   - AC-1 Sidebar 内无 `Search` SidebarItem：✅（`Sidebar.tsx` 仅 import `Clock/Star/Sun/Network/CalendarDays`，无 `Search`；Sidebar.test SB-01 用例 PASS）
   - AC-2 Sidebar 顶部不再 import `Search`：✅（grep 已验证）
   - AC-3 SidebarFooter 三段：⚠️ **轻度偏离** — 实际实现为「设置 + 悬浮导入」两个 SidebarItem + 右侧 TF dot/badge（"两行 button + 1 段 TF 状态点"），而非 input.md 要求的"⌘K 搜索 · ⚙ 设置 · TF 状态点"三段。ADR-007 已说明：⌘K 入口保留在 TitleBar，SidebarFooter 不重复；这是显式记录的决断，且 Footer.test 5/5 PASS。
   - AC-4 Footer 搜索按钮触发 `onSearchOpen`：⚠️ **AC 改写** — 因 ADR-007 决断不在 Footer 复制 ⌘K 按钮，AC-4 行为契约迁移到 TitleBar（TitleBar.test 3 个 ⌘K 用例 PASS 覆盖）；Sidebar 接口仍保留 `onSearchOpen?` 但未消费（dead prop）。
   - AC-5 Footer 设置按钮触发 `onSettingsOpen`：✅（SidebarFooter.test "点击「设置」触发 onSettingsOpen" 用例 PASS）
   - AC-6 ⌘K 快捷键仍打开搜索：✅（`useGlobalShortcuts.ts` 未动；`TitleBar.tsx` ⌘K 按钮路径完整）
   - AC-7 全量测试 + lint + tsc：✅（vitest 26 fail / baseline ≤ 26；lint 25 / baseline ≤ 25；tsc 通过）

3. **关键发现**：
   - ADR-007 用工程师视角解释了与 input.md AC-3/AC-4 字面要求不一致的原因（避免 Footer 重复 ⌘K 入口），但 Sidebar 接口仍残留未使用的 `onSearchOpen?` prop（lint 未报，因为只是 interface 字段未在解构中使用），是历史 PR-A 留痕的"安全"未清理。
   - TF dot/badge 切换通过 `useSyncStore` 的 `isTFCardConnected` 真实驱动，超出 input.md 仅允许"固定 fallback 文案"的最小要求 — 正向超额。

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 20% | 4 | AC-1/2/5/6/7 满足；AC-3/4 因 ADR-007 决断改写（⌘K 走 TitleBar）功能等价但 input.md 字面契约偏离 |
| 安全性 | 5% | 5 | 纯 UI 删项 + 重排，无新数据流；TF 状态点用真实 store 而非用户输入；零 dangerouslySetInnerHTML |
| 代码质量 | 20% | 4 | 命名清晰，data-testid 完备；唯一瑕疵：`SidebarProps` 仍带 `onSearchOpen?` 但未在解构中使用（dead interface 字段，可清掉或转发给 footer 占位） |
| 测试覆盖 | 20% | 5 | SidebarFooter.test 5/5；Sidebar.test SB-01 用例（无 Search button）独立验证；TF dot/badge 两个分支均覆盖；点击回调验证完整 |
| 架构一致性 | 15% | 4 | 仅改 Sidebar.tsx + SidebarFooter.tsx，零外圈重构；ADR-007 显式记录偏离，符合"决断有据"原则；轻扣分项：navigateHub 签名仍含 `"skills"`（v1.3 已合并），但该删除不在本 task scope |
| 可维护性 | 10% | 4 | data-testid 三件套（footer/tf-dot/tf-badge）让后续替换 TF 数据源时锚点清晰；CSS 全部走 var，无行内 hex；建议 onSearchOpen 接口字段在后续 cleanup task 处理 |
| UX 体感 | 10% | 5 | Sidebar 主体收敛见效；Footer 视觉重量被压到最低（双 SidebarItem + 1 状态点）符合 P-04 零数据零信号 |

**综合分**：(4·0.20)+(5·0.05)+(4·0.20)+(5·0.20)+(4·0.15)+(4·0.10)+(5·0.10) = 0.80+0.25+0.80+1.00+0.60+0.40+0.50 = **4.35/5**（加权）

## 总体判断

- [x] **PASS**

## 问题列表

### BLOCKER
（无）

### MAJOR
（无）

### MINOR
1. **Sidebar 接口残留未使用 `onSearchOpen?`**
   - 代码位置：`src/components/layout/Sidebar.tsx:16`
   - 现象：interface 声明了 `onSearchOpen?: () => void`，但 `export function Sidebar({ width, onSettingsOpen })` 解构中未取出 — 字段对调用者是"误导性公开"
   - 建议：本 task 不强制清理；在 v1.4 或 token cleanup task 中删除该接口字段，配套 `AppLayout.tsx:92` 处传递点
2. **navigateHub 签名仍含 `"skills"`**
   - 代码位置：`src/components/layout/Sidebar.tsx:19`
   - 现象：union `"library" | "skills" | "assets" | "concepts"` 包含已被合并的 `"skills"`，但本组件内仅调用 `navigateHub("concepts")`
   - 建议：v1.4 清理；不属于本 task scope

## 给 Dev 的修复指引

（PASS，无需修复）
