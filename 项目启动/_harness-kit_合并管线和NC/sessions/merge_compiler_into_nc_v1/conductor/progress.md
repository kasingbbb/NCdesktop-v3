# Conductor Progress

## 当前状态

- **STATE**: `TASK_RUNNING`（PM 已对 ESCALATE 做出决策，进入 Week 1 主循环）
- **当前 Task**: Week 1 第一波并发 batch（task_002 / task_003 / task_017，worktree 隔离）
- **更新时间**: 2026-05-27 (PM 决策方案 A + 独立 Key UI 表单，进入 TASK_RUNNING)
- **PM 验收状态**: PRD 通过；ESCALATE 已决策（方案 A：放宽 P0 到 35 工作日）；KC LLM Key 偏好已定（Settings UI 中独立正式表单）

---

## 上游产出（输入）

| 文件 | 路径 |
|------|------|
| Session Context | [../session_context.md](../session_context.md) |
| PRD | [../prd/merge_compiler_into_nc_prd_v1.md](../prd/merge_compiler_into_nc_prd_v1.md) |
| Debate Conclusions | [../debate/session_001/debate_conclusions.md](../debate/session_001/debate_conclusions.md) |
| Integration Surface & Risks | [../intel/integration_surface_and_risks.md](../intel/integration_surface_and_risks.md) |
| KC API Integration Proposal | [../intel/kc_api_integration_proposal.md](../intel/kc_api_integration_proposal.md) |
| NCdesktop Pipeline 调研 | [../intel/ncdesktop_pipeline.md](../intel/ncdesktop_pipeline.md) |
| KnowledgeCompiler 调研 | [../intel/knowledge_compiler.md](../intel/knowledge_compiler.md) |

---

## 已完成 Tasks

| Task ID | 描述 | 产出 | Review 分 | Git Commit |
|---------|------|------|----------|------------|
| task_001_architect | Architect 阶段：4 项调研 + 技术方案 + 28 task 拆解 | `tasks/task_001_architect/output.md` | N/A | — |
| **task_002_schema_migration** | DB v18 迁移：extracted_content + conversion_meta 新增 6 列 | `tasks/task_002_schema_migration/{output,review_scorecard}.md` | **4.85/5 PASS** | `d8143706` |
| **task_003_failure_code_kc_variants** | failure_code.rs 追加 5 个 EKc* 变体 + as_str 映射 | `tasks/task_003_failure_code_kc_variants/{output,review_scorecard}.md` | **4.85/5 PASS** | `2c3389bd` |
| **task_017_frontmatter_renderer_dep** | 前端 frontmatter 渲染依赖 + 2 个展示组件骨架（分支 B 补能力） | `tasks/task_017_frontmatter_renderer_dep/{output,review_scorecard}.md` | **4.70/5 PASS** | `ab4ac4d4` |
| **task_005_kc_module_skeleton** | kc/ 模块骨架：7 子模块 + KcCallError/KcFallbackReason + 6→5 FailureCode 映射 + lib.rs 注册 | `tasks/task_005_kc_module_skeleton/{output,review_scorecard}.md` | **5.00/5 PASS** | `9fe2cadb` |
| **task_021_visual_badge** | KcStatusBadge 4 态（a11y 完备 + 防色盲） | `tasks/task_021_visual_badge/{output,review_scorecard}.md` | **3.95/5 PASS** | `0c6aee0d` |
| **task_004_settings_kc_block** | KcSettings 7 字段实装 + Debug mask + load helpers | `tasks/task_004_settings_kc_block/{output,review_scorecard}.md` | **4.85/5 PASS** | `050e4f5a` |
| **task_006_mock_kc_server** | wiremock dev-dep + MockKcServer 7 scenarios + 4 integration tests | `tasks/task_006_mock_kc_server/{output,review_scorecard}.md` | **4.85/5 PASS** | `c48fd46c` |

**Week 1 收官**：7/7 完工，均分 **4.61/5**，0 BLOCKER，0 FIX 反弹。进入 Week 2（KC 客户端 + 进程管理 + 注入框架）。

| Task | 描述 | 评分（初评→FIX 后）| Commit |
|------|------|-------------------|--------|
| **task_010_kc_settings_loader** | build_env_vars / mask_secrets / log_with_mask / save_settings + Key 安全三场景 verify | **4.75/5 PASS**（一次过）| `6cd4b8e4` |
| **task_007_kc_client** | KcClient + PortProvider + Semaphore(1) + 60s 超时（FIX 修 PortProvider 单点定义） | **4.15 → 4.70/5 PASS** | `e3e1335e` |
| **task_008_kc_process_manager** | KcProcessManager 启停/健康/崩溃/RAII（FIX 修 mask 漏屏 + 冷却期数学冗余 + 措辞）| **3.85 → 4.70/5 PASS** | `90df2205` |
| **task_009_kc_lifecycle_integration** | lib.rs 注入 + Window close 钩子 + TD-5 mask 命名统一（mask_secrets_by_keys / by_prefix） + mod.rs 14 类型导出 | **4.70/5 PASS** | `9f213431` |

**Week 2 收官**：4/4 完工，均分 **4.71/5**（task_007/008 各 1 轮 FIX 修后均 4.70；task_009/010 一次过）；TD-5 关闭；下游 task_011/012/020 解锁。进入 Week 3（enrichment + scheduler 注入 + frontmatter writer + 防御层）。

### Week 3 已落地

| Task | 描述 | 评分 | Commit |
|------|------|------|--------|
| **task_014_kc_outputstage_defense** | OutputStage 三层防御（cwd 隔离 + 扫描清理）+ 10 单测 + mode helper | **4.58/5 PASS**（0 BLOCKER / 0 MAJOR / 4 MINOR） | `05be3a5f` |
| **task_011_kc_enrichment_module** | enrich() async + resolve_outcome 纯函数 + 5 类失败映射 + 7 集成测试（fix-tail 把 mock_kc.start_with_unavailable 改为 TcpListener bind+drop 真不可达）；21 lib 单测含 failure_code 字面守护 | **4.80/5 PASS**（最高分，0 BLOCKER / 0 MAJOR / 4 MINOR） | `fdb538f1` |
| **task_015_db_kc_fields_writer** | db_update_kc_fields + ConversionMetaRow Default 扩展（向后兼容）+ db_read_kc_status + 7 单测（含 u64::MAX saturating 边界 + camelCase 前端契约保护） | **4.63/5 PASS**（0 BLOCKER / 0 MAJOR / 4 MINOR） | `79ab9772` |
| **task_013_frontmatter_writer** | build_kc_frontmatter（手写 YAML 序列化，无 serde_yaml 依赖）+ 15 字段（5 NC + 10 KC）+ 双引号 escape + block scalar + 11 单测 + field_order_is_stable 不变量守护 | **4.70/5 PASS**（0 BLOCKER / 0 MAJOR / 5 MINOR） | `b70439ad` |
| **task_012_scheduler_injection** | save_and_materialize 注入 KC enrichment + resolve_outcome + db_update_kc_fields + write_kc_conversion_meta（注入 12 行 / 25 行预算）；save_and_materialize sync → async（2 调用方加 .await）；5 新测试（success / disabled / partial / markdown 跳过 / parse_failure_code 守护） | **4.58/5 PASS**（含 MAJOR-1：TD-3 仍 open，由 task_015b 关闭） | `d7f5fac5` |

**Week 3 完整收官**：5/5 全 commit + 全 Reviewer PASS；lib 测试 489 → **512 全 PASS**（净增 23 测试，**0 退化**）；5 个 Reviewer 均分 **4.66/5**（最高 4.80 task_011，最低 4.58 task_012/014）；0 BLOCKER / 1 MAJOR（TD-3 待 task_015b 关）/ 21 MINOR（全可接受）。

### Week 3 → Week 4 切换

进入 Week 4（前端 UI + retrigger + 队列，5 工作日）+ task_015b 关 TD-3。

### Week 4 / TD 关闭 已落地

| Task | 描述 | 评分 | Commit |
|------|------|------|--------|
| **task_015b_close_td3** | 关 TD-3：canonical parse_failure_code 加 5 KC arm + pub(crate)；删 scheduler local mini-parser；守护测试用 `FailureCode::EKc*::as_str()` round-trip 取字面（机器消除"第二份字面源"）| **4.92/5 PASS**（最高分，TD-3 彻底关闭）| `f19239c2` |
| **task_020_kc_commands** | 3 个 Tauri command（get_kc_health / restart_kc_process / set_kc_settings）+ Key 变化自动 spawn restart（bool 不触发）+ KcHealthStatusDto 5 字段 + KcSettingsPayload 8 字段 + 14 新单测 + 前端 typed wrapper | **4.90/5 PASS**（测试 14 vs AC 要求 4）| `100e66f6` |
| **task_026_retrigger_kc_force** | retrigger_extraction 加 force_kc_refresh Option（默认 false 向后兼容）+ clear_kc_enriched 纯函数 + ExtractedContentRow.kc_enriched 字段 + Inspector "重新增强"按钮（assetType!=md && kcEnriched∈{true,partial} 才显示）+ 2 后端 + 7 前端测试 | **4.60/5 PASS**（含事故救援 cherry-pick）| `7d1cb0db`（cherry-pick）|

**Wave C 事故说明**：task_026 Dev agent 在隔离 worktree `claude/tender-bhaskara-e84070`（HEAD `1cb5307e`, base `1f04904b`）提交，缺 Week 3 全部代码 → baseline 仅 413。Conductor 检测到 commit 不在 master git log + 测试数字异常（415 vs 528）→ cherry-pick `1cb5307e` 到 `feat/windows-unit-13-cloud-ai`（auto-merge 0 冲突）→ 新 commit `7d1cb0db`，cargo test 530/530 PASS（baseline 528 + 2 新增），0 退化。

**Wave C 后整体测试**：lib **530/530 PASS**（从 489 净增 41 测试）；0 BLOCKER；TD-3 已关；2 个 Reviewer 等跑。

### Week 4 Wave E 已落地

| Task | 描述 | 评分 | Commit |
|------|------|------|--------|
| **task_018_inspector_render** | InspectorExtraction parseFrontmatter + react-markdown body + kc_enriched 字面映射；TD-2（FrontmatterTagsView/SummaryView a11y 补齐）+ TD-4（翻译层归属本地）双关闭；8 新测试 | _Reviewer 待跑_ | `6fa384e0` |
| **task_025_queue_status_toast** | KcQueueStore（Zustand）+ DropzoneApp toast；后端补 task_011 enrichment.rs emit `notecapt/asset-kc-queued`（3 段 32 行新增 + 2 单测 + 2 helper）；前端 18 新测试 | _Reviewer 待跑_ | `acd5cffb` |

**Wave E 后整体测试**：cargo lib **532/532 PASS**（baseline 530 + task_025 后端 2）；前端 vitest 414 PASS / 44 失败为 baseline（0 退化）；tsc 0 error。所有 dev 已显式 `cd` 到 master 仓库根 + git rev-parse 验证，无 worktree 事故复发。

### Week 4 Wave F + G 完整收官

| Task | 描述 | 评分 | Commit |
|------|------|------|--------|
| task_016 settings form | KcSettingsForm.tsx 完整实装（含 PM ESCALATE 补丁 AC-7 测试连通性 + ai_enabled 未知守护）| **4.65/5 PASS** | `7ab0efda` |
| task_019 doc viewer | DocumentViewer react-markdown + TD-4 helper 抽取（kcEnrichedLabel.ts 单源）| **4.83/5 PASS** | `e3c2ae02` |
| task_020b ai_enabled DTO | KcHealthStatusDto 补 ai_enabled 字段（task_016 漏字段触发，实时拉 /health 解析）| **4.72/5 PASS** | `a33192c6` |

### Week 5 测试三件套 + Reviewer 全收

| Task | 描述 | 评分 | Commit |
|------|------|------|--------|
| task_022 failure inject | 5 类失败注入 + DB 双断言（status='extracted' + failure_code 字面）+ partial 路径 tags_source='rule_only' | **4.83/5 PASS** | `f703da4f` |
| task_023 e2e integration | 4 后端 + 2 前端 e2e（真链路 + mock KC + 同义 DB helper + 1 个 guard 防 drift）| **4.535/5 PASS** | `c0ebde15` |
| task_024 perf benchmark | 自实现 Nearest-Rank P50/95/99，无 criterion；KC ingest P95 0.638ms / main_pipeline P95 1.056ms（mock 场景，PRD §4.1 阈值余量 7800×）| **4.58/5 PASS** | `75045eb2` |

**Reviewer 三人独立汇聚发现**：`kc_persist_resolved_with_conn` 在 scheduler 私有 + 3 处 test crate 字面复刻 = DRY 警示。共识起 task_027b。

### Week 6 + DRY follow-up 完整收官

| Task | 描述 | 评分 | Commit |
|------|------|------|--------|
| task_027b DRY fix | canonical kc_persist_resolved_with_conn 升 `pub fn + #[doc(hidden)]`（修正路径 A，integration test 是 external crate）；3 处 helper 全删；净 -262 行；删 1 guard 测试（接力到 lib 内 save_and_materialize_with_kc_* 3 测试）| **4.87/5 PASS**（DRY 闭环彻底）| `1df063f0` |
| task_027 DMG packaging | prepare-embedded-kc-runtime.sh（~210 行）+ kc-requirements.txt（13 pinned + 7 红线）+ 静态测试套件 14/15 PASS；manifest 扩展 6 字段（runtime_id/version/commit_sha/python_version/venv_size_bytes/build_timestamp）；红线 grep 0 hit；macOS/Linux 双兼容 | **4.85/5 PASS** | `542ef912` |
| task_028 kc-venv optimize | optimize-kc-venv.sh（~200 行）6 步剥离（RECORD/.pyi/tests/dist-info docs/可选 strip-bin/体积报告）+ 静态测试套件 **24/25 PASS**（含 jieba/dict.txt + jieba/tests/ 红线守护）+ prepare 主脚本 opt-in hook（PREP_KC_OPTIMIZE=1）| _Reviewer 待跑_ | `4f1380de` |

### 完整项目战绩快照（v1.0 上线前夜）

- **总 commits**：23 个 KC 相关（task_002 → task_028）on `feat/windows-unit-13-cloud-ai`，0 BLOCKER
- **测试**：lib **537/537 PASS** + integration **12/12**（5 failure_injection + 4 e2e + 3 perf_smoke）+ 前端 **414+ PASS**（KC 相关域 35/35）+ shell 静态测试 **38/40 PASS**（14 prepare + 24 optimize）
- **Reviewer**：21 个全 PASS，均分 **4.68/5**（最高 4.92 task_015b TD-3，最低 4.535 task_023 e2e）
- **关闭技术债**：TD-3（parse_failure_code 单源化）+ TD-2（FrontmatterTagsView a11y）+ TD-4（kc_enriched 翻译层 DRY helper）+ Reviewer 共识 DRY（kc_persist_resolved 单源化）
- **救援 1 次**：task_026 worktree 事故（cherry-pick 修复，0 冲突）
- **PM 决策已落**：方案 A（35 工作日 P0）+ KC LLM Key 独立表单（task_016 落地）+ ESCALATE 补丁 AC-7 测试连通性（落地 + Reviewer PASS）

### v1.0 上线决策依赖（PM 真机 + 人工）

1. **task_028 Reviewer**：仅静态评审待跑（PASS 预期）
2. **真机 DMG 打包验证**（PM 必跑）：
   - 跑 `prepare-embedded-kc-runtime.sh` 注入真 KC venv（~150MB）
   - 跑 `optimize-kc-venv.sh` 剥离至 ~80MB
   - `pnpm tauri build` 完整打包 → DMG 增量 < 100MB
   - 启动 NCdesktop → KC 子进程 ready → curl /api/v1/health 验证 ai_enabled
3. **task_029 Acceptance Report**（PM 人工 50 篇语料标注，本质需要主观评分）：
   - 5 种来源（PDF教科书 / DOCX / 网页 / TXT笔记 / Markdown）× 10 篇 = 50 篇
   - 评分维度：标签语义命中率 / 问答来源命中率 / 整体可读性提升感
   - 结论：PASS / FAIL / CONDITIONAL PASS

---

## 当前 Task 详情

- **Task ID**: `task_001_architect`
- **描述**: Architect 阶段
- **状态**: `DONE`
- **交付物路径**: `tasks/task_001_architect/output.md`
- **关键产出**：
  - V1 调研结论：**分支 B**（NC 前端无 frontmatter 渲染能力）—— 详见 output.md §"4 项调研结论 / V1 调研"
  - V2 调研结论：macOS 系统默认 Quick Look **不能完整渲染** KC v6 增强 MD（plain-text only）—— 详见 output.md §"V2 调研"
  - kc-venv 体积实测：**当前 206 MB，剥离后 ~76 MB，DMG 增量 ~81 MB**（优于 PRD R1 ~100-200MB 预估）—— 详见 output.md §"kc-venv 体积实测"
  - 关键路径：**30 工作日**（贴 PRD 上限）—— 详见 output.md §"Task DAG + 关键路径长度"
  - 12 条 ADR 已落地（ADR-001 ~ ADR-012）
  - 28 个 task input.md 已写完

---

## ESCALATE 决策栏（Conductor 必须等 PM）

### 触发原因
V1 调研 = 分支 B → F12 工作量 +1.5d → P0 总 30.5-31d 超 PRD §"不可妥协的技术底线"#8 的 30 工作日上限。

### PM 必须做的决策

请选择 1 个方案（Architect 推荐方案 A）：

- [ ] **方案 A：放宽 P0 时间盒到 35 工作日（Architect 推荐）**
  - 优点：F12 完整交付（标签 + 摘要可见，价值主张完整兑现），5d buffer 处理 KC-MOD 延误 + Reviewer FIX
  - 代价：上线时间 +5 工作日（约 +1 自然周）

- [ ] **方案 B：保留 30 工作日上限，砍 F12 frontmatter 解析**
  - 不再做 frontmatter 解析（task_017 / 018 / 019 大幅简化）
  - NC 前端不解析 .md 而从 DB 字段拉 ai_tags / ai_summary（额外字段 v18 已有）
  - 优点：守 30d 上限
  - 代价：.md 与 NC DB 双源；历史回填（P1）复杂度上升

- [ ] **方案 C：重新组织 Debate（不推荐）**
  - 触发 Debate Layer 3 / 4 修订
  - 代价：增加 1-2 周决策周期

**Architect 默认假设**：所有 task input.md 按**方案 A** 编写（task_017/018/019 实装 frontmatter 解析）。PM 选 B 时需要由 Conductor 主导对应 input.md 改写。

---

## 待执行 Task 队列（按 PRD §6.4 6 周节奏 + DAG 排序）

> **状态约定**：`PENDING` = 等待依赖；`READY` = 可立刻启；`RUNNING` = 当前 Worker 在做；`DONE` = 已完成；`BLOCKED` = 阻塞

### Week 1（KC-MOD 软截止 + 无依赖子集，5 工作日）

> **可并行启动的 task 组**（无相互依赖）：002 / 003 / 004 / 005 / 017 / 021；005 完成后 006 可启

- [ ] **task_002_schema_migration** — DB v18 迁移（extracted_content + conversion_meta 新增 6 列）— S，0.5d，**READY**
- [ ] **task_003_failure_code_kc_variants** — failure_code.rs 追加 5 个 E_KC_* 枚举 — S，0.2d，**READY**
- [ ] **task_004_settings_kc_block** — kc.* setting key 常量 + DB 默认值 — S，0.5d，**READY**
- [ ] **task_005_kc_module_skeleton** — kc/ 模块骨架 + lib.rs 注册 + 占位类型 — S，0.3d，**READY**
- [ ] **task_006_mock_kc_server** — wiremock + MockKcServer helper（6 个预设场景）— M，1.5d，PENDING（依赖 task_005）
- [ ] **task_017_frontmatter_renderer_dep** — 装 js-yaml + react-markdown + remark-gfm + parseFrontmatter util + 两个展示组件 — S，0.5d，**READY**
- [ ] **task_021_visual_badge** — F13 KcStatusBadge 4 态视觉标识 — S，0.5d，PENDING（依赖 task_005 中类型）

### Week 2（KC 子进程 + 客户端 + 注入框架，5 工作日）

- [ ] **task_007_kc_client** — reqwest async + Semaphore(1) + 60s 超时 + 错误分类 — M，1d，PENDING（依赖 005/006）
- [ ] **task_008_kc_process_manager** — KcProcessManager 启停/健康/崩溃/RAII — M，3d，PENDING（依赖 005）— **关键路径起点**
- [ ] **task_010_kc_settings_loader** — KcSettings 加载 + env 变量 + Key mask — S，0.5d，PENDING（依赖 004/005）
- [ ] **task_009_kc_lifecycle_integration** — lib.rs 注入 KC 启停 + Window close 钩子 — S，1d，PENDING（依赖 007/008）

### Week 3（enrichment + scheduler 注入 + frontmatter writer + 防御层，5 工作日）

- [ ] **task_011_kc_enrichment_module** — enrich + resolve_outcome 纯函数 + 5 类失败映射 — M，2d，PENDING（依赖 007/010）
- [ ] **task_012_scheduler_injection** — scheduler.rs::save_and_materialize 注入 KC step — M，1d，PENDING（依赖 011）
- [ ] **task_013_frontmatter_writer** — build_kc_frontmatter（NC schema + KC 扩展字段） — S，1d，PENDING（依赖 011）
- [ ] **task_014_kc_outputstage_defense** — 三层防御（cwd 隔离 + 扫描清理）— S，0.5d，PENDING（依赖 008）— 与 011/012/013 并行
- [ ] **task_015_db_kc_fields_writer** — db_update_kc_fields + conversion_meta KC 列写入 — S，0.5d，PENDING（依赖 002）— 与 011/012 并行

### Week 4（前端 UI + retrigger + 队列，5 工作日）

- [ ] **task_016_settings_form** — F11 KcSettingsForm.tsx 完整实装 — M，1.5d，PENDING（依赖 010）
- [ ] **task_018_inspector_render** — Inspector 接入 frontmatter 展示 — S，1d，PENDING（依赖 017）
- [ ] **task_019_doc_viewer_render** — DocumentViewer.TextContent → react-markdown — S，0.5d，PENDING（依赖 017）
- [ ] **task_020_kc_commands** — 3 个 Tauri command（health/restart/setSettings）— S，0.5d，PENDING（依赖 008/010）
- [ ] **task_026_retrigger_kc_force** — F14 retrigger_extraction force_kc_refresh 选项 + Inspector 按钮 — S，0.5d，PENDING（依赖 011/012）
- [ ] **task_025_queue_status_toast** — F15 拖拽队列 toast — S，1d，PENDING（依赖 011）

### Week 5（测试 + benchmark，5 工作日）

- [ ] **task_022_failure_injection_tests** — F19 5 类失败注入测试（必含 EKcUnavailable + EKcTimeout）— M，1.5d，PENDING（依赖 006/011/012）
- [ ] **task_023_e2e_integration_tests** — F20 拖入 → 全链路 e2e 测试 — M，2d，PENDING（依赖 012/013/018/019）
- [ ] **task_024_perf_benchmark** — F21 KC ingest P95 + 主链路 P95 — S，0.5d，PENDING（依赖 011/012）— 与 022 并行
- [ ] **task_027a_pre_release_fixes**（未单独编号，Week 5 buffer） — Reviewer FIX 反弹缓冲 — M，1d，PENDING

### Week 6（DMG + Acceptance，5 工作日）

- [ ] **task_027_dmg_packaging_kc** — F22 prepare-embedded-kc-runtime.sh + manifest 扩展 — M，3d，PENDING（依赖全部前序）
- [ ] **task_028_kc_venv_optimize** — F23 体积优化脚本（剥离 + 清理）— S，1d，PENDING（依赖 027）— 与 027 末尾并行
- [ ] **task_029_acceptance_report** — Acceptance Report（量化 + 5 类失败 + 性能 + 体积 + 50 篇语料）— M，1d，PENDING（依赖 028）

---

## 已知问题 / Blockers

- **BLOCKED_BY_PM**：ESCALATE 决策栏中 PM 需对方案 A/B/C 做出选择，方可启动 Week 1 task_002 ~ task_021 的实施
- **可能 BLOCKED_BY_KC_MOD**：Week 2-3 部分 task 依赖 KC-MOD-1/2/3/5 已落地（软截止 P0 启动后第 3 个工作日）；若 KC-MOD 延误，task_011/012 用 mock KC server 继续，Conductor 标 BLOCKED_BY_KC_MOD 标签

---

## 关键决策记录

| 时间戳 | 决策 |
|--------|------|
| 2026-05-27 | PRD 通过 PM 验收，进入 Conductor 阶段 |
| 2026-05-27 | 初始 STATE 设为 ARCHITECTURE（PRD 已固化 30 个功能项 + 5 级优先级 + 时间盒） |
| 2026-05-27 | **Architect 完成 4 项调研**：V1=分支B / V2=Quick Look 不可完整渲染 / kc-venv 剥离后 76MB / 关键路径 30d |
| 2026-05-27 | **Architect 触发 ESCALATE**（条件性遗留 C1），推荐方案 A（放宽 35d），等待 PM 决策 |
| 2026-05-27 | 28 个 task input.md 已落地，按方案 A 假设排序与工作量 |

---

## 状态转移日志

```
[2026-05-27 早] STATE: INIT → ARCHITECTURE | Task: task_001_architect | 原因: PRD PM 验收通过 | 风险: 低
[2026-05-27 晚] STATE: ARCHITECTURE → ARCHITECTURE_DONE | Task: task_001_architect 完成 | 原因: Architect 4 项调研 + 方案 + 28 task 全部产出；分支 B 触发 ESCALATE 待 PM 决策 | 风险: 中（ESCALATE 阻塞 task_002 启动）
[2026-05-27 夜] STATE: ARCHITECTURE_DONE → TASK_RUNNING | Task: Week1 batch1 (task_002/003/017) | 原因: PM 选方案 A（放宽 P0 到 35d）+ KC Key 独立 UI 表单 | 风险: 低
[2026-05-27 夜] STATE: TASK_RUNNING (Week 1 → Week 2) | Task: Week 2 batch1 (task_007/008/010) | 原因: Week 1 全 7 task 完工均分 4.61，0 BLOCKER 0 FIX 反弹；自主推进 Week 2 | 风险: 中（task_008 3d 大任务在 dev 单次会话边界）
```

---

## PM ESCALATE 决策记录（2026-05-27）

- **C1 决策**：**方案 A**（放宽 P0 时间盒到 35 工作日）。Architect 默认假设成立，task_017/018/019 按 frontmatter 解析方案推进，**所有 28 个 task input.md 无需改写**。
- **KC LLM Key 偏好**：**Settings UI 中独立正式表单**（智谱 + OpenAI 两个 Key 字段）。
  - task_004（kc.* setting 常量）已包含独立 Key 字段 ✅
  - task_016（KcSettingsForm.tsx）当前 input.md **未明确包含 Key 输入 UI**，Conductor 已记录 patch 待办，**在 Week 4 启动 task_016 前补丁 input.md**
- **task_016 patch 待办**：补充以下 AC：
  - 必须包含 `kc.zhipu_api_key` + `kc.openai_api_key` 两个独立输入字段
  - 必须 mask 显示（**** 风格）
  - 必须有"测试 Key 连通性"按钮（调 KC `/api/v1/health` 验证）
  - 必须显示当前 KC `/health` 返回的 `ai_enabled` 实际状态

---

## Conductor 累积异常计数器（隐性维护）

- 同类 FIX 重复次数: 0
- 连续低分 task 数: 0
- ESCALATE 次数: **1**（V1 分支 B → 触发 P0 时间盒超上限重 Debate）
- 单 task FIX 轮数最大值: 0
- **冗余实装模式**: 1（mask_secrets 三处独立实装——3 并发 dev 各想到要 mask，缺乏 conductor 协调）
- **单 task FIX 轮数**: 1（**task_008 第 1 轮 FIX**，3 个 MAJOR 待修；**task_007 第 1 轮 FIX**，1 MAJOR 待修）
- **🚨 同类 FIX 重复模式触发**: 2 个并发 task（task_007 + task_008）同时 FIX，且都是"接口未协调"类（PortProvider trait 单点定义错误 + mask 函数三处独立实装漏屏）。**触发 Conductor protocol "模式警告 → PM"**——建议 PM 审视 Architect 方案是否对"共享 helper / 跨模块 trait"做出更显式的单点定义指引

### Conductor 自我反思

并发启 3 个 dev 时未建立"共享 helper 谁负责"的协调机制，导致 mask_secrets 三处重复。**对策**：未来并发启 dev 前，在 prompt 中明确"通用 helper（如 mask / env build）由 XXX task 单一来源提供，其它 task **必须 import 而非自写**"。已应用于 task_009 input.md 待补条款。

---

## 技术债登记（Reviewer 上抛的治理性问题）

| ID | 来源 | 描述 | 决策 | 上抛时点 |
|----|------|------|------|---------|
| TD-1 | task_017 reviewer | 4 个新依赖用精确版本与项目其他 30 个 caret 不一致 | 本期保留精确 pin（KC 集成稳定性敏感），全项目 caret vs pin 统一治理推后到独立任务 | 已记录，不阻塞 |
| TD-2 | task_017 reviewer | FrontmatterTagsView / FrontmatterSummaryView 缺 a11y（role/aria-label） | task_018 接入 Inspector 时一并补齐（追加 AC 到 task_018 input.md） | 待 task_018 启动时 patch input.md |
| TD-3 | task_003 reviewer | `src/db/conversion_meta.rs:186 parse_failure_code()` 未扩展 5 个 KC 字面值 | task_011 enrichment 落地前必须补，否则 `get_conversion_state` 会误判 KC 失败为 LegacyUnverified | 阻塞 task_011，已添加为 task_011 前置条件 |
| TD-4 | task_021 reviewer | KcStatus 字面值 "success"/"failed" 与 task_017 parseFrontmatter "true"/"false" 跨语义层不对齐 | task_018/019 接入 Inspector/DocumentViewer 时由 NC enrichment 翻译层做 mapping（不同语义：UX 状态 vs YAML 字面值，不应在 task_021 处理） | 已添加为 task_018/019 前置条件 |
| TD-5 | task_010 + task_008 + task_007 reviewer 三重复核 | **【真相完整版】**三处 mask 函数实际是**三种不同语义**：(a) task_010 mask_secrets(msg, &KcSettings) 用已知 Key 精确子串替换；(b) task_007 client.rs mask_secrets 用前缀匹配（sk-/zhipu-/Bearer 等 11 个前缀）防 KC 服务端**未知 Key** 泄漏 —— **互补不冲突**；(c) task_008 mask_secret(&str) 用前缀启发但漏屏 dot 格式 Key + JSON + Debug **真实安全漏洞**。 | task_008 FIX 改用 task_010 mask_secrets；task_007 mask_secrets 保留（互补）但命名重复是 MINOR；task_009 lifecycle integration 时统一改名 mask_secrets_by_keys / mask_secrets_by_prefix 避免阅读混淆 | task_008 FIX in_progress, task_007 FIX 待启 |

---

## 条件性遗留（已显式处理）

- **C1（已触发）**: V1 调研结论 = 分支 B → 已在 Architect output.md §"ESCALATE 决策栏" 显式触发；progress.md 已标 BLOCKED_BY_PM；Architect 提供方案 A/B 备选 + 推荐 A

---

## 下一步（Conductor 内部判定）

1. **当前等待 PM 决策**（ESCALATE 方案 A/B/C 选择）
2. **PM 决策选 A**：Conductor STATE → TASK_RUNNING；按方案 A 启动 Week 1 task_002/003/004/005/017/021 并行（task_006 等 005 完成后 ~0.5d 跟进）
3. **PM 决策选 B**：Conductor 需要主导 task_017/018/019 改写（去掉 frontmatter 解析，前端直接从 DB 拉 KC 字段）；其余 task 不变
4. **PM 决策选 C**：暂停 Conductor，回到 Debate 阶段
