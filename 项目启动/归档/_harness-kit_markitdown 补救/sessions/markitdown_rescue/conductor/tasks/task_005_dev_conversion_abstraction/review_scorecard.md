# Review Scorecard — task_005_dev_conversion_abstraction

- **Reviewer**：Code Reviewer（Conductor 角色）
- **审查时间**：2026-05-12
- **审查产物**：
  - `src-tauri/src/extraction/conversion.rs`（新建，247 行含测试）
  - `src-tauri/src/extraction/mod.rs`（+1 行：`pub mod conversion;`）
  - `src-tauri/Cargo.toml`（+1 行：`sha2 = "0.10"`）

---

## 1. 审查前验证（契约 8 字段）

| 字段 | 状态 |
|------|------|
| 实现摘要 | ✅ |
| 修改的文件清单 | ✅ |
| 对 Architect 方案的遵守声明 | ✅（含偏离） |
| 测试命令 | ✅ |
| 测试结果 | ✅ |
| 自测验证矩阵 | ✅ |
| 已知局限 | ✅ |
| Reviewer 特别关注事项 | ✅ |

契约完整。

---

## 2. 思考协议

**Task 意图复述**：在 `extraction/conversion.rs` 中落地 ADR-005 要求的"无新 trait"方案，提供 `ConversionAttempt` 纯数据结构 + serde、`file_sha256` 流式哈希、`classify_error` 错误归类工具；不破坏 M-1（cargo check 0 error）。

**AC 逐条核对**：见第 4 节。

**关键发现**：
1. Cargo.toml 的 sha2 偏离是 **合理且必要** 的：input.md / session_context 的"sha2 已在 Cargo.toml"陈述与事实不符（原始 Cargo.toml 仅有 sha1）。Dev 在 output.md 中明确登记并提供事实证据。
2. 实现完全遵守 ADR-005（无新 trait、纯数据 + serde），匹配顺序设计合理（markitdown 先于 python）。

---

## 3. 实地验证

- `cargo check`：0 error，3 既存 warning（非本 task 引入）。✅ M-1 保持。
- `cargo test --lib extraction::conversion`：6/6 PASS。
- `extraction/mod.rs:4`：scheduler 注释保留未取消。✅
- `Cargo.toml`：仅新增 `sha2 = "0.10"` 一条，未连带升级 / 引入其他依赖。✅
- 非测试代码内 `unwrap()` / `expect()` 出现次数：**0**（所有断言均在 `#[cfg(test)]` 模块内）。✅

---

## 4. AC 逐条核对

| AC | 要求 | 结果 |
|----|------|------|
| AC-1 | 9 字段全部存在、camelCase 序列化、Serialize + Deserialize 双向 | ✅ 9 字段一一对应；`#[serde(rename_all = "camelCase")]`；`conversion_attempt_serializes_camel_case` + `conversion_attempt_roundtrip` 双测覆盖 |
| AC-2 | 流式 8KB / hex 小写 / 已知向量 | ✅ 8KB buf 循环；`format!("{:x}", ...)`；"hello world" → `b94d27b9...cde9` 已断言；20000 字节多块边界测试 |
| AC-3 | 8 种 class 全覆盖、不区分大小写、markitdown 优先于 python | ✅ 8 分支齐全；`to_lowercase()` 后子串匹配；`classify_error_priority_markitdown_over_python` 显式覆盖 |
| AC-4 | 单测覆盖 file_sha256 + 8 个 stderr | ✅ 6 个单测；known vector + multi-block + 8-class coverage + priority + camelCase + roundtrip |
| AC-5 | `pub mod conversion;` + scheduler 注释保留 | ✅ mod.rs:1 添加；mod.rs:4-5 scheduler 注释完整未动 |

---

## 5. ADR-005 合规

- 无新 trait（仅 `derive` Debug/Clone/Serialize/Deserialize）。✅
- `ConversionAttempt` 为纯数据结构，无业务方法（无 impl 块）。✅
- 不触碰现有 `Extractor` trait（mod.rs:12-25 未改动）。✅

---

## 6. 偏离判断 — `sha2 = "0.10"`

**判断：合理偏离，无需 ESCALATE，需在 Conductor 层登记 session_context / input.md 的事实错误。**

理由：
1. 硬约束 #5（"哈希算法固定 SHA-256"）+ AC-2（`file_sha256` 必须实现）必须要 sha2，但原始 Cargo.toml 不存在该依赖。
2. `extraction/mod.rs:4` 的既存注释已明确 scheduler 因"sha2 等"依赖未恢复而被屏蔽 —— 这是 Architect 方案 / session_context 的事实陈述错误。
3. Dev 选择"满足核心约束 + 主动登记偏离"，而非"机械执行错误事实导致 AC-2 不可达"，处置正确。
4. `sha2 = "0.10"` 与既存 `sha1 = "0.10"` 同属 RustCrypto 维护，无版本冲突；cargo check 验证通过。
5. output.md 偏离说明充分（事实对比 + 风险评估 + 备选方案陈述）。

**建议**：Conductor 在 progress.md 登记 session_context.md / task_005 input.md 的事实错误，task_008 接线时无需再处理 sha2。

---

## 7. 六维评分（权重综合）

| 维度 | 权重 | 评分 | 说明 |
|------|------|------|------|
| 功能正确性 | 30% | 5/5 | AC 全过；known vector + multi-block 双向验证 |
| 架构一致性 | 20% | 5/5 | 严格遵守 ADR-005；无新 trait；纯数据 + serde |
| 可维护性 | 15% | 5/5 | 注释清晰；匹配顺序在代码内解释；测试自带回归网 |
| 安全性 | 10% | 5/5 | 无子进程；无 SQL；无 unwrap；error_class 不暴露原文 |
| 测试覆盖 | 15% | 5/5 | 6 测试覆盖 5 个 AC 关键路径 + 边界（多块、歧义、roundtrip、null 序列化） |
| 代码质量 | 10% | 5/5 | 无 unwrap/expect 非测试出现；命名清晰；返回 `std::io::Result` |

**综合分：5.00 / 5**

---

## 8. 裁决：PASS

- BLOCKER：0
- MAJOR：0
- MINOR：0
- 建议（非阻断）：
  1. 后续 task_008 应在 scheduler 解封时把 `compute_sha256(&str)` 与 `file_sha256(&Path)` 的边界统一（一处算字符串、一处算文件），避免日后混淆。
  2. `classify_error` 兜底"conversion_error"未来若想区分 `dependency_error` / `runtime_error`，可扩展无破坏。

---

## 9. 给 Conductor 的回执

- 状态：**PASS**
- 综合分：**5.00 / 5**
- sha2 偏离判断：**合理**（无需 ESCALATE，建议登记 session_context / input.md 的事实错误以避免 task_008 重复处理）
- 下一步：可推进 task_006；同时建议 Conductor 在 progress.md 备注"sha2 已由 task_005 落地"以纠正 session_context.md 的事实描述。
