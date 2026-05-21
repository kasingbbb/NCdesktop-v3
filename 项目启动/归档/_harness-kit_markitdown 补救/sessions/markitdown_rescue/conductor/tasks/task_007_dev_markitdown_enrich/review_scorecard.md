# Review Scorecard — task_007_dev_markitdown_enrich

## 审查前验证（交接契约 8 字段）

- [x] 实现摘要 — 完整，5 项条目
- [x] 修改的文件 — 表格列出 3 个文件、变更类型、说明
- [x] 对 Architect 方案的遵守声明 — 含偏离说明（C 节方案一选择 + 测试输入剥离前缀）
- [x] 测试命令 — 提供
- [x] 测试结果 — 7 passed, 0 failed
- [x] 自测验证矩阵 — 9 行，正常路径全 PASS，1 行"未测"已合理标注（subprocess 真启动避免）
- [x] 已知局限 — 4 条
- [x] 需 Reviewer 关注 — 5 条

→ 契约完整，进入实质性审查。

## 审查思考过程

1. **Task 意图**：增强 `MarkItDownExtractor`：缓存版本号、把错误归类到 `error_class` 编码到 ParseError、提升嵌入式 venv python 候选优先级；为 task_008 scheduler 落库 ConversionAttempt 准备元数据通道。

2. **AC 检查结果**：
   - AC-1（版本缓存 + `detected_version()`）：✅ `RwLock<Option<String>>` + inherent `detected_version()`；首次 extract 成功路径 best-effort 探测
   - AC-2（error_class 前缀编码 ParseError）：✅ `parse_error_with_class()` 调用 `conversion::classify_error`，格式 `error_class:xxx|<msg>`；契约清晰
   - AC-3（候选顺序 embedded → cmd → python3 → python，去重）：✅ `python_candidates` 闭包 `push_unique` 实现
   - AC-4（7 个测试，3 个分类 + 候选 + 去重 + 缺省 + 版本初值，不真起 subprocess）：✅
   - AC-5（手测）：scope 外，由 Conductor 在合成阶段验证

3. **关键发现**：
   - **正向**：实现高度紧贴 input.md；`Extractor` trait 未改，`detected_version` 是 inherent；scheduler 注释保持注释；cargo check 0 error；7 tests 全绿（本地复跑验证）；安全要求遵循（`Command::args` 数组传参、stderr 仅 `log::warn!`、无 unwrap/expect）。
   - **轻微偏离**：`error_class_file_not_found` 测试为绕开 task_005 `classify_error` 优先级规则，剥离了 `python3:` 前缀。Dev 已在 output.md "偏离说明 §2" 显式标注。这反映了真实 extract() 失败路径在含 "python" 子串时会被归类为 `python_unavailable`——本测试仅证 `parse_error_with_class` 函数对纯 file_not_found 输入分类正确，未覆盖实际链路。属于 MINOR，可由 task_008 scheduler 在文件预检阶段显式构造来覆盖。

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 30% | 5 | AC-1~4 全部满足；cargo check 绿；7 tests pass；scheduler 注释未动；mod.rs 仅最小构造点适配 |
| 架构一致性 | 20% | 5 | `Extractor` trait 不变；`detected_version` inherent；`ExtractOptions` 增量字段沿用 `derive(Default)` 无破坏；未引入新依赖（仅 std `RwLock`） |
| 可维护性 | 15% | 4 | `error_class:xxx|` 前缀契约清晰，但仅在 output.md "已知局限 §2" 提及 `|` 边界，建议在 `parse_error_with_class` doc 加一行注释。`probe_markitdown_version` best-effort 语义清晰 |
| 安全性 | 10% | 5 | `Command::args(...)` 数组传参（grep 验证两处均合规）；stderr 仅 `log::warn!`，UI 不可见原文；ParseError 字符串供 scheduler 落库，不外泄前端 |
| 测试覆盖 | 15% | 4 | 7 个测试覆盖核心分类与候选；显式回避 subprocess，借 `build_parse_error_msg` 复刻聚合逻辑——合理。但 file_not_found 测试输入剥离 `python3:` 前缀使其与真实 extract() 路径解耦，扣 1 分 |
| 代码质量 | 10% | 5 | 无 `unwrap()`/`expect()` 在非测试代码（`unreachable!` 在测试是无害匹配）；命名清晰；闭包 `push_unique` 去重优雅；`is_none_or` Rust 1.82+ stable |

**综合分：4.70/5**（加权计算：5×0.30 + 5×0.20 + 4×0.15 + 5×0.10 + 4×0.15 + 5×0.10 = 1.50+1.00+0.60+0.50+0.60+0.50 = 4.70）

## 总体判断

- [x] **PASS**

无 BLOCKER，无 MAJOR；2 个 MINOR 已识别。

## 问题列表

### BLOCKER
（无）

### MAJOR
（无）

### MINOR

1. **`error_class_file_not_found` 测试与真实链路解耦**：测试输入 `"FileNotFoundError: ..."` 不含 `python3:` 前缀，绕开了 task_005 `classify_error` 的 `python_unavailable` 优先级规则。建议在 task_008 添加端到端测试覆盖 scheduler 文件预检阶段。当前属可接受取舍。
2. **`error_class:xxx|<msg>` 解析契约只在 output.md 提及**：建议在 `parse_error_with_class` 函数上加 doc 注释，说明 task_008 解析时应只 `splitn(2, '|')`，避免 attempts 内含 `|` 引起歧义。
3. **`probe_markitdown_version` 不解析 stderr**：部分 venv 中 `markitdown --version` 可能写入 stderr。属 best-effort，不影响 task_007 scope；task_008 可考虑回退到 `check_markitdown_status`（Dev 已在已知局限 §1 提示）。

## 给 Dev 的修复指引

PASS，无需修复。MINOR 项可在 task_008 集成时一并处理。
