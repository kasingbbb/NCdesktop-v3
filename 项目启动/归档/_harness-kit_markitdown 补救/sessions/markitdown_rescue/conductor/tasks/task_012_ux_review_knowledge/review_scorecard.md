# Review Scorecard — task_012_ux_review_knowledge

## 审查思考过程

1. **Task 意图**：Review-only 审计任务——盘点知识抽取 / 全文搜索 / 标签筛选三条数据流的真实数据源，回答 5 个必答问题（Q1~Q5），对 ❌ 项给出下一轮迭代骨架。本 task 不写代码。

2. **AC 检查结果**：
   - AC-1（含抽取入口 + FTS 数据源 + 三场景数据流图）：✅ 报告"详细分析 Q1"列出 `extract_concepts_for_library` 入口及调用路径；FTS 数据源在 Q2 与场景 B 中明确点出（fts_assets 仅索引 name + file_path）；"三个场景的实际数据流图"section 完整提供标签筛选 / 全文搜索 / 知识抽取三条数据流的箭头式路径描述（含文件名 + 函数）。
   - AC-2（5 个问题全部给结论）：✅ Q1/Q4/Q5 ✅ 已对齐；Q2/Q3 ❌ 未对齐，每条均有 file:line 引用。
   - AC-3（每项有 ✅/⚠️/❌ 判定）：✅ 自测验证矩阵 + 详细分析双重标注。
   - AC-4（❌ 项有迭代骨架）：✅ 末尾 P-1（搜索去重+全文索引）、P-2（自动触发增量抽取）、P-3（placeholder 防御性测试）。

3. **独立复核（关键证据复核结果）**：
   - Q2 ❌ 证据复核：跑 grep migration.rs 确认 fts_assets 三处 trigger 都只写 `name, file_path`（行 436/439/442/443），不含 structured_md；search.rs + commands/search.rs grep `source_asset_id|GROUP BY|UNION|DISTINCT` 返回空 → 完全无去重。**与 Dev 判定一致**。
   - Q3 ❌ 证据复核：`grep -rn "concept-extract-requested" src/` 返回 0 条；后端唯一引用在 scheduler.rs:686 注释 + 691 emit。**与 Dev 判定一致，死信确认**。
   - Q5 ✅ 证据复核：knowledge.rs:388-391 确实用 `status='extracted'` LEFT JOIN + WHERE 外层过滤 markdown 衍生件。逻辑正确。
   - scheduler.rs:680-700 区域 emit 真实存在，注释自陈"MVP 采用事件驱动：前端监听..."——证据链完整。

4. **关键发现**：
   - **最该被 PM 看到**：FTS 索引只覆盖 name + file_path，**完全不索引正文**——这意味着 markitdown 把 PDF/音频转出 .md 后，用户在搜索框搜 PDF 内文字依旧搜不到。Q2 ❌ 同时叠加双重 hit 问题。这是 markitdown 集成最大的"做了但用户感知不到"反差点。
   - Q3 死信事件 + Q4 算法已对齐 = 自动闭环只差前端一段 listener 或后端直接 spawn。P-2 方案 A（后端 tokio::spawn）更稳健，建议优先考虑。

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 审计结论事实正确性 | 25% | 5 | 4 处关键 file:line 独立复核全部命中，无虚构引用 |
| 安全性 | 25% | 5 | review-only 任务无代码改动，且报告并未引导写出不安全代码 |
| 报告结构清晰度 | 15% | 5 | 自测矩阵 + 详细分析 + 数据流图 + 迭代骨架四段式结构，file:line 完整 |
| 5 个问题覆盖度 | 15% | 5 | Q1~Q5 每个问题均有独立小节、判定、证据、分析 |
| 架构一致性 | 10% | 4 | 结论与架构 §九 R7 期望一致；Q4 正确识别 task_008 content_hash 链路；轻微扣分：未对照引用 §九 R7 原文 |
| 可维护性 | 10% | 4 | 已知局限诚实登记（4 条），但未审 knowledge_understanding/units/synthesis.rs，留下盲区由 PM 决定是否补审 |

**综合分：4.75/5**（加权计算：5×0.25 + 5×0.25 + 5×0.15 + 5×0.15 + 4×0.10 + 4×0.10 = 4.80）

## 总体判断

- [x] **PASS**

## 问题列表

### BLOCKER
无。

### MAJOR
无。

### MINOR

1. **未交叉验证 placeholder 写入路径的 status 回写**
   - 位置：output.md Q5 末段 + scheduler.rs:749-790 区域
   - 说明：Dev 已诚实登记此局限并建议补 unit test。可作为 P-3 跟踪项。
2. **knowledge_understanding.rs / knowledge_units.rs / knowledge_synthesis.rs 未逐行核 SQL 数据源**
   - 位置：output.md "已知局限"第 1 条
   - 说明：grep 显示无 `read_to_string` 调用即认为对齐，未走 SQL 走查。建议下次 review 时补审。
3. **Q2 ❌ 严重程度未定调**
   - 位置：output.md "需要 Reviewer 特别关注的地方"第 1 条
   - 说明：Dev 主动请求 PM 判断是否降级为 ⚠️。Reviewer 倾向保留 ❌：FTS 不索引正文 + 双条 hit 双重缺陷叠加，已超出"局部小改"范畴。

## 给 PM 的传达建议

- 立即跟进 P-1（搜索）与 P-2（自动触发）作为 markitdown 集成的"完成感"补丁，否则 task_008 落地的用户感知 ≈ 0。
- P-3 可推迟到下一轮回归测试 sprint。
