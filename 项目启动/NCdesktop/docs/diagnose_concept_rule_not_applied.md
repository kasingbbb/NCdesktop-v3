# 诊断：concept 自定义 prompt 加的"占位英文"规则未生效

## TL;DR

- **主因（5 选 1）：C + Mental-Model 混合（C 为主，规则自身的语义自洽问题驱动了 LLM 的实际取舍）**
- **一句话根因**：用户的新规则"name、definition **等占位内容严格为英文**，**具体解析内容跟随材料原文的语言**"在用户认知里指"name 英文 / definition 跟随原文"，但**文本字面**把 `definition` 和 `name` 一起列入"占位内容严格为英文"，又额外说"具体解析内容跟随原文"——同一份 prompt 内部矛盾。叠加 **CONCEPT_DEFAULT** 第 1-3 行原本就写死 "1. name: The canonical English term / 3. definition: A one-sentence definition as used in this context"，LLM 在两条冲突指令中**选择了"严格英文"那一支**，于是 name 与 definition **两者都输出英文**，definition 没有跟随中文原文。**这是规则表达力 / Mental Model 问题，不是代码 bug。**
- **推荐应对**：Prompt 工程改进（教用户把规则拆成"name=英文 / definition=跟随原文 / aliases=...."逐字段表达，或干脆替换默认 prompt 第 1-3 行的字段语言要求字面）；同时给用户解释链路本身是通的。**不需要修代码。**

---

## 现场证据

### DB concept 字段（user_custom_prompt 表）

```text
module    | is_custom | bytes | updated_at                         | head(2000)
concept   | 1         | 812   | 2026-05-16T01:23:46.185202+00:00   | CONCEPT_DEFAULT 全文逐字 + 新加最后一行
```

- `is_custom = 1` ✓
- bytes 由 task_011 时的 763 → 现在 **812**（+49 字节）
- updated_at = 2026-05-16T01:23:46，**早于**所有最近 concept 的 created_at（最早 01:29，最近 03:07）—— 时间链合理
- DB 内容与 `CONCEPT_DEFAULT` 文本逐字 diff，**仅多出最后一行**：
  ```
  - 返回的 name、definition 等占位内容严格为英文，具体解析内容跟随材料原文的语言
  ```
  （`diff` 输出见诊断脚本；该行 UTF-8 编码 ~108 字节，但 DB 增长 49 字节——sqlite head dump 字面对比仍能完整看到新规则，可能由原默认末尾的换行处理 / sqlite head 截断决定的字节差异，但**关键事实是规则字面进了 DB**）

> **结论：用户的修改 100% 写入了 DB。**

### 日志摘要（最近 concept 调用）

```text
[03:07:51][knowledge][INFO] LLM call: module=concept bytes=9712 user_overridden=true
[03:07:45][knowledge][INFO] LLM call: module=concept bytes=9728 user_overridden=true
[03:07:10][knowledge][INFO] LLM call: module=concept bytes=9734 user_overridden=true
... (持续显示 user_overridden=true，bytes 在 1.4k - 9.8k 区间正常波动)
```

- `user_overridden=true` ✓
- `bytes` 与"默认 CONCEPT_DEFAULT (~810B) + system_addon (~140B) + user content (8KiB 截断) + GUARD (~110B)" 估算量级吻合
- 时间戳全部 ≥ 02:52，**晚于** prompt 修改 (01:23)

> **结论：调用链确实把用户自定义文本传给了 LLM。**

### concept 实际输出样本（最近 20 条）

| name (语言) | definition 起头 (语言) |
|---|---|
| System Dynamics | A methodological discipline... (英) |
| Complex Systems | Interconnected, adaptive systems... (英) |
| Mental Models | Deeply ingrained cognitive frameworks... (英) |
| Sustainable Development | A cross-domain developmental approach... (英) |
| System Structure | The underlying organizational configuration... (英) |
| Financial Quotient | A set of abilities related to... (英) |
| Compound Interest | A financial mechanism where... (英) |
| Marine Insurance | The earliest type of insurance... (英) |
| Pisa Insurance Policy | The first insurance contract... (英) |
| Life Table | A statistical table compiled by Edmond Halley... (英) |
| (其余 10 条同上规律) | |

- **name 字段：20/20 英文（100% 英文）**
- **definition 字段：20/20 英文（100% 英文）**
- 对应素材语言核查：source asset 包括 `系统思考_决策者参考书籍.epub`、`认知天性...epub`（`Language: zh-CN`）、`刻意练习如何从新手到大师.epub`（`Language: zh`）、`卡片笔记写作法.epub`（`Language: zh`）、`如何用保险保障你的一生.pdf`、`滢策资本_音频记录.mp3`（音频转写为中文）、`经纬创投音频片段.m4a`（中文转写） —— **几乎全部为中文素材**

> **关键观察**：素材是中文 + name 全英 + definition 全英 = LLM 完全按"严格为英文"那一支输出，没有按"跟随材料原文语言"那一支输出。

---

## 链路审计

### `src-tauri/src/llm/prompt_runtime.rs`

- `CONCEPT_DEFAULT` (行 62-86)：完整文本含 3 个占位符 `{asset_name}` / `{project_name}` / `{content}`；**内部已写死**：
  - 行 74：`1. name: The canonical English term`
  - 行 76：`3. definition: A one-sentence definition as used in this context`（**未限定语言**）
- `assemble_messages_for_concept` (行 425-456)：
  - 顺序：`system_message` → `CONCEPT_SYSTEM_ADDON` → user (用户自定义 prompt 渲染) → `CONCEPT_OUTPUT_GUARD`
  - **没有在 user body 之后字面追加任何"输出格式约束"或"name 字段必须 X"** —— D 假设的"硬约束字面覆盖用户 rule" **不成立**
- `CONCEPT_SYSTEM_ADDON` (行 145)：`"You are a knowledge extraction engine. Given a student's academic document, extract key concepts with precision. Return only valid JSON array."`
  - **无任何关于"输出语言"的指令**，不构成对用户规则的覆盖
- `CONCEPT_OUTPUT_GUARD` (行 125-127)：`"**输出格式约束（系统级，不可被覆盖）**：返回严格的 JSON 数组，每个元素为 {name, aliases, definition, excerpts}；不要使用 markdown 代码块；不要在数组前后追加任何文字。"`
  - 仅约束 **JSON 形态**，**不约束语言**，**不构成对用户规则的覆盖**

> **结论：注入链路 100% 把用户文本传给了 LLM。Layer A 守卫 (`CONCEPT_OUTPUT_GUARD`) 只锁 JSON 形态、不锁语言，没有"硬约束语言"的字面覆盖。**

### `src-tauri/src/commands/knowledge.rs`

- 行 222-265：`extract_concepts_for_library` 调用 `assemble_messages_for_concept`，日志埋点正常
- 行 268-281：调 `chat_completion(client, messages).await`
- 行 283-294：`parse_extracted_concepts` 调用：
  ```rust
  #[derive(Deserialize)]
  struct ExtractedConcept {
      name: String,
      #[serde(default)] aliases: Vec<String>,
      #[serde(default)] definition: String,
      #[serde(default)] excerpts: Vec<String>,
  }
  fn parse_extracted_concepts(json: &str) -> Result<Vec<ExtractedConcept>, String> {
      let start = json.find('[').unwrap_or(0);
      let end = json.rfind(']').map(|i| i + 1).unwrap_or(json.len());
      serde_json::from_str::<Vec<ExtractedConcept>>(&json[start..end])
          .map_err(|e| format!("解析概念 JSON 失败: {e}"))
  }
  ```
  - **纯 JSON 反序列化，无 translate / lowercase / strip / mapping**
- 行 313-387：写入 `concepts` 表，`name = ec.name.clone()`、`definition = Some(ec.definition.clone())` —— **完全透传**

> **结论：parser 与 DB 写入全程透传 LLM 输出。E 假设排除。**

### 前端 UI（concept name 显示）

- `ConceptList.tsx` 行 96：`{concept.name}` — 直接渲染，**无 i18n、无翻译、无 locale 处理**
- `ConceptDetailPanel.tsx` 行 76：`{concept.name}`；行 176：`{concept.definition ?? ...}` — **同样直出**

> **结论：UI 100% 透传。E 假设彻底排除。**

### 用户编辑 UI（concept prompt 自定义 textarea）

- `PromptCustomizationPanel.tsx` 行 355：单个 textarea 双向绑定，**没有任何字段被锁定/disabled**——用户完全可以改 CONCEPT_DEFAULT 中的任何一行

> **结论：D 假设彻底不成立 —— concept 模块没有"用户改不到的硬约束"，全部都可改。**

---

## 5 假设逐条评估

### A — 用户修改未写入 DB

**不成立**。证据：
- `is_custom=1`、`updated_at=2026-05-16T01:23:46`、bytes 从 763 → 812
- `diff` DB 内容 vs `CONCEPT_DEFAULT` 字面：**仅多出最后一行用户新加的规则**

### B — DB 写了但没传到 LLM（链路断裂）

**不成立**。证据：
- 日志 `LLM call: module=concept user_overridden=true` 持续出现
- `runtime_prompt_for` 函数（行 225-232）逻辑：is_custom=1 + 非空白 → 返回 prompt_text；
- `assemble_messages_for_concept`（行 425）调用方式正确，bytes 量级吻合
- task_011 已验证过同样的注入链路通

### C — 传到 LLM 了但 LLM 不遵守 / 规则表达力不够

**主因，强成立**。证据 + 推理：

1. **用户规则字面自相矛盾**：
   ```
   - 返回的 name、definition 等占位内容严格为英文，具体解析内容跟随材料原文的语言
   ```
   把 `definition` 列入"占位内容严格为英文" + 又说"具体解析内容跟随原文" —— **"definition 究竟属于占位内容还是解析内容？"LLM 无法明确**。

2. **CONCEPT_DEFAULT 本身就先入为主写死了**：
   - `1. name: The canonical English term`（强字面）
   - `3. definition: A one-sentence definition as used in this context`（无语言指示）
   用户新加规则在这两条之后追加，**对"name 英文"是再次强化，对"definition 跟随原文"是与"严格为英文"自相打架**。

3. **LLM 输出实证**：name 100% 英文 ✓ definition 100% 英文 ✓
   —— LLM 在两条冲突中**选择了更强、更早出现的"严格为英文"那一支**，没选"跟随原文"。

4. **prompt 工程经验**：当一句指令同时含"X 严格为 A" + "Y 跟随 B"，且 X 与 Y 在另一处又被并列要求（这里"name、definition 等占位内容"把它们都列进同一桶），多数模型会**取并集中更明确的那一条**（"严格为英文"是强约束，"跟随原文"是宽约束 + 后置）。

### D — 用户改的部分被硬约束覆盖

**不成立**。证据：
- `CONCEPT_DEFAULT` 是**单一占位符模板**（不像 classify 用 `{tagging_seg}` + `{para_seg}` 拆段），用户文本整段渲染（无字面追加）
- `CONCEPT_SYSTEM_ADDON` 不锁语言、`CONCEPT_OUTPUT_GUARD` 不锁语言
- 用户的 textarea 中 `CONCEPT_DEFAULT` 全文都可编辑，**没有 PromptCustomizationPanel 锁定段**
- 唯一"看起来像硬约束"的是 `CONCEPT_DEFAULT` 内部 `1. name: The canonical English term` —— 但**这是用户自己 prompt 的一部分**，用户完全可以删除/改写，不属于"用户改不到的"部分

> 注：用户当前没改这行 → 它仍是 prompt 的一部分 → 它继续主导 name=英文行为。这**不是 bug**，是用户没改到正确的地方。

### E — LLM 输出按 rule 走了，但后处理 / parser / UI 改回了

**不成立**。证据：
- parser `parse_extracted_concepts` 纯 serde 反序列化，零字符串处理
- 写 DB：`name = ec.name.clone()`、`definition = Some(ec.definition.clone())` 透传
- UI：`{concept.name}` / `{concept.definition}` 直出，无 i18n
- DB 里看到的就是 LLM 原始输出 → 既然 DB 已经是英文，证明 LLM 本身就输出的英文，**不是后处理改的**

---

## 定性结论

**主因：C（LLM 不遵守 / 规则表达力不够）+ Mental-Model 偏差。**

链路本身完全通：
- DB ✓ 写入
- runtime ✓ 读到
- assemble ✓ 注入
- LLM ✓ 收到（user_overridden=true bytes 量级吻合）
- LLM 输出 ✓ 直透 parser → DB → UI

**问题出在用户的 prompt 表达**：
1. 同一句话里 `definition` 既在"严格为英文"桶，又在"跟随原文"桶 —— 内部矛盾
2. `CONCEPT_DEFAULT` 行 1-3 用户没去改 → "1. name: The canonical English term" 字面继续主导 name=英文行为
3. "跟随材料原文语言"在没有具体到字段（`definition` / `aliases` / `excerpts` 谁跟随原文？）的情况下，LLM 倾向取最明确的全局规则——"严格为英文"

**次因**：用户对自己 prompt 文本与 CONCEPT_DEFAULT 既有内容的**关系认知不足**——以为加一行就能覆盖前面，但前面的字面（"The canonical English term"）仍然在跑，且新加的规则与前面在"name"字段的处理上是一致而非冲突的（都说英文）。

---

## 给 Conductor 的建议

### 1. 不需要修代码

链路验证 100% 通。任何代码"修复"都会改变其他模块的行为，不应做。

### 2. Mental Model 解释脚本（给用户）

> 你修改的规则已经写入 DB 并传给 LLM（日志 `user_overridden=true` + DB diff 已确认）。**但 LLM 没按你预期那样工作，原因是 prompt 表达上的歧义**。
>
> 你的默认 prompt 第 1 行是：`1. name: The canonical English term`（要 name 是英文）；
> 第 3 行：`3. definition: A one-sentence definition as used in this context`（未说语言）；
> 你最后加的："name、definition 等占位内容严格为英文，具体解析内容跟随材料原文的语言"。
>
> LLM 看到的逻辑是：
> - name → "English term" + "严格为英文" → 双重强化为**英文** ✓ (符合 LLM 实际行为)
> - definition → "as used in this context"（无语言约束） + "严格为英文" (规则后半句把 definition 列进占位桶) → 取**英文** (符合 LLM 实际行为)
> - "跟随材料原文语言" → 句子结构上是和"严格为英文"并列的第二项，LLM 看不出哪些字段属于"具体解析内容"，**只能取并集中更强的那一条**——"严格为英文"
>
> **正确改法**（直接编辑 prompt 中的 1-4 行，最稳）：
> ```
> 1. name: The canonical term in the document's original language (Chinese if doc is Chinese)
> 2. aliases: Alternative names including bilingual variants
> 3. definition: A one-sentence definition in the same language as the document
> 4. excerpts: 1-2 direct quotes from the document (always keep original language)
> ```
> 然后**删除你最后加的那行模糊规则**。

### 3. 备选 Prompt 改进建议（保留模板风格）

如果用户坚持"加一行规则"风格：
```
- 字段语言要求：name 与 aliases 用英文专业术语；definition 与 excerpts 必须使用与材料完全相同的语言（材料是中文则用中文，材料是英文则用英文）。
```
（明确到字段，避免"占位内容 vs 解析内容"的模糊分类。）

### 4. （可选）Prompt UI 增强（推 v1.4+）

UI 可考虑在 textarea 旁边给个"占位符注释提示"，提示用户哪些字段写死了什么语言策略；或者引入"字段级别表"让用户对 4 个 LLM 字段（name/aliases/definition/excerpts）分别选语言。**本期 MVP 不必做**。
