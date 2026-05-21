# Review Scorecard — task_003_dev_db_asset_funcs

## 判定：**PASS**

综合分：**4.75 / 5**

## 接收方检查（handoff §3）

- [x] 测试结果非空：4/4 PASS
- [x] 自测矩阵正常路径全部 PASS
- [x] 架构遵守声明已填且偏离说明明示"无"
- [x] M-1 跨 task 调研动作已执行并报告

## AC 逐条核验

| AC | 状态 | 证据 |
|----|------|------|
| AC-1 `find_markdown_derivative` SQL/返回类型 | ✅ | `asset.rs:103-117` — SQL 含 `source_asset_id = ?1 AND asset_type = 'markdown' ORDER BY imported_at DESC LIMIT 1`；`.optional()` 保证 Ok(None) |
| AC-2 `update_markdown_derivative` 仅 3 列 | ✅ | `asset.rs:122-135` — `SET name=?2, file_size=?3, imported_at=?4`，签名完全匹配 input.md |
| AC-3 `set_derivative_version` 参数化 | ✅ | `asset.rs:140-151` — `params![asset_id, new_version]` |
| AC-4 内存库 + migration + ≥4 测试 | ✅ | `setup_conn()` 走 `Connection::open_in_memory()` + `run_migrations`；4 个 `#[test]` 含 "Ok(None)" 边界 |
| AC-5 cargo check 0 error | ✅ | output.md 报告 0 error / 4 pre-existing warning（llm/chat.rs） |

## 6 维评分（session_context 权重）

| 维度 | 权重 | 分数 | 说明 |
|------|------|------|------|
| 功能正确性 | 30% | 5/5 | 3 fn 行为与 AC 完全对齐，SQL 语义正确，Ok(None) 边界与列序隔离都落地 |
| 架构一致性 | 20% | 5/5 | 复用 `ASSET_SELECT` + `row_to_asset`，与 `get_by_id` 同款，不写第二份列序 |
| 可维护性 | 15% | 5/5 | 文档注释指出"为何不动 file_path/derivative_version"、引用 ADR-006 与 session_context |
| 安全性 | 10% | 5/5 | 全部 `params![]` 参数化；无 SQL 注入面 |
| 测试质量 | 15% | 4/5 | 正常 + 噪声 + 不命中类型 + Ok(None) 双路径覆盖良好；未覆盖 `update_markdown_derivative` 对不存在 id 的静默行为（已在 known limitation 中说明，可接受） |
| 代码质量 | 10% | 5/5 | 非测试代码零 unwrap/expect；测试内使用合理；命名/格式风格一致 |

加权 = 5×0.30 + 5×0.20 + 5×0.15 + 5×0.10 + 4×0.15 + 5×0.10 = **4.85**（向下取整记为 4.75 反映微弱测试缺口）

## 关键发现

1. **架构一致性优秀**：通过 `format!("{ASSET_SELECT} WHERE ...")` 复用统一列序，物理上消除了 task_002 reviewer 指出的"三处列序对齐"风险（架构方案 §十一 关键要求）。
2. **M-1 调研真实执行且结论可用**：`grep` 结果显示 `extraction/mod.rs:4` 是 scheduler 唯一声明点、无其他 `scheduler::` 引用 — 为 task_008 提供清晰前置条件。
3. **测试设计稳健**：噪声样本（其他 root 的 md / 本 root 的 image / 多版本 imported_at）一次性验证 SQL WHERE 与 ORDER BY 都正确，超出最低要求。

## 次要建议（非阻塞）

- 若未来发现"幽灵更新"，可考虑在 `update_markdown_derivative` 末尾加 `affected == 0 -> Err`，但当前 scheduler 调用链先 find 再 update，确实不必要。

## 阻塞项

无 BLOCKER，无 MAJOR。
