# Task 输入 — task_004_dev_rust_co_occurrence

## 目标

在 Rust 侧实现概念共现关系计算逻辑（`co_occurrence.rs` 模块）和 `knowledge_compute_co_occurrence` Tauri Command，在概念提取完成后异步计算所有概念对的共现关系并写入 `concept_relations` 表，无需 LLM 调用。

---

## 前置条件

- 依赖 task：**task_002_dev_db_migration**（`concept_relations` 表已在 V4 migration 中创建完毕）
- 必须先存在的文件/接口（均已确认存在）：
  - `src-tauri/src/commands/knowledge.rs` — 包含已有概念提取相关 Command（搜索 `extract` 关键词定位）
  - `src-tauri/src/db/knowledge.rs` — 包含 `Concept` 结构体，`source_asset_ids: Vec<String>` 字段已确认是 JSON 字符串数组存储
  - `src-tauri/src/db/mod.rs` — 声明所有 db 模块

> **Host 架构校正**：Architect 规划的 `src/knowledge/mod.rs` 不存在。实际项目使用扁平模块结构。新代码按以下约定放置：
> - 共现计算逻辑 → `src-tauri/src/db/co_occurrence.rs`（新建，在 db/ 目录下，与其他 db 模块并列）
> - `knowledge_compute_co_occurrence` Command → 追加到 `src-tauri/src/commands/knowledge_understanding.rs`（task_003 将创建此文件，但 task_004 可与 task_003 并行，因此也可独立创建 co_occurrence command，或放入 knowledge.rs 尾部）

---

## 验收标准（Acceptance Criteria）

1. **AC-1**：`src-tauri/src/db/co_occurrence.rs` 存在，包含：
   - `pub fn compute_co_occurrence(conn: &Connection, concept_ids: &[String]) -> Result<usize, KnowledgeError>`
   - 函数对传入的概念 ID 列表两两配对，查询各自的 source_asset_ids，若有交集则写入 `concept_relations` 表（relation_type = "co_occurrence"）
   - 返回新增关系记录数

2. **AC-2**：`knowledge_compute_co_occurrence` Tauri Command 存在于 `commands.rs` 或 `co_occurrence.rs` 中并已注册到 `invoke_handler`。

3. **AC-3**：共现关系写入遵循"字符串排序，小的在 concept_a_id"原则（`concept_a_id < concept_b_id`），确保两个方向的共现只写一条记录。

4. **AC-4**：使用 `concept_relations` 表的 `UNIQUE INDEX idx_concept_relations_pair`——即若两个概念已有 co_occurrence 关系，再次计算时执行 `INSERT OR IGNORE` 或 `ON CONFLICT DO UPDATE SET co_occurrence_count = co_occurrence_count + 1`（更新计数，不重复插入）。

5. **AC-5**：函数正确处理空输入（`concept_ids` 为空时返回 0，不报错）。

6. **AC-6**：在已有的概念提取 Command 完成后，调用（直接调用或通过 Tauri Command invoke 的方式）`knowledge_compute_co_occurrence`；Dev 需要定位已有提取逻辑并在合适位置追加调用（作为异步后处理步骤，不阻塞提取完成的响应返回）。

7. **AC-7**：性能验证：对 50 个概念（约 1225 对）的共现计算，完成时间 ≤ 5s（可通过 `eprintln!` 计时日志验证）。

---

## 技术约束

- **不调用 LLM**：整个 co_occurrence 模块只做 SQLite 查询，严禁引入任何网络调用
- **source_asset_ids 的存储格式**：概念的 source_asset_ids 在 `concepts` 表中的实际存储格式需要 Dev 先阅读已有代码确认（可能是 JSON 字符串数组，也可能是关联表）；共现判断逻辑基于"两个概念的 source_asset_ids 是否有交集"
- **关系方向约束**：共现关系无向，存储时统一 `concept_a_id < concept_b_id`（字典序），避免重复
- **事务处理**：批量写入 `concept_relations` 时使用 SQLite 事务包裹，提高性能
- **不修改已有表**：计算过程中只读 `concepts` 及相关已有表，只写 `concept_relations`（新表）
- **错误处理**：SQLite 错误通过 `KnowledgeError::Database` 返回；不 panic

---

## 参考文件

- **技术方案（共现计算逻辑）**：`sessions/知识概念迭代为知识/conductor/tasks/task_001_architect/output.md` — ADR-006 章节，以及「API 设计 Command 4」章节
- **PRD 功能 5（共现规范）**：`sessions/知识概念迭代为知识/prd/knowledge_evolution_prd_v1.md` — 「功能 5：概念关系网络（共现版）」章节
- **数据模型（concept_relations Schema）**：技术方案文档「数据模型 - 新增表 4」章节
- **已有概念提取 Command**：需在 `src-tauri/src/` 中定位（搜索 `concept_extract` 或类似 Command 名），找到在提取完成后追加调用的位置

---

## 预估影响范围

**新建文件**：
- `src-tauri/src/db/co_occurrence.rs`（约 100-150 行）

**修改文件**：
- `src-tauri/src/db/mod.rs`：追加 `pub mod co_occurrence;`
- `src-tauri/src/commands/knowledge.rs`（末尾追加 `knowledge_compute_co_occurrence` Command，约 20 行）
- `src-tauri/src/lib.rs`（或 `main.rs`）：注册 `knowledge_compute_co_occurrence` Command
- 已有概念提取 Command 文件（待 Dev 定位）：在提取完成后追加对共现计算的异步调用
