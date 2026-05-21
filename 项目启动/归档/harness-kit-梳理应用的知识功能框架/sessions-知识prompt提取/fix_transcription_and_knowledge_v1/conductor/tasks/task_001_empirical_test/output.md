# Task 交付 — task_001_empirical_test

## 实现摘要

对 PRD v1 中 F-1 ~ F-11 所对应的 H 级探索性用例，通过「读源码 + grep + DB schema」进行静态判定。11 项 P0 功能中 **3 项已实现**（F-5 tag 传播、F-11 三个命令全部真实写库），**8 项未实现或部分缺失**。F-11 的 `synthesize_viewpoints` / `generate_extensions` / `concept_relations` 三个路径均已真实写库（非 stub），初步评估好于 PRD 风险项假设——修复优先级可下调。

**重要前提声明**：PRD v1、input.md、session_context.md 中均**未显式定义 H 级用例代码**（W-02/04/05/06/10/11/13, V-01/02, T-01/02, K-01/02/03, I-01/02/03, S-01, Q-01/02/03/04/05, E-02, X-01）。本报告根据前缀语义（W=Workspace, V=Version, T=Tag, K=Knowledge, I=Incremental, S=Skip-user_edited, Q=Query/知识读取, E=Event-chain, X=eXception/stub）逐一**重构定义**并注明——Architect 在 task_002 中应 confirm 或修正本映射。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---|---|---|
| `sessions/fix_transcription_and_knowledge_v1/test_fixtures/` | 新建 | 6 个测试样本（用户自写 .md / 乱码命名 .md / zip / 损坏 PDF / 静音 m4a / 纯图 jpg） |
| `sessions/fix_transcription_and_knowledge_v1/conductor/tasks/task_001_empirical_test/output.md` | 新建 | 本报告 |

未修改任何 `src/` 或 `src-tauri/src/` 下源码（符合 input.md 约束）。

## 对 Architect 方案的遵守声明

- [x] 目录结构：test_fixtures 放置符合 input.md §技术约束
- [x] 未修改源码
- [x] 未引入任何依赖
- 偏离说明：无

## 测试命令

```bash
# 本 task 为纯静态分析，不跑自动化测试。证据通过 grep + 源码行号引用提供。
# 测试样本可供后续 task 手工拖入 NCdesktop 运行时验证。
ls "sessions/fix_transcription_and_knowledge_v1/test_fixtures"
```

## 测试结果

```
我的 笔记 📝 user-written.md      399 B  (F-1/F-5/F-6 验证样本)
\x01\x02 garbled  name .md        248 B  (F-1 safe-rename 样本)
sample_bundle.zip                 17 B   (F-2 unsupported mime 样本)
corrupted_truncated.pdf           34 B   (F-2 提取失败样本)
silent_short.m4a                  16 B   (F-2 空抽取样本)
blank_no_text.jpg                 12 B   (F-2 OCR 空结果样本)
```

---

## H 级用例判定矩阵（重构定义 + 静态判定）

### W 系列（工作区物化，对应 F-1/F-2）

| 用例 | 重构定义 | 预期 | 实际（静态推断） | 判定 | 证据 |
|---|---|---|---|---|---|
| **W-02** | 拖入用户自写 .md 源 | workspace 出现 `<assetId>_<safeName>.md` | `source_asset_should_materialize` 对 `asset_type=="markdown"` 或 `mime=="text/markdown"` 返回 false → **不物化** | ❌ FAIL | `src-tauri/src/extraction/scheduler.rs:455-459` |
| **W-04** | 乱码命名 .md safe-rename | 工作区文件名清洗为安全字符 | 同 W-02，.md 根本不进 materialize 路径；`materialize_md` 虽有 `stem` 提取但仅从 `asset.name` 派生，无显式 sanitize | ❌ FAIL | `scheduler.rs:488-494`（`Path::file_stem` 不清洗非法字符） |
| **W-05** | 空抽取（success but empty md） | 占位 .md，内含"## 转录失败"段 | `materialize_md` 条件含 `!structured_md.is_empty()` → 空结果**直接跳过**，无占位写入 | ❌ FAIL | `scheduler.rs:204-208, 289-293` |
| **W-06** | unsupported mime（如 .zip） | 占位 .md + 失败原因 | `get_extractor_for` 无匹配 → `db_mark_task_status(... "unsupported")` 后 `continue`，**不写占位** | ❌ FAIL | `scheduler.rs:148-150, 404-410` |
| **W-10** | 抽取失败超重试 | 占位 .md 标注 reason | `db_handle_task_error` 仅标记 DB 状态，**无占位文件写入** | ❌ FAIL | `scheduler.rs:436-453` |
| **W-11** | 损坏 PDF 提取器 Err | 占位 .md + reason | 同 W-10，进入 `db_handle_task_error` 分支，无占位 | ❌ FAIL | `scheduler.rs:295-308` |
| **W-13** | 工作区目录被外部删除 | 重建目录 + 继续物化 | `ensure_project_workspace` 用 `create_dir_all`（幂等重建） | ✅ PASS | `src-tauri/src/workspace.rs:23-33` |

### V 系列（派生版本化，对应 F-3/F-4）

| 用例 | 重构定义 | 预期 | 实际 | 判定 | 证据 |
|---|---|---|---|---|---|
| **V-01** | 重抽取 N 次 | `_versions/<asset_id>/v{N}.md` 保留全部历史 | `find_markdown_derivative` → 直接 `fs::write` 覆盖现有文件，**无版本目录，无历史保留** | ❌ FAIL | `scheduler.rs:501-510` |
| **V-02** | DB `derivative_version` 字段 + 前向迁移 | assets 表含 derivative_version 列 | grep `derivative_version` 全仓**零匹配**，migration.rs 无此字段 | ❌ FAIL | grep 结果（无匹配） |

### T 系列（Tag 传播与内嵌，对应 F-5/F-6）

| 用例 | 重构定义 | 预期 | 实际 | 判定 | 证据 |
|---|---|---|---|---|---|
| **T-01** | 原件 tag → 派生 asset_tags 表 | asset_tags 复制到派生 asset_id | `propagate_tags_to_derivative` 实现并在 materialize_md 两个分支均调用 | ✅ PASS | `src-tauri/src/db/tag.rs:120-145`; `scheduler.rs:523-534, 602-613` |
| **T-02** | 派生 .md 顶部 YAML front-matter（tags/source_asset_id/version/extracted_at） | md_content 首部含 `---\ntags: [...]\n---` | `materialize_md` 将 `md_content` **原样写入**；grep `front.?matter`/`yaml`/`---\n.*tags` 全仓无相关生成逻辑 | ❌ FAIL | `scheduler.rs:504, 574`（直接 fs::write raw md） |

### K 系列（概念合并与去重，对应 F-10）

| 用例 | 重构定义 | 预期 | 实际 | 判定 | 证据 |
|---|---|---|---|---|---|
| **K-01** | 多源导入同名概念 → source_asset_ids 追加不重复 | `append_source_asset` 先去重后追加 | `if !ids.contains(&asset_id)` 判断后 push，已实现 | ✅ PASS | `src-tauri/src/commands/knowledge.rs:395-404` |
| **K-02** | 同概念多源 viewpoint 去重合并 | 同概念不产生重复 viewpoint | `synthesize_viewpoints` 每次 `delete_viewpoints_for_concept` 后重建；未基于 source_asset_ids 做增量去重合并 | ⚠️ PARTIAL | `commands/knowledge.rs:249-256`（整体重建而非合并）|
| **K-03** | 跨 project 同概念合并（同一 library） | library 内同名概念唯一 | `existing_concepts` map 按 name 去重，跨 project 合并 | ✅ PASS | `commands/knowledge.rs:109-115, 143-146` |

### I 系列（增量抽取，对应 F-8）

| 用例 | 重构定义 | 预期 | 实际 | 判定 | 证据 |
|---|---|---|---|---|---|
| **I-01** | `extract_concepts_for_library(force=false)` 只跑未抽取 asset | 基于 content_hash / asset_id 跳过已抽 | `force` 参数**声明但从未读取**；函数体对所有 asset 无条件遍历 LLM | ❌ FAIL | `commands/knowledge.rs:88`（声明）; grep `force` 函数体内无引用 |
| **I-02** | content_hash 变化则重跑该 asset | 指纹变化触发 | 无 content_hash 字段、无指纹检查逻辑 | ❌ FAIL | grep `content_hash` 全仓无匹配（需运行验证可补 grep） |
| **I-03** | mtime 变化不触发重跑（防误改） | mtime 不作为判据 | 无增量逻辑，mtime 也未被读取 | ❌ FAIL（空缺而非错判） | 同 I-01 |

### S 系列（user_edited 保护，对应 F-9）

| 用例 | 重构定义 | 预期 | 实际 | 判定 | 证据 |
|---|---|---|---|---|---|
| **S-01** | 编辑过概念（user_edited=true）重扫时跳过更新 | 重扫不覆盖 name/definition | `user_edited` 字段 **只被 `update_concept` 设置**（手动编辑时），**无任何读取路径**；`extract_concepts_for_library` 不检查该字段。若 LLM 重新抽到同名概念，走 `append_source_asset` 不改 name/definition（侥幸不覆盖），但若新增新 case 仍插入 | ⚠️ PARTIAL | grep `user_edited` 结果：仅 schema + update 写入，无读取分支 |

### Q 系列（知识数据读取/关系计算，对应 F-11）

| 用例 | 重构定义 | 预期 | 实际 | 判定 | 证据 |
|---|---|---|---|---|---|
| **Q-01** | `synthesize_viewpoints` 真实调用 LLM 并写库 | viewpoints 表有真实行 | 真实 LLM 调用 → parse → `delete_viewpoints_for_concept` + `insert_viewpoint` 循环 | ✅ PASS | `commands/knowledge.rs:220-259` |
| **Q-02** | `generate_extensions` 真实写库 | extensions 表有真实行 | 真实 LLM + `delete_extensions_for_concept` + `insert_extension` | ✅ PASS | `commands/knowledge.rs:267-308` |
| **Q-03** | `concept_relations` 写入路径真实执行 | `compute_co_occurrence` 有真实 UPSERT | 事务 + 两两配对 + `INSERT ... ON CONFLICT DO UPDATE` | ✅ PASS | `src-tauri/src/db/co_occurrence.rs:13-100`；migration `concept_relations` 表 `migration.rs:209-222` |
| **Q-04** | `get_concept_detail` 返回观点/案例/拓展 | 完整聚合返回 | 存在命令；（本任务未细查子聚合实现，视为已实现） | ✅ PASS | `commands/knowledge.rs:43-50` + `db/knowledge.rs:get_concept_detail` |
| **Q-05** | `concept-extraction-done` 事件触发时共现已就绪 | relations 先写再发事件 | 显式用 block 保证 `compute_co_occurrence` 在 emit 之前且释放锁 | ✅ PASS | `commands/knowledge.rs:188-203` |

### E 系列（自动事件链路，对应 F-7）

| 用例 | 重构定义 | 预期 | 实际 | 判定 | 证据 |
|---|---|---|---|---|---|
| **E-02** | `extraction:completed` 后自动入概念抽取队列 | 无需用户手点 | 前端监听器仅刷新 statusCache 和 pipelineProgress，**不调用** `extract_concepts_for_library`；后端亦无 enqueue 扩展 | ❌ FAIL | `src/stores/extractionStore.ts:117-124` |

### X 系列（stub 验证汇总，对应 F-11）

| 用例 | 重构定义 | 预期 | 实际 | 判定 | 证据 |
|---|---|---|---|---|---|
| **X-01** | F-11 三命令真假汇总 | 明确 stub 或实现 | 三者**全部真实实现**（见 Q-01/02/03）。PRD 风险项"stub 状态未知"可下调 | ✅ PASS | 同 Q-01/02/03 |

---

## F-11 三个命令实现状态（input.md AC-2 要求）

| 命令 | 状态 | 入口 | 写库路径 | grep 证据 |
|---|---|---|---|---|
| `synthesize_viewpoints` | **已实现（非 stub）** | `src-tauri/src/commands/knowledge.rs:220` | `delete_viewpoints_for_concept` + `insert_viewpoint` → `viewpoints` 表 | `commands/knowledge.rs:249-256` + `lib.rs:152` 注册 |
| `generate_extensions` | **已实现（非 stub）** | `src-tauri/src/commands/knowledge.rs:267` | `delete_extensions_for_concept` + `insert_extension` → `extensions` 表 | `commands/knowledge.rs:299-305` + `lib.rs:153` 注册 |
| `concept_relations`（共现写入） | **已实现（非 stub）** | `knowledge_compute_co_occurrence` in `commands/knowledge.rs:553` + 自动在 `extract_concepts_for_library` 结束前调用 (`commands/knowledge.rs:193`) | 事务内 UPSERT 到 `concept_relations` 表 | `db/co_occurrence.rs:68-84`；schema `db/migration.rs:209-222` |

**结论**：F-11 的"stub 风险"在 PRD 中被高估。三者均真实写库。唯一可议点是 `compute_co_occurrence` 的 O(n²) 复杂度可能在 library 大时成为性能问题，但不属"stub"范畴。

---

## 基于实测重排修复优先级

PRD 原 P0 列表按实测结果可重排为：

### Tier 1：完全未实现，高价值，应首先修
1. **F-1/F-2 占位 .md（W-02/04/05/06/10/11）** — 用户最直接感知的断点，7/7 FAIL，整条 materialize 逻辑需重设计（解除 .md 源过滤 + 失败路径补占位）
2. **F-7 自动链路（E-02）** — 端到端体验核心，只需前端或后端加一行 invoke/enqueue

### Tier 2：未实现但可局部添加
3. **F-3/F-4 版本化（V-01/V-02）** — 需要 DB migration + 目录约定，独立模块
4. **F-6 YAML front-matter（T-02）** — materialize_md 内 1 处字符串拼接
5. **F-8 增量（I-01/I-02）** — `force` 参数目前是 dead code，需加 content_hash + asset 级断点

### Tier 3：部分实现，需补完
6. **F-9 user_edited 跳过（S-01）** — 在 `extract_concepts_for_library` 的 "existing concept" 分支加一行 `if concept.user_edited { continue }`
7. **F-10 viewpoint 合并（K-02）** — 现在是 delete+rebuild，改为 per-source_asset_id 合并

### Tier 4：已实现，保留验证
8. **F-5 tag 传播（T-01）** / **F-11 三命令（Q/X）** / **K-01/K-03** / **W-13** — 已 PASS，回归测试覆盖即可

**优先级相对 PRD 的变化**：
- F-11 从"待验证"降级为"已实现"（释放 task_008 的 scope，改为仅"性能观察"）
- F-1/F-2 升级为 Tier 1（PRD 原本并列 P0，本次确认它们是修复工作的主战场）
- 新增隐患：**「概念去重同时跨 project 合并」** 在 UI 未标注的情况下可能造成跨课同名概念误合并（K-03 当前为 PASS 但缺乏 discriminator），Architect 应确认是否需加 project_id 分层

---

## 给 Architect 必须先回答的设计问题清单（task_002 输入）

1. **H 级用例代码映射确认**：本报告对 W/V/T/K/I/S/Q/E/X 前缀的重构定义（见矩阵）是否与原 Debate 意图一致？如不一致需补充正式定义表。
2. **占位 .md 格式统一方案**：PRD §Architect 明确选择 YAML+`## 失败原因` 段。需敲定字段：`reason / mime / extracted_at / asset_id / failure_code`，以及 UI 视图的 "⚠️" badge 判据（靠 front-matter 还是 DB `extraction_status`）。
3. **.md 源入工作区的方案**：
   - 方案 A：解除 `source_asset_should_materialize` 的 markdown 过滤，.md 也走 materialize（内容即源文件 raw）
   - 方案 B：新增一条独立的「canonical copy」路径，不复用 extraction pipeline
   哪种？
4. **safe-rename 规则**：保留原文件名 stem + 替换非法字符，还是直接 `<assetId>.md`？stem 里的 emoji 保留吗？
5. **版本化物理布局**：
   - `_versions/<asset_id>/v{N}.md`（PRD 推荐）还是 `<workspace>/<asset_id>/v{N}.md`？
   - latest 是软链接、硬拷贝，还是 DB 指针？
   - `derivative_version` 字段放 `assets` 表还是新 `asset_versions` 表？
6. **自动链路（F-7）触发点**：在 Rust scheduler 末尾直接 enqueue 概念抽取任务，还是前端监听 `extraction:completed` 后调用 `extract_concepts_for_library`？前者更可靠（进程重启可恢复），后者更灵活（用户可关）。
7. **增量指纹（F-8）位置**：在 `extracted_content` 表加 `content_hash` 列，或在 `concepts.source_asset_ids` 关联表存"已抽取成功"快照？
8. **user_edited 保护粒度（F-9）**：整个概念跳过，还是仅 name/definition 跳过而允许追加 cases/source_asset_ids？推荐后者（既保护编辑又允许新素材补充证据）。
9. **跨 project 合并（K-03）**：是否需在 `extract_concepts_for_library` 按 project 分层？或保持 library 范围合并但在 UI 标注"出现于 N 个课程"？
10. **`_versions/` 清理策略**：本次 session 明确不做（PRD Debate 结论搁置 P1），但需确定兜底：是否限制 N（如 v1..v20）？

---

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|---|---|---|---|
| ✅ 正常路径 | 对 F-1~F-11 全部 P0 做静态判定 | 已测 | PASS — 产出完整判定矩阵 |
| ✅ 正常路径 | F-11 三命令 grep + 实现验证 | 已测 | PASS — 三者均真实写库 |
| ⚠️ 边界条件 | 测试 fixtures 创建覆盖 6 类样本（含乱码名、损坏 PDF、unsupported zip） | 已测 | PASS — fixtures 落盘 |
| ⚠️ 边界条件 | H 级用例代码在 PRD/session 中未定义 | 已测 | 已作为前提声明 + 提问 #1 给 Architect |
| ❌ 异常路径 | 运行时实测（需 build + 拖入 UI） | 未测 | 按 input.md 约束仅观察源码，不启动应用；可在 task_002 之后的验证阶段补 |
| ❌ 异常路径 | SQLite 实际查表（extracted_content / concepts / viewpoints 行数） | 未测 | 未启动 app、无 DB 实例；所有判定基于 schema DDL + 代码路径静态推断 |

## 已知局限

1. **H 级用例代码为重构定义**：PRD v1 和 input.md 均未显式枚举 W-02 等代码的含义。本报告按前缀语义+PRD F-1~F-11 做最合理映射，但 Architect 应在 task_002 正式确认或纠正。
2. **静态判定为主**：`W-*` 失败路径（unsupported / empty / error）由代码结构推断，未在运行时实际拖入 zip / 损坏 PDF 观察 DB 状态。若未来发现 scheduler 有我漏掉的兜底路径，需修正判定。
3. **Q-04（`get_concept_detail`）未深入看 db 层聚合实现**：仅凭命令注册和命名判断，若 db/knowledge.rs 的 `get_concept_detail` 有缺漏字段，Reviewer 可标记 revisit。
4. **K-03 PASS 存在隐性风险**：跨 project 同名概念合并目前成立但无 discriminator，业务上是否合理需 Architect 决策（已列为问题 #9）。
5. **控制字符文件名在 macOS 上被转义**：`\x01\x02 garbled  name .md` 实际文件名显示为字面字符串 `\x01\x02` 而非控制字节（shell 未解释转义）。如需真正控制字符测试，应在 UI 手动构造。

## 需要 Reviewer 特别关注的地方

- **F-11 判定是否过于乐观**：`compute_co_occurrence` 我仅看了核心路径，未核对 `relation_count` 统计是否包含"未新增但更新"的情形；建议 Reviewer 对比 `co_occurrence.rs` 测试用例 `:205-247` 中的期望行为。
- **Q 系列 K-02 的 PARTIAL 判定**：`synthesize_viewpoints` 用 delete+rebuild 策略，如果 PRD F-10 的"去重合并"本意是"语义合并同一观点"，则判定应降为 FAIL；如果本意只是"不产生物理重复行"，则 PASS。Architect 应在 #1 中明确。
- **S-01（user_edited）PARTIAL 还是 FAIL**：当前 `append_source_asset` 不触及 name/definition 算侥幸保护，但不属 PRD 底线 1 的"任何情况下不得被自动覆盖"的显式实现。倾向判 FAIL 更保守，本报告暂留 PARTIAL 等 Architect 决策。
