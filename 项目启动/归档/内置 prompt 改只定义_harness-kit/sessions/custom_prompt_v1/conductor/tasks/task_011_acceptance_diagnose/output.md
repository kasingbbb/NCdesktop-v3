# Acceptance Diagnose Report

> 诊断角色:Conductor 派出的诊断专用 subagent(零代码改动)
> 时间:2026-05-15
> 范围:custom_prompt_v1 流水线 PM 手动验收阶段两个 bug 的根因诊断

---

## 现场勘察

### DB 路径与表内容

- DB 路径:`~/Library/Application Support/com.notecapt.desktop/notecapt.db` (~85 MB,含 WAL)
- 目标表:`user_custom_prompt`(V15 schema,主键 `module`,字段与 ADR-002 一致)

`user_custom_prompt` 当前内容(`SELECT module, is_custom, length(prompt_text), substr(prompt_text, 1, 200), updated_at`):

| module  | is_custom | bytes | updated_at                       | preview |
|---------|-----------|-------|----------------------------------|---------|
| concept | 1         | 763   | 2026-05-15T13:06:30.388227+00:00 | `# Document Analysis Request\n\n## Document\nTitle: {asset_name}\nProject/Course: {project_name}\nContent:\n---\n{content}\n---\n\n## Task\nExtract all significant academic concepts from this document. For each c` |
| para    | 1         | 160   | 2026-05-15T15:16:08.976814+00:00 | `一、核心路由（PARA Router）——自上而下穿透，直到唯一物理定位：\n【P】1-项目：服务于有明确目标与截止期的短期活动？\n【A】2-领域：无明确终点、但需长期维持标准的责任领域？\n【R】3-资源：暂无任务、但有潜在利用价值的课题/兴趣？\n【A】4-存档：已完结、取消或无限期搁置？\n【A】5-电子书：电子书资源？` |

**关键发现**:
- concept 自定义文本与内置 `CONCEPT_DEFAULT` 完全一致(用户改了又改回,或仅复制了默认模板;`is_custom=1` 但语义等同未改)
- **para 自定义文本仅在末尾追加了一行 `【A】5-电子书：电子书资源？`** —— 这是用户对"电子书"分类期望的产物
- tagging / aggregation 无 row,走默认 fallback

### 应用日志摘要

- 日志路径:`~/Library/Logs/com.notecapt.desktop/NoteCapt.log`(26 KB)
- task_004 埋点命中**仅 1 条**:
  ```
  [2026-05-15][15:16:28][app_lib::commands::llm][INFO] LLM call: module=classify bytes=4034 user_overridden=true
  ```
  说明:用户自定义 prompt 注入路径**真实生效**(`assemble_messages_for_classify` 读到了 `is_custom=1`)
- 拖入流程:`[15:16:28] 拖入工作区目录: ...18523c9f.../`,处理对象为 **《刻意练习》一本 .epub**;`[15:16:31]` 物化 MD v1 完成;`[15:16:44]` "拖放 AI 后台分类完成"
- **日志中无任何 reset/save 操作的错误痕迹**,DB 写命令均成功

### 物理工作区现场

`~/Downloads/NoteCaptWorkPlace/18523c9f-271d-4a33-ac26-e6d9a8ff9865/organized/`:
- 只存在 `3-资源/` 与 `1-项目/` 等 4 个 PARA 内置目录
- **没有 `5-电子书/` 或类似自定义分类目录**
- 用户本次拖入的《刻意练习》.epub(asset_id `2f5b02db...`)被分到 `organized/3-资源/`

`ai_analyses` 中本次拖入对应的行:
- `topics=3-资源` `suggested_name=学习方法参考_刻意练习如何从新手到大师.epub`
- 历史所有 .epub 均归到 `3-资源`(认知提升读物/蒙台梭利等)

---

## BUG-1 诊断:恢复默认无效

### 现象复述

PM 报告"恢复默认功能没有生效"(具体路径未明:单条恢复 / 全部恢复 / UI / DB 哪一环回退失败)。

### 代码路径审计

**完整路径已逐文件 Read 比对**:

| 层级 | 文件 | 关键函数 | 结论 |
|------|------|---------|------|
| 前端 store | `src/stores/userPromptStore.ts:162-183` | `reset(module \| null)` | `module=null` → `resetUserPrompt(null)` + `loadAll()` 重建 4 项;`module=X` → `resetUserPrompt(X)` + `getUserPrompt(X)` 拉新 + 同步 `drafts/dirty` |
| 前端契约 | `src/lib/tauri-commands.ts:837-839` | `resetUserPrompt(module \| null)` | `invoke("reset_user_prompt", { module })` —— 对 `null` Tauri 序列化为 `Option::None` |
| 前端 UI | `src/components/settings/PromptCustomizationPanel.tsx:168-178` / `:93-105` | 单条 onReset / handleResetAll | 双重 `window.confirm` 二次确认,`reset(module/null)` 调用前 `useUserPromptStore.setState({ error: null })` |
| 后端 cmd | `src-tauri/src/commands/user_prompt.rs:160-176` | `reset_user_prompt` | `ensure_writable` 守卫 + `Option<String>` 分发:None→`delete_all`,Some→`validate_module`+`delete` |
| 后端 DB | `src-tauri/src/db/user_prompt.rs:79-93` | `delete` / `delete_all` | 单参 SQL DELETE,无 trigger 拦截;返回 `Result<(), String>` |
| AppMode | `src-tauri/src/lib.rs:61` | `app.manage(AppMode::Normal)` | 写命令必经的 `ensure_writable` 守卫已注册 Normal,生产路径放行 |

**测试覆盖度极高**:
- `user_prompt_e2e.rs::e2e_reset_single_module_only_affects_that_module` / `e2e_reset_all_clears_all_four_modules` —— 后端真实 DB 路径 GREEN
- `userPromptStore.test.ts::AC-5 reset(null)` / `AC-5 reset(module)` —— store 三表同步 GREEN
- `PromptCustomizationPanel.test.tsx::AC-5 ⑥/⑦` —— UI 单条+全部 reset 调用、confirm 拒绝、按钮 disabled 条件 GREEN
- `user-prompt.contract.test.ts::resetUserPrompt(null)` / `('tagging')` —— IPC 参数包 `{ module: null }` / `{ module: "tagging" }` GREEN

### 定性结论

**Mental Model 偏差 / 误报(高置信)** ;无代码 bug。

四个最可能的真实情景(按可能性排序):

1. **PM 在 confirm 弹窗中点了"取消"**(单条/全部 reset 均有 `window.confirm` 二次确认)。取消后**前端不调 IPC**,因此 DB 中 concept + para 仍保留自定义记录,UI 也不刷新 ——"看起来恢复默认没生效"
2. **PM 误判 reset 按钮的可点击状态**:`resetDisabled = !isCustom`;当 module 当前 `isCustom=false`(从未保存过 / 已 reset 完成)时按钮 disabled。PM 在测试 tagging / aggregation(从未自定义)的 reset 按钮发现"点不动"
3. **PM 期望 reset 后某种 toast / 显式"操作成功"反馈**;但当前 UX 仅靠 textarea 内容回退 + 状态指示从"已自定义"→"默认"反应。如果 PM 当时折叠状态没展开,看不到 textarea 变化,会以为"无反应"
4. **DB 时间戳证据**:`para.updated_at=2026-05-15T15:16:08` 与日志 `LLM call @ 15:16:28` 仅差 20 秒。可能 PM 报 BUG-1 时**根本还没点过 reset**(测试顺序是"保存→拖文件→看 LLM 行为"而不是"保存→reset→看 reset 是否生效")

**支持"无代码 bug"的硬证据**:
- DB 中 concept 自定义文本与 `CONCEPT_DEFAULT` **完全字面等价**(763 字节 + 内容核对) —— 说明用户曾经"保存=没改"或者点过 reset 又点了 save 内置文本。任何一种都不是 reset 失败的标志
- 日志中**无任何**"reset_user_prompt 失败"或 DB 写错误
- 后端守卫 `ensure_writable` 在 `lib.rs` 已注册 `AppMode::Normal`,与 save 路径同一条链路 —— **如果 reset 真的被阻断,save 必然也阻断**;但 save 已成功完成两次(concept + para 各 1 次),反证 reset 路径若被触发必然也成功

### 修复建议

**不修代码**;建议两件低成本的 UX 增强(优先级低,可放到下个迭代):

1. **PromptCustomizationPanel.tsx**:在单条 reset / 全部 reset 完成后追加一个轻量 toast 或非阻塞 banner(类似"已恢复『PARA 分组』为默认值"),给 PM 显式反馈
2. **reset 按钮 disabled 时显示 tooltip 解释"无可恢复的自定义"**,与保存按钮已有的 `saveDisabledReason` 模式对齐

如果需要 conduit 进一步排查,建议 PM 提供:
- "恢复默认无效"的精确步骤序列(单条还是全部?哪个 module?)
- confirm 弹窗是否完整完成
- 操作后是否切了 Tab / 重启过应用(loadAll 应在每次进 Prompt Tab 时跑)

---

## BUG-2 诊断:自定义分类未生成新文件夹

### 现象复述

PM 在 PARA 分组的自定义 prompt 末尾追加 `【A】5-电子书：电子书资源？`,期望导入 .epub 后生成 `organized/5-电子书/` 文件夹。实际:`organized/3-资源/` 收纳,无新文件夹。

### LLM 调用链审计

| 阶段 | 文件:行 | 关键文字 |
|------|--------|---------|
| ① 用户保存 para | `db/user_prompt.rs:61-76` upsert | DB 写入成功,`is_custom=1` |
| ② 拖入触发 | `commands/dropzone.rs:444-479` `spawn_dropzone_ai_job` | 后台 spawn 调 `apply_llm_classify_to_asset` |
| ③ 调 LLM | `commands/llm.rs:102-133` `llm_classify_with_db` | `assemble_messages_for_classify(conn, ClassifyVars)` + 埋点 `LLM call: module=classify ...` |
| ④ 组装 | `llm/prompt_runtime.rs:386-415` | `runtime_prompt_for(conn, "para")` 读用户文本 → 注入 `classify_prompt_v2(content, tagging_seg, para_seg)` |
| ⑤ user body | `llm/prompts.rs:53-101` `classify_prompt_v2` | **用户的 para_seg 注入到「一、核心路由」位置**,但同一 user message 后续仍包含: |
|  | `prompts.rs:74-77`(写死字面) | `「1）category（主类别，字符串，**必须且仅能**取下列之一）：『1-项目』『2-领域』『3-资源』『4-存档』，仅当完全无法做 PARA 判定时才用 『other』」` |
| ⑥ system 压底 | `prompt_runtime.rs:119-122` `CLASSIFY_OUTPUT_GUARD` | 「JSON 必须包含字段:category、tags...」 ADR-003 Layer A 永远 system 最后压底 |
| ⑦ LLM 输出 | `ai_analyses.topics` | 实际返回 `3-资源` ——**LLM 完全没采纳"5-电子书"**,严格遵守了 user message 内的 "必须且仅能" 硬约束 |
| ⑧ parser | `llm/classify_parse.rs:8-21` `ClassifyResult.category` | `#[serde(default)]` 不限值;接受任意字符串 |
| ⑨ 文件夹生成 | `commands/dropzone.rs:195-275` `organize_asset_file_after_classify` | `category_slug = sanitize_path_segment(r.category)` → `organized/<category_slug>/`;`other`/`none`/空串跳过,**其余任意字符串都会创建目录** |

### PARA parser 与文件夹生成逻辑审计

**关键发现 1**:`sanitize_path_segment`(`dropzone.rs:151-162`)只保留 `is_alphanumeric()` + `'-' | '_'`,Unicode 中文字符是 `is_alphanumeric()=true`,所以 `5-电子书` 这种合法字符串**完全可以**生成目录。

**关键发现 2**:即便用户在 `para` 自定义里加了"5-电子书"分类项,LLM 调用时,user body 在用户 para_seg 后**仍然**会被字面追加:
```
1）category（主类别，字符串，**必须且仅能**取下列之一，用于磁盘 `organized/<category>/`）：
   - `1-项目` `2-领域` `3-资源` `4-存档`
   - 仅当完全无法做 PARA 判定时才用 `other`...
```
这条字面写死在 `classify_prompt_v2`(`prompts.rs:74-77`),**不在用户可自定义的 `para` module 范围内**。

**关键发现 3**:`para` 用户自定义对应的占位符是 `{para_seg}`(`prompts.rs:62`,仅替换"一、核心路由(PARA Router)"那 5 行)。用户**改不到**第"四、与本系统字段的对应关系"段(即 category 白名单)。

**关键发现 4**:运行时验证 —— `ai_analyses` 表中本次《刻意练习》.epub 行 `topics=3-资源`、suggested_name = "学习方法参考_刻意练习如何从新手到大师.epub" —— LLM 没采纳"电子书"分类项,而是按硬约束归到 `3-资源`。

### 定性结论

**Mental Model 偏差(高置信)** ;无代码 bug。

用户对"PARA 分组 Prompt 自定义"的心理模型 ≠ 实际系统设计:
- **用户心智**: 修改 PARA Prompt = 整体修改"分类系统",包括 category 取值范围,LLM 会按我加的"5-电子书"创建新文件夹
- **实际系统**:`para` 自定义只覆盖 `{para_seg}` 一段(PARA Router 判定问题文本);**category 字段的 4-类白名单**是 `classify_prompt_v2` 函数体内的**系统硬约束**,且系统压底 `CLASSIFY_OUTPUT_GUARD` 中也明确"JSON 必须包含 category" —— 系统设计上**PARA 4 类是闭合枚举**

支持"Mental Model 偏差非代码 bug"的硬证据:
- 用户保存的 para 文本字节数 160 —— 仅最后一行追加 `【A】5-电子书：电子书资源？`,前 4 行与 `PARA_DEFAULT` 完全一致(逐字符核对 5 行内容)
- 日志埋点 `user_overridden=true` —— 自定义注入路径生效
- LLM 仍输出 `3-资源`(在 4 类白名单内) —— 严格遵守了 `classify_prompt_v2` 内的硬约束
- task_001 Architect output § 4.2 ADR-003 Layer A 明确写过:"输出格式硬守卫永远 system 压底,用户自定义不能绕过"
- task_001 Architect output § 2.1 R4 已识别:"classify 是 tagging+para 合并到同一次调用",但**未提及"PARA 类目能扩展"**
- PRD 中没有任何文字承诺"用户可在 PARA 中新增类目"

### 应对建议(无代码 bug,给用户解释或产品文档增强)

**短期(无代码修改,推荐立即落地)**:

给用户的解释脚本(可贴到 PRD § 3.2 "已知约束"或设置面板 "PARA 分组" 折叠头副标题里):
> 「PARA 分组」自定义仅影响 **路由判定文本**(LLM 对"项目/领域/资源/存档"如何判定的思考引导);
> category 字段的取值范围是 PARA 系统的核心契约(`1-项目` / `2-领域` / `3-资源` / `4-存档` / `other`),不可被用户 prompt 覆盖;
> 若需要"电子书"这种分类,推荐通过 **tags** 实现(在「文件打标签」自定义中加入"电子书"标签规则),系统会按 tag 维度二次组织,而非新建顶层目录。

**中期(需求级别,与 PM 对齐再做)**:

如果产品确认"用户应能扩展 PARA 类目":这是**真实的产品需求扩展**,涉及多处改动(`classify_prompt_v2` 硬约束、`sanitize_path_segment` 白名单、`organize_asset_file_after_classify` 创建目录策略、UI 一个"自定义类目"配置入口、迁移既有 organized 目录的迁移脚本),应作为单独 PRD 立项,**不应**纳入 custom_prompt_v1 修补范围。

**UI 文案改进(可考虑放进下个 round)**:

`PromptCustomizationPanel.tsx::PROMPT_MODULE_SUBTITLES.para` 当前是:
> "与「文件打标签」共用同一次分类调用,两者同时生效"

建议追加一句:
> "提示:PARA 类目固定为 4 类(项目/领域/资源/存档),自定义文本仅影响 LLM 的路由判定思路。"

---

## 给 Conductor 的整体建议

### BUG-1

- **范围**:零代码修改(误报 / Mental Model)
- **是否进入 task_012**:不进入 FIX,但可派一个**低优先级**任务到下一个 round:
  - "PromptCustomizationPanel: reset 完成后给一个非阻塞 toast,与按钮 disabled 时的 tooltip 提示对齐" (~10 行 React 代码)
- **回归测试加项(可选)**:在 `PromptCustomizationPanel.test.tsx` 追加一个用例验证 "reset 成功后 textarea 重置为 defaultText" —— 当前用例只验了 `reset` 被调用而未验视觉反馈

### BUG-2

- **范围**:零代码修改(Mental Model)
- **是否进入 task_012**:**不应**作为 task_012 FIX 处理
- **建议转为**:
  1. **PRD 增项**:在 `custom_prompt_prd_v1.md § 3.2 "PARA 分组"行**显式声明"PARA 4 类固定不可扩展"** —— 1 行文字工作
  2. **UI 文案增强**:`PROMPT_MODULE_SUBTITLES.para` 追加约束说明 —— 1 行代码工作
  3. **(可选)产品需求新立项**:若 PM 确实希望支持"扩展类目",立一个新的产品概念探索 charter,与 custom_prompt_v1 独立

### 回归测试加项

- 在 `user_prompt_e2e.rs` 追加一个 R4 用户预期偏差用例:
  ```rust
  // 测试用户的 para 自定义不能覆盖 classify_prompt_v2 中的 category 硬约束
  fn e2e_user_para_custom_cannot_override_category_whitelist() { ... }
  ```
  即便用户自定义 para_seg 中提到"5-电子书",最终 user body 中**仍然**含 `「必须且仅能」取 1-项目/2-领域/3-资源/4-存档`。这能在文档+代码层双重锁定这条不变量。

### 优先级建议

| 项 | 优先级 | 工作量 | 备注 |
|----|--------|--------|------|
| PRD 补一条 PARA 不可扩展约束 | P0 | 5 分钟 | 解决用户预期偏差最关键的一步 |
| PromptCustomizationPanel para 副标题补一行 | P1 | 10 分钟 | UI 层提前阻断 mental model 偏差 |
| reset toast UX 增强 | P2 | 30 分钟 | 防 BUG-1 再次误报 |
| e2e R4 不变量测试 | P2 | 30 分钟 | 锁定 ADR-003 守卫 |
| 产品需求"PARA 可扩展"立项 | P3 | 单独 charter | 仅在 PM 确认需要时 |

---

## 一句话总结

两个 BUG 均**无代码层 bug,均是 Mental Model 偏差**:BUG-1 表象是 reset 无反馈/PM 测试顺序问题,BUG-2 表象是用户期望 PARA Prompt 自定义能扩展 category 类目但 ADR-003 / `classify_prompt_v2` 已锁定 4 类闭合枚举。Conductor 应将后续工作收敛到 PRD / UI 文案增强,而非 task_012 FIX。
