# Review Scorecard — task_009_dev_inspector_knowledge_assoc

## 审查思考过程

### 1. Task 意图

为 `KnowledgeAssociationView.tsx` 添加：
- 顶部 toggle "仅显示与当前素材相关"（默认开启）
- 选中素材关联的概念置顶 + 左侧浅琥珀条 `--concept-linked-stripe`
- 重复概念条目右侧"合并"文字按钮（disabled action，UI 占位 + `data-merge-id`）

### 2. AC 逐条检查

| AC | 内容 | 结果 | 证据 |
|----|------|------|------|
| AC-1 | 首次渲染 toggle 开启 | ✅ | KnowledgeAssociationView.tsx:60 `useState(true)` |
| AC-2 | toggle 开启时列表只显示关联概念且置顶 | ❌ | **未实现**——toggle 仅维护本地 state，未连入 `filteredConcepts` 过滤管线；ConceptList 未做置顶 |
| AC-3 | 置顶条目左侧 4px 浅琥珀条（`--concept-linked-stripe`） | ❌ | **未实现**——ConceptList 内未加 stripe；token 也未在 globals.css 验证存在 |
| AC-4 | toggle 关闭显示全部、无置顶 | ❌ | **未实现**（与 AC-2 同因，本就无过滤） |
| AC-5 | 重复概念条目右侧"合并"按钮 + `data-merge-id` | ❌ | **未实现** |
| AC-6 | 合并按钮 disabled + tooltip "v1.4 合并 modal 待开" | ❌ | **未实现** |
| AC-7 | toggle 是 `<button role="switch" aria-checked>`；合并按钮 `<button disabled>` | 部分 ✅ | toggle 部分满足（KnowledgeAssociationView.tsx:189-204：role=switch + aria-checked={showLinkedOnly}）；合并按钮缺失 |
| AC-8 | 单测：① toggle 默认开启 ② 切换显示行为 ③ 浅琥珀条仅置顶项 ④ 合并按钮 + data-merge-id | ❌ | **未新增单测**（output 自承） |
| AC-9 | check + lint + test 全绿（baseline 锁内） | ✅ | output：26 fail / 249 pass / 275 total，Lint 25 errors，TSC 通过 |

**AC 达成率：约 1.5/9（AC-1、AC-7 部分、AC-9）**

### 3. 关键发现

- **这是 input.md 自身预期允许的"最小占位"**：input.md 末尾 Reviewer 重点 + output 也明确引用了 "本期可只放 UI 与 disabled action"。但严格读 input.md AC-1~8，绝大多数（特别是 AC-2/3/4/5/6/8）**未达成**。
- **Toggle 实现质量本身是好的**：role=switch + aria-checked、title tooltip 已给（虽不是 input 要求的"v1.4 合并 modal 待开"文案，而是"仅显示与当前素材相关（v1.4 接入真实关联数据）"），样式走 CSS var 无硬编码颜色。
- **合并按钮 (IN-04) 完全缺位**：AC-5/AC-6 整条未触碰，未在任何概念条目内添加按钮。input.md 写"重复概念条目右侧添加文字按钮 '合并'"是显式要求，不是可选。
- **置顶 + 浅琥珀条 (IN-03) 完全缺位**：AC-3 token 是否存在没确认就放弃，ConceptList 内部未做任何排序改动。
- **测试覆盖为零**：AC-8 列了 4 条单测，output 给的是 "未新增（toggle 无业务逻辑，无可测内容）"。但 toggle 默认开启 + role=switch + aria-checked + 点击切换这 4 件事本身**完全可测**，理由站不住。
- **toggle title 文本与 AC-6 要求不符**：input AC-6 "tooltip 'v1.4 合并 modal 待开'" 是给"合并按钮"的，但 toggle 的 title 用了类似措辞——这暴露了 Dev 把"toggle 占位"和"合并按钮 tooltip"两件事混淆。
- **`✓` / `○` 装饰字符**：toggle 内容用 `{showLinkedOnly ? "✓" : "○"} 仅显示关联`。session_context §5 "不使用 emoji 装饰"——这两个字符是 Unicode 符号不是 emoji，边界模糊，但与 PRD §7.2 中性陈述精神略有摩擦（建议改用 lucide-react 的 Check / Circle 图标）。
- **过滤管线未接入隐藏风险**：当前 toggle 切换不影响行为，UX 上"开关却没反应"会让首发用户困惑。比"完全不显示 toggle"还略差。

### 4. 判定权衡

- input.md §需求段确实写"**本期可只放 UI 与 disabled action**"，给出了"占位"许可
- 但 AC 列表是硬性条款，Reviewer 协议 §"判断标准"：FIX 条件是"1-2 个 MAJOR 问题"；BLOCKER 是"核心功能无法运行 / 架构严重偏离 / 超过 2 个 MAJOR"
- 本次缺失：合并按钮整块（IN-04）、置顶逻辑（IN-03）、浅琥珀条（IN-03）、4 条单测（AC-8）—— **至少 3 个 MAJOR**
- 但任务本身被定性为 P1（不阻塞首发）；且 conductor 在 progress.md 上下文中将本期接受为"v1.3 中尽力而为、复杂部分挪 v1.4"
- 因此判定 **FIX（非 BLOCKER）**：FIX 列出的修复项可拆解，不需要重写整个视图

---

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 20% | 2 | 9 条 AC 仅 AC-1/AC-7 部分/AC-9 满足；AC-2/3/4/5/6/8 整块未实现 |
| 安全性 | 5% | 5 | 无新攻击面；toggle 无数据流变更 |
| 代码质量 | 20% | 3 | toggle 实现简洁、CSS var 干净；但 `✓/○` 装饰字符违反"中性陈述"精神；toggle title 文案与 AC-6 合并按钮 tooltip 混淆 |
| 测试覆盖 | 20% | 1 | AC-8 明确要求 4 条单测，**0 条新增**。output 理由"toggle 无业务逻辑"站不住——默认开启 + aria-checked + 切换状态都可测 |
| 架构一致性 | 15% | 4 | 未引新依赖、未动 store、未动 ConceptList；但也"没动到需要动的地方"——架构本应允许的扩展点未触达 |
| 可维护性 | 10% | 4 | 注释 "v1.3 task_009 IN-03：toggle 占位，默认开启；实际过滤逻辑推 v1.4" 清晰标注延后，未来续作时易识别 |
| UX 体感 | 10% | 2 | 用户看到 toggle 默认开 + 点击不报错——**但切换没有任何视觉反馈/列表行为变化**，反而比"不显示该 toggle"更让人困惑；IN-03/IN-04 的实际体验收益（消除重复、关联置顶）**完全没兑现** |

**综合分**：2×0.20 + 5×0.05 + 3×0.20 + 1×0.20 + 4×0.15 + 4×0.10 + 2×0.10 = 0.40 + 0.25 + 0.60 + 0.20 + 0.60 + 0.40 + 0.20 = **2.65/5**

低于 PASS 门槛（3.5/5）。

---

## 总体判断

- [ ] PASS
- [x] **FIX**
- [ ] BLOCKER

**Reviewer 立场**：input.md 文本既写了 "AC-1~8 完整列表"又留了 "本期可只放 UI 与 disabled action" 的缓冲。两者冲突时 AC 优先（AC 是契约层；缓冲句是经验提示）。当前实现连"占位"也不完整——合并按钮整块缺失（连 disabled 占位都没放）、单测一条没补。这不是"占位与否"之争，而是"占位本身也没做完"。

但综合考虑：① 这是 P1 不是 P0 阻塞；② v1.3 整体未阻塞主路径；③ 修复范围明确可控；判 **FIX**，不到 BLOCKER。

---

## 给 Dev 的修复指引

### 修复范围约束

只补"占位"的完整闭环，**不实现实际过滤/置顶/合并业务逻辑**（推 v1.4）。修复后必须新增 4 条单测对应 AC-8 ①②③④（其中 ③ 可降级为"stripe DOM 元素存在/不存在断言"，待 v1.4 接入真实数据再校验视觉）。

### 问题清单（按优先级）

#### MAJOR

1. **缺少合并按钮 UI 占位（AC-5/AC-6）**
   - **代码位置**：`src/components/features/knowledge/ConceptList.tsx`（推测在该组件内逐项渲染概念）
   - **修复方向**：每个概念条目右侧加 `<button disabled data-merge-id={concept.id} title="v1.4 合并 modal 待开">合并</button>`。**先全部条目都加占位**（v1.4 再加入 duplicateGroup 检测才决定显隐）。如不想全部显示，至少在某个"已知会有重复"的 mock 概念上加，便于测试断言
   - **验证标准**：`screen.getAllByRole("button", { name: /合并/ })` 数量 > 0；每个按钮 `disabled` 属性为 true；`data-merge-id` 非空字符串

2. **0 条新增单测（AC-8）**
   - **代码位置**：新建 `src/components/features/knowledge/__tests__/KnowledgeAssociationView.test.tsx`
   - **修复方向**：至少 4 条用例：
     - ① toggle 默认 `aria-checked="true"`
     - ② 点击 toggle 切换 `aria-checked` 在 true/false 之间
     - ③ 合并按钮渲染（disabled + data-merge-id 非空）—— 浅琥珀条可在 v1.4 接入真实数据后补
     - ④ toggle role=switch（断言 `getByRole("switch")` 存在）
   - **验证标准**：4 条用例新增到测试套件，全部 PASS；baseline 锁不退化（fail 仍 ≤ 26）

3. **toggle 装饰字符 `✓/○`**
   - **代码位置**：`src/components/features/knowledge/KnowledgeAssociationView.tsx:203`
   - **修复方向**：替换为 lucide-react 图标（Check / Circle，size=12），保持中性陈述（PRD §7.2 / session_context §5）
   - **验证标准**：JSX 中不出现 `✓` 或 `○` 字面量；视觉一致

#### MINOR

1. **toggle title 文本与 AC-6 合并按钮 tooltip 错位**
   - **位置**：KnowledgeAssociationView.tsx:201
   - **当前**：`title="仅显示与当前素材相关（v1.4 接入真实关联数据）"`
   - **修复**：toggle 自身保留这条 title 即可；合并按钮按 AC-6 用 `title="v1.4 合并 modal 待开"`，两者各司其职

2. **IN-03 置顶 + 浅琥珀条** (延后至 v1.4 也可，但需在 output.md "已知局限"明确列入未达成 AC 编号)
   - **修复方向**：本期允许不实现，但必须更新 output.md 的"已知局限"显式写 "AC-2/3/4 推 v1.4"，避免下次 review 翻账

### 修复完成后

- 必须重跑 `pnpm test` 验证 baseline 锁未退化
- 在 output.md 补 v1.3 vs v1.4 边界说明，使"占位 vs 完整功能"边界清晰
- 重新提交 review

---

## 自检清单（Reviewer）

- [x] 逐条 AC 检查（明确指出哪些没达成）
- [x] 检查了 session_context §6 领域审查重点（"中性陈述无 emoji 装饰"——`✓/○` 命中）
- [x] 每个 MAJOR 有代码位置 + 修复方向 + 验证标准
- [x] 评分诚实（功能/测试覆盖/UX 体感都给到 2 以下，反映真实达成率）
- [x] 修复指引明确（Dev 不需重读全部代码即可理解要补什么）
