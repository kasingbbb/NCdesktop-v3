# Task 输入 — task_012_ux_review_knowledge

## 目标（Review-only，不写代码）
审计知识抽取/搜索/全文索引模块的实际数据源：是否在消费 canonical markdown 衍生件，还是直接读 `extracted_content.structured_md` 或各格式 raw_text。输出审计报告并标注是否需要后续单独迭代。

## 前置条件
- 依赖 task：task_002~011 全部完成（确保 canonical .md 在系统里是稳定第一公民）

## 验收标准（AC）
1. **AC-1**：审计报告 `task_012_ux_review_knowledge/output.md` 必须包含：
   - 知识抽取入口函数列表（含调用路径）
   - 搜索全文模块的数据源（FTS 索引建在哪个表？基于 raw_text 还是 structured_md 还是文件内容？）
   - 三种场景的实际数据流图：标签筛选 / 全文搜索 / 知识抽取
2. **AC-2**：明确以下五个问题的结论：
   - Q1：知识抽取读的是 `extracted_content.structured_md` 还是 canonical .md 文件内容？
   - Q2：搜索是否会对同一 root 返回原件 + 衍生件两条结果？
   - Q3：`notecapt/concept-extract-requested` 事件前端是否真的监听并触发？
   - Q4：如果 canonical .md 内容更新（task_008 的幂等覆盖），知识抽取是否能感知？
   - Q5：placeholder 状态的 .md 是否被知识抽取误消费？
3. **AC-3**：每个问题给出明确判断：`✅ 已对齐 | ⚠️ 局部对齐需小改 | ❌ 未对齐需独立迭代`。
4. **AC-4**：若有 ❌ 项，在报告末尾给出"建议下一轮迭代"骨架（不展开实现）。

## 技术约束
- 本 task 不允许写代码；只读文件 + 输出 markdown 报告。
- 报告必须引用具体文件:行号，禁止泛泛而谈。

## 参考文件
- `src-tauri/src/db/knowledge.rs`、`db/knowledge_understanding.rs`、`db/knowledge_units.rs`、`db/concepts_extraction_log.rs`
- `src-tauri/src/commands/knowledge*.rs`、`commands/search.rs`
- `src-tauri/src/db/search.rs`
- `src/components/features/knowledge/**`
- 上次 commit 184c6c0（知识进化系统）相关变更
- 架构方案 §九 R7

## 预估影响范围
- 新建文件：
  - `sessions/markitdown_rescue/conductor/tasks/task_012_ux_review_knowledge/output.md`（审计报告）
- 修改文件：无
