# Preserve vs Modify Matrix — task_011

> 锁定 `markitdown.rs` / `scheduler.rs` 中"易被后续 task 误重构 / 误删"的关键行为，逐项给出**保留 / 修改 / 删除**决策与源码锚点。本文件由 task_011 产出，并被 Reviewer 在后续涉及 markitdown / scheduler / extraction 路径的 task 引用为 checklist。

源码版本参照：`NCdesktop/src-tauri/src/extraction/extractors/markitdown.rs`（task_007/008/010 后稳定态）。

---

## 矩阵主体（10 项，≥ input.md 字面 6 项 + grep 发现 ≥ 2 项）

| # | 行为 | 现状 | 决策 | 替代实现指向 | 源码锚点 |
|---|---|---|---|---|---|
| 1 | 90s 子进程总超时（`MARKITDOWN_TIMEOUT`） | 已实现 | **保留** | — | `markitdown.rs:9`（常量）；`markitdown.rs:197`（`run_with_timeout` 调用点）；`markitdown.rs:303-358`（实现） |
| 2 | image 输出为空 → 最小元数据 MD 回退（`extractor_type = "markitdown_image_fallback"`） | 已实现 | **保留**（task_008 已将 extractor_type 固定为 `markitdown_image_fallback`） | task_008 AC-5 | `markitdown.rs:259-283`（image fallback 分支） |
| 3 | markitdown 版本探测缓存（`probe_markitdown_version` + `RwLock<Option<String>>`） | 已实现 | **保留** | — | `markitdown.rs:24`（字段）；`markitdown.rs:207-211`（首次成功时缓存）；`markitdown.rs:367-381`（实现） |
| 4 | `exit==0 && stdout==''` 判 success | 已实现（历史） | **修改** → 改走 `classify_output` 四元判定 | task_008 `failure_code::classify_output` | `failure_code.rs:79-146`；`markitdown.rs:203`（调用点） |
| 5 | `python_candidates` 顺序探测（4 级：embedded → user-config → python3 → python） | 已实现 | **修改** → 严格三级（task_007 已通过 `runtime_check_failed` 入口短路把"嵌入式自检失败"独立成第 0 级前置） | task_007 `runtime_check::verify_runtime_manifest` + `extract()` 入口短路 | `markitdown.rs:132-136`（task_007 短路）；`markitdown.rs:387-421`（候选顺序） |
| 6 | `SUPPORTED_MIME_TYPES` 含 `audio/*` / `video/*` | 当前已**不含**（task_010 已剔除） | **保留**（grep gate） | task_010 AC-1 | `markitdown.rs:59-74`（数组定义）；`markitdown.rs:55-58`（grep CI gate 注释） |
| 7 | `run_with_timeout` 用后台读线程持续 drain stdout/stderr（避免 macOS 16–64KB pipe buffer 死锁） | 已实现 | **保留**（删除会让大输出文件被误判超时） | — | `markitdown.rs:314-328`（后台读线程 spawn）；`markitdown.rs:339-340`（kill 后 join） |
| 8 | `extract()` 入口对 audio/video 误路由的 `debug_assert!` + release fallback `E_AUDIO_WRONG_ROUTE` | 已实现 | **保留**（H5 / PRD 底线 #4） | task_010 AC-2 | `markitdown.rs:147-163` |
| 9 | `parse_error_with_class` 给 `ExtractionError::ParseError` 加 `error_class:xxx\|` 前缀（scheduler 据此分类 + 落库） | 已实现 | **保留** | task_008 scheduler 解析；`extraction::conversion::classify_error` | `markitdown.rs:361-364`；`markitdown.rs:133/158/289`（三处调用） |
| 10 | `options.runtime_check_failed` 入口短路（自检失败时**不**起 Python 子进程） | 已实现 | **保留** | task_007 FIX AC-3 | `markitdown.rs:131-136` |

---

## 决策语义说明

- **保留**：本行为是 task_007/008/009/010 PASS 后稳定形态的关键支柱，任何后续 task 删除 / 大改 → Reviewer 拒收。源码处用 `// task_011 preserve: ...` 注释自我标记。
- **修改**：本行为来自历史实现，已被指定 task 用更严谨方案替代。任何后续 task 把它"回退"到历史实现 → Reviewer 拒收。
- **删除**：本期不存在（无原生功能需在 task_011 阶段净化）。

---

## 与 task_008 `classify_output` 联调声明（AC-4）

**关键路径**：image 输入 + 子进程 exit 0 + stdout 空。

判定顺序（按 `markitdown.rs:203 → 223 → 267` 真实执行流）：

1. `classify_output(stdout="", Some(0), elapsed)` → `Err(FailureCode::EOutputEmpty)`；
2. 此时 `is_image == true`（由文件扩展名判定，`markitdown.rs:176-185`）→ 标记 `had_empty_success = true`；
3. 候选循环结束后命中 `markitdown.rs:267` 分支 → 返回 `extractor_type = "markitdown_image_fallback"` + `quality_level = 1`，**不**作为失败回包。

**关键不变量**：
- image fallback 路径**不**被 classify_output 错判为最终 `EOutputEmpty`（任何后续 task 把 image fallback 分支挪到 classify_output 上游 / 干掉 `is_image` 早判 → 违反 AC-4，Reviewer 拒收）；
- 非 image 输入（如 pdf）+ exit 0 + 空 stdout → classify_output → `EOutputEmpty` → 候选循环结束后**不**走 fallback → 返回失败错误码（与 task_008 字面一致）。

---

## Reviewer Checklist 引用

> 后续涉及 `markitdown.rs` / `scheduler.rs` / extraction 路径的 task，Reviewer 应核对：

- [ ] "保留"项注释 `// task_011 preserve:` 是否被误删（grep 计数应 ≥ 4）；
- [ ] "修改"项是否被回退到原实现（特别是第 4 项 `classify_output` 与第 5 项 `python_candidates` 严格三级）；
- [ ] `grep -nE '"(audio|video)/' src/extraction/extractors/markitdown.rs` 在 `SUPPORTED_MIME_TYPES` 数组段仍为 0 命中；
- [ ] `MARKITDOWN_TIMEOUT` 仍为 `Duration::from_secs(90)`；
- [ ] `extractor_type == "markitdown_image_fallback"` 字面（不得改为 `"markitdown"`，否则 quality 评估与下游知识进化系统会把空回退当真识别）；
- [ ] `run_with_timeout` 内的后台读线程仍在 spawn 后立即 take 句柄（不得改回阻塞 `child.wait_with_output()`）。

任一项失败 → 视为回归，要求 Dev 修复。
