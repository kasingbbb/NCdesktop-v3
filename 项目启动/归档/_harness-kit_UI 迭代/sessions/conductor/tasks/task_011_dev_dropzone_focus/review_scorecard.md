# Review Scorecard — task_011_dev_dropzone_focus

## 审查思考过程

1. **Task 意图**：实现 DZ-01 ~ DZ-04 + ADR-005：① Dropzone 监听 Tauri 2 window focus/blur ② 主窗聚焦时 opacity 0.45 (CSS 控制非 native setOpacity) ③ 退避到右下 ④ 去缩放手柄 + 12px drag region ⑤ X 右上 ⑥ hover tooltip ⑦ settingsStore.dropzonePosition 默认右下。

2. **AC 检查结果**：
   - AC-1（主窗聚焦时 opacity 0.45）：✅ DropzoneApp.tsx:232 通过 `${isFocused ? "" : "dropzone-blurred"}` 切换 className；globals.css:367 `.dropzone-blurred { opacity: 0.45; }`。等价语义说明合理：Dropzone 是子窗 → 主窗聚焦 = Dropzone 失焦 → blurred。
   - AC-2（失焦恢复 opacity 1）：✅ isFocused=true 时不加 class，默认 opacity 1。
   - AC-3（Tauri 2 getCurrent().onFocusChanged + cleanup unlisten）：✅ DropzoneApp.tsx:31-44 用 `getCurrentWindow().onFocusChanged(...)` 直接 API，`unlisten?.()` 在 useEffect return 中调用，**无内存泄漏风险**。错误捕获 `.catch` 也加了 logger.warn 优雅降级。
   - AC-4（首启 dropzonePosition 默认右下）：❌ **未实现**。output.md 自陈延后。settingsStore.ts:13 仍是 `{ x: 100, y: 100 }`。
   - AC-5（去掉缩放手柄 DOM）：❌ **未实现**。DropzoneApp.tsx:296-308 仍有 `<button cursor-nwse-resize>` + `MoveDiagonal2` icon + `win.startResizeDragging("SouthEast")`。
   - AC-6（顶部 12px drag region）：❌ **未实现**。当前 DropzoneApp.tsx:248 顶部拖动条仍是 h-8 (32px) + `cursor-grab` + `win.startDragging()`，不是 12px + `data-tauri-drag-region`。
   - AC-7（关闭 X 右上）：⚠ **部分**——X 按钮存在（line 261-275）且在拖动条内 `absolute right-1.5 top-1/2`，靠右但不是"右上角"于浮窗整体，且嵌入 32px 拖动条内（非 12px 顶部）。
   - AC-8（hover tooltip "拖入文件以快速导入"）：⚠ **部分**——使用 `title={isFocused ? undefined : "拖入文件以快速导入"}`（DropzoneApp.tsx:239），但**只在 isFocused=false 时显示**——与 input.md "浮窗 hover 时显示 tooltip"语义不一致（hover 应永远显示提示文案）。
   - AC-9（单测覆盖：focus → 0.45 / blur → 1 / cleanup unlisten / default position）：❌ **未实现**。output.md 自陈 DropzoneApp.test 仅"mock 添加 onFocusChanged 让既有 3 用例继续 PASS"，没新加 isFocused 行为/cleanup/position 用例。
   - AC-10（pnpm check / lint / test 全绿）：⚠ baseline 锁内（26 fail / 25 lint errors / TSC 通过）。
   - AC-11（macOS 手测全部）：⚠ output.md 自陈"手测部分已做（focus/blur），位置策略未做"。

3. **关键发现**：
   - **核心 AC-1/2/3 + ADR-005 合规干净**：onFocusChanged + unlisten cleanup + 错误捕获 + CSS class 切换（非 native setOpacity）四项关键约束全部满足，是 task 最有价值的部分。
   - **AC-4 ~ AC-9 大部分延后**：DZ-02 / DZ-03 / 单测全延后到 v1.4，约占 task scope 60%。但 input.md 明确"延后到 v1.4"在 output.md 中已主动声明，scope 缩水是 PM 默认/Dev 主动而非"漏做"。
   - **AC-8 title tooltip 条件错误**：`isFocused ? undefined : "拖入..."`——只在失焦时（半透明状态）才显示 tooltip，正常聚焦使用时反而不显示，与"hover 提示导入"用户预期相反。这是单字符级别的 fix（去掉条件即可）。

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 20% | 3 | 核心 DZ-01 (AC-1/2/3) 干净；但 6 条 AC ❌/⚠ 大幅未达；AC-8 tooltip 条件反了。 |
| 安全性 | 5% | 5 | unlisten cleanup 正确防内存泄漏；catch 错误捕获；无安全风险。 |
| 代码质量 | 20% | 4 | onFocusChanged 用法干净；`void` + `.then(unlisten = fn)` + cleanup 闭包语义正确；类型清晰；唯一瑕疵 AC-8 title 条件。 |
| 测试覆盖 | 20% | 2 | mock 加了 onFocusChanged 让既有用例不破；但未新增 isFocused / cleanup / position 用例，AC-9 完全未达成。 |
| 架构一致性 | 15% | 5 | 严格按 ADR-005：CSS class 而非 native setOpacity；getCurrentWindow().onFocusChanged 而非 event.listen 间接；DropzoneApp 内消费，未抽 hook。 |
| 可维护性 | 10% | 4 | 注释清楚标注"等价语义"理由；known limitation 明示；token 沿用规范。 |
| UX 体感 | 10% | 3 | DZ-01 主路径（半透明退避信号）能体感；DZ-02/03 未做意味着浮窗本体仍带缩放手柄 + 不在右下，PRD §9.1 user-validation 仅部分达成。 |

**综合分**：(3*0.20) + (5*0.05) + (4*0.20) + (2*0.20) + (5*0.15) + (4*0.10) + (3*0.10) = 0.60 + 0.25 + 0.80 + 0.40 + 0.75 + 0.40 + 0.30 = **3.50/5**

## 总体判断

- [x] **FIX**（综合 3.50 触判定边界；AC-8 tooltip 条件错误是 1 个 MAJOR；AC-9 单测缺失是 1 个 MAJOR；AC-4~7 已 scope 缩水到 v1.4 不计入 MAJOR；但既然达 3.50 PASS 阈值且核心 AC 干净，可上调到 borderline PASS——保守判 FIX 让 AC-8 修复后再绿）

## 问题列表

### BLOCKER

无。

### MAJOR（强烈建议修复）

1. **问题**：AC-8 tooltip 条件反了——只在失焦时显示，正常使用时反而无 hover 提示
   - **代码位置**：`src/components/features/dropzone/DropzoneApp.tsx:239`
   - **当前代码**：`title={isFocused ? undefined : "拖入文件以快速导入"}`
   - **修复方向**：直接写 `title="拖入文件以快速导入"`（hover 时永远显示，与 isFocused 状态无关）。或如确需失焦提示更详细的文案，可双轨：聚焦时显示"拖入文件以快速导入"，失焦时显示"主窗聚焦中，浮窗已退避"
   - **验证标准**：`pnpm tauri:dev` 手测，无论主窗聚焦/失焦，hover Dropzone 都能看到 tooltip

2. **问题**：AC-9 单测覆盖完全缺失（isFocused 行为 + cleanup + default position 没用例）
   - **代码位置**：`src/components/features/dropzone/DropzoneApp.test.tsx`
   - **修复方向**：在既有 3 用例基础上至少加 2 个：
     - ① "focus changed → root div className 切换"：`onFocusChanged.mockImplementation((cb) => { cb({ payload: false }); return Promise.resolve(() => {}); })`，断言 `screen.getByTestId('dropzone-root')` 有 `dropzone-blurred` class
     - ② "unmount → unlisten 被调用"：mock unlisten 为 `vi.fn()`，rerender unmount 后 `expect(unlistenSpy).toHaveBeenCalled()`
   - **验证标准**：`pnpm test DropzoneApp.test` 至少 5/5 PASS（含新增 2 用例）

### MINOR

1. AC-4 ~ AC-7（DZ-02 / DZ-03 退避位置 / 12px drag region / 缩放手柄 / X 右上角）已 scope 缩水到 v1.4——可接受，但应在 progress.md / 风险登记表显式标注"DZ-02/03 deferred to v1.4"以便 task_013 UX 审查不重复诊断
2. globals.css `.dropzone-blurred` 仅一行 `opacity: 0.45`——缺 `transition: opacity var(--duration-fast) var(--ease-out-expo);` 否则切换是突变而非渐隐。input.md 隐含约束"用 --duration-fast 200ms 过渡"
3. DropzoneApp.tsx:232 `transition-opacity` 是 Tailwind 默认 150ms，不在 v1 三档（100/200/300）token 内——与 task_012 TK-03 三档收敛目标冲突。建议改为 `style={{ transition: 'opacity var(--duration-fast) var(--ease-out-expo)' }}` 或新增 utility class

## 给 Dev 的修复指引

### 修复范围约束

- **只修以上 MAJOR 1 + 2 + MINOR 2**（tooltip 条件 + 2 个单测 + dropzone-blurred 加 transition）
- **AC-4 ~ 7 不必本轮修**：已 scope 缩水到 v1.4 是 PM 隐性同意的事实
- 修复完成后 `pnpm test DropzoneApp.test` 必须 PASS；手测 macOS 主窗 / 浮窗 hover tooltip 显示
