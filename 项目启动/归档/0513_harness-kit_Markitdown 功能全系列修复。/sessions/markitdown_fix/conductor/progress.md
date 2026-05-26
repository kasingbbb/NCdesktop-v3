# Conductor Progress

## 当前状态
STATE: DEVELOPING (双轨并行)
当前 Task: task_003 (T-C: venv shim) 待分发；task_000 (脱敏 SOP) dev-desensitize 后台跑
已完成: task_001 PASS 4.85/5；task_002 PASS 5.00/5；D-1/D-2/E-1/E-2 全部已清
更新时间: 2026-05-13

## 已完成 Tasks
- onboarding: 读取 harness-kit core docs（bootstrap / handoff_contracts / state_machine）
- session_context.md: 填写完毕（L 复杂度）
- debate/session_001: 4 层完整 Debate 完成
- prd v1.0: `sessions/markitdown_fix/prd/ncdesktop_markitdown_prd_v1.md` 含 Conductor 桥接摘要
- task_001_architect: 技术方案 `tasks/task_001_architect/output.md`（10 条 ADR + 模块拓扑 + 风险登记 + 17 task 清单）
- task input.md 全套（17 份）：task_000 + task_001..015 均符合 handoff_contracts §2 (Architect → Dev) 契约

## 当前 Task 详情
Task ID: 待 Conductor 在新会话中按依赖图分发 task_000 → task_001 → ...
描述: ARCHITECTURE 阶段已完成；下一步进入 DEVELOPING：先分发 task_000（Sprint 0 前置），完成后串行分发 task_001..006（打包链），错峰并行 task_008..010、task_011/014，最后 task_012/013、task_015。
状态: 待分发
交付物路径: `sessions/markitdown_fix/conductor/tasks/task_<id>/input.md`

## 待执行 Task 队列（拓扑，详见 task_001_architect/output.md §6）

打包链强制串行：
- task_000_sample_desensitization_sop  ← Sprint 0 前置
- task_001_prepare_embedded_python_rpath_verify  (T-A)
- task_002_markitdown_extras_pin_manifest        (T-B, 依赖 T-A)
- task_003_venv_shim_symlink_cold_boot           (T-C, 依赖 T-B)
- task_004_entitlements_reverse_sign             (T-D, 依赖 T-C)
- task_005_notarize_staple_gatekeeper            (T-E, 依赖 T-D)
- task_006_build_macos_dmg_integration           (T-F, 依赖 T-E)

可并行于打包链中段：
- task_008_error_codes_and_failure_code_migration   （可在 T-A..T-D 期间并行；task_007 / 009 / 010 / 011 / 014 的前置）
- task_007_runtime_manifest_self_check              （依赖 task_002 + task_008）
- task_009_scan_pdf_route_guard                     （依赖 task_008；与 010 可并行）
- task_010_audio_route_guard                        （依赖 task_008；与 009 可并行）
- task_011_preserve_vs_modify_matrix                （依赖 008/010）
- task_014_legacy_unverified_migration              （依赖 task_008）

验收/收尾：
- task_012_real_sample_matrix_ci_secret            （依赖 000/002/006/007/008/009/010）
- task_013_clean_vm_smoke_test                     （依赖 005/006/012）
- task_015_rollback_sop_manifest_schema_version    （依赖 006/012/013）

## 已知问题 / Blockers
- 无活跃 blocker
- 监控项（来自 Debate 脆弱性声明）：pbs 在 macOS 13.x dyld 兼容性、markitdown 0.1.5 epub 转换器是否本身有 bug、CI macOS runner 是否可用
- task_000 与 task_012 强依赖"脱敏负责人 ≠ 打包负责人"——分发前 Conductor 需向 PM 确认人选

## 关键决策记录
- [2026-05-13] 架构决断：MVP arm64-only DMG，universal2 推 P2（ADR-008）
- [2026-05-13] 打包链原子化：T-A/B/C/D/E/F 6 原子 task，依赖图强制串行
- [2026-05-13] 禁用 `codesign --deep`，改为逆序逐个签名（Apple TN3127 / ADR-004）
- [2026-05-13] 禁用 `venv --copies`，使用 symlink venv-shim（ADR-003）
- [2026-05-13] 真实样本必须脱敏 + AES-256 加密 + 私有 git-lfs + CI secret，脱敏负责人 ≠ 打包负责人（ADR-009）
- [2026-05-13] Scope 裁剪原则 #4：任何引入新分类器/启发式的 task 一律踢出 P0 进 P1（H6）
- [2026-05-13] 升级回填策略：旧 `conversion_meta.status=success & content=''` 标 `legacy_unverified`（ADR-007 / task_014）
- [2026-05-13] 扫描 pdf 路由用 mime + PDF 文件头 XObject 嗅探，禁文本字数启发式（ADR-006 / task_009）
- [2026-05-13] runtime-manifest 启动一次性自检，每次 extract 不重复探测（ADR-010 / task_007）

## 状态转移日志
[2026-05-13] STATE: INIT → DEBATE | 原因: 复杂度 L，启动 4 层完整 Debate | 风险: 中
[2026-05-13] STATE: DEBATE → PRD_READY | 原因: Debate 4 层共识达成，PRD v1.0 已落盘 | 风险: 低
[2026-05-13] STATE: PRD_READY → ARCHITECTURE | Task: task_001_architect | 原因: 加载 architect prompt 产出技术方案 + 17 task input.md，全部符合 handoff_contracts §2 | 风险: 低
[2026-05-13] DECISION: 组织约束妥协 | 原因: 单人团队 MVP 期采用方案 A（双 subagent 模拟 ADR-009 职责分离），仅适用于内部开发/未接触第三方真实用户文件阶段；正式发布前必须补齐真人双人复核 | 风险: 中（合规债务，已登记）
[2026-05-13] STATE: ARCHITECTURE → DEVELOPING | Task: task_001_prepare_embedded_python_rpath_verify (T-A) | 原因: 用户指定打包链优先，task_000 由 dev-desensitize subagent 在 task_001 进 REVIEW 时异步并启 | 风险: 低
[2026-05-13] STATE: DEVELOPING → REVIEW | Task: task_001 | 原因: dev-pack 交付 output.md（含 5/8 实测 PASS + 3 项 PENDING-USER-MACHINE + 2 项声明偏离）；待 Reviewer 评分卡 | 风险: 低（偏离均为 PBS 实测必需，非范围蔓延）
[2026-05-13] STATE: REVIEW → DEVELOPING | Task: task_001 → task_002 | 原因: Reviewer PASS 4.85/5；D-1（ADR-001 补强 rpath 字面前缀 5 项）+ D-2（verify-rpath.sh 增开发机路径前缀防御）已 conductor 直接 patch（Reviewer scorecard 为变更凭证）；smoke test 通过 | 风险: 低
[2026-05-13] ESCALATE: task_002 E-1/E-2 | 原因: AC-6 体积 289M > 200M (FAIL) + AC-3 imports 字面 docx 与 markitdown[docx] 真实依赖 mammoth 不符；用户裁决：E-1 阈值升 300M 推迟裁剪到 task_006/011；E-2 imports 改 mammoth + 删 python-docx；ADR-010 + task_002 input.md 已同步补强 | 风险: 中（magika 三巨头未来 task 还需处理）
[2026-05-13] STATE: DEVELOPING → REVIEW | Task: task_002 | 原因: dev-pack-fix 完成 M-1..M-4 + 自测重跑（7 imports OK / du 289M < 300M / verify-manifest exit 0），output.md 追加修订记录段 | 风险: 低
[2026-05-13] PARALLEL: task_000 启动 | 原因: 按方案 A 异步并启 dev-desensitize subagent；与打包链代码区无交集；task_012 前置依赖前移 | 风险: 低
[2026-05-13] STATE: REVIEW → DEVELOPING | Task: task_002 → task_003 | 原因: Reviewer PASS 5.00/5（满分）；8 字段 manifest 完整、7 imports 字面对齐、红线全过、无越权 | 风险: 低
[2026-05-13] PATCH: task_003 input.md + task_002 input.md AC-2 字面 docx → mammoth | 原因: E-2 修订一致性回扫；后续 task input.md 同步对齐 ADR-010 补强 | 风险: 低
[2026-05-13] STATE: DEVELOPING → REVIEW | Task: task_003 | 原因: dev-pack 交付 output.md（6/6 自测 PASS + 1 macOS VM PENDING）；2 项 minor Reviewer 关注点 | 风险: 低
[2026-05-13] STATE: DEVELOPING → REVIEW | Task: task_000 | 原因: dev-desensitize 交付 output.md（9 PASS + 5 PENDING 多为 AC-4 入库前置不可执行）；2 项 Reviewer 关注（forbid regex 范围 / gitignore 广度）可能升级 | 风险: 低
[2026-05-13] STATE: REVIEW → FIXING | Task: task_003 | 原因: Reviewer 判决 FIX (4.35/5)；MAJOR-1 `verify-venv-shim.sh:81` 透传 HOME 偏离 AC-3 字面 `env -i PATH=/usr/bin:/bin`，需删 HOME 或改 `-E -s` 标志 + user site stub 负向测试；scorecard.md 已落 | 风险: 低
[2026-05-13] CONDUCTOR-ERR | Task: task_003 | 原因: Reviewer 重启时我误描任务范围为"子线程+stderr 诊断"（实为 task_003=symlink venv-shim+冷启 imports 探针，ADR-003 非 ADR-007）；Reviewer 已用 input.md 真相源纠偏不影响判决；后续 Reviewer prompt 必须先复读 input.md 再写范围 | 风险: 低
[2026-05-13] STATE: REVIEW → PASS | Task: task_000 | 原因: Reviewer 判决 PASS (4.20/5)；三项红线全部合规；2 项关注点均不阻塞（关注 1 PASS、关注 2 MINOR 不强制）；5 项 PENDING 全部合理；scorecard.md 由 Conductor 代落盘（Reviewer Write 被拒） | 风险: 极低
[2026-05-13] TOOL-PERM-ISSUE | Task: task_003 + task_000 | 原因: 两个 Reviewer 子代理 Edit/Write/Bash(touch/redirect) 工具被会话权限拒绝；task_003 scorecard 由前一 Reviewer 实例已落盘所以无影响，task_000 scorecard 已由 Conductor 代落盘；后续 Reviewer 启动前可考虑显式提示子代理仅需读权限 | 风险: 低
[2026-05-13] FIX-DISPATCH | Task: task_003 | 原因: dev-pack 修复轮子代理启动，修复 MAJOR-1（删 verify-venv-shim.sh:81 的 HOME 透传，方案 A=`-E -s` 或方案 B=`HOME=/var/empty` 任选）+ 新增 user site stub 负向测试 + 重跑 6 项自测矩阵 + 追加 output.md FIX-LOG 段 | 风险: 低
[2026-05-13] STATE: FIXING → REVIEW-R2 | Task: task_003 | 原因: dev-pack FIX 交付（方案 A=-E -s，决定性对照实测旧形式实抛 ImportError 验证修复闭合；6/6 自测重跑 PASS；仅改 verify-venv-shim.sh 一文件未触越权区）；Reviewer R2 复审轮启动 | 风险: 低
[2026-05-13] STATE: REVIEW-R2 → PASS | Task: task_003 | 原因: Reviewer R2 判决 PASS (4.85/5，功能 4→5、安全 4→5)；A/B/C/D/E 五项全闭合（HOME 透传删除 + -E -s + stub 负向测试 trap 清理 + 仅 verify-venv-shim.sh 一文件 + 无 AC 回归）；scorecard R2 段已追加 | 风险: 极低
[2026-05-13] STATE: ARCHITECTURE → DEVELOPING | Task: task_004 (T-D 逆序签名) + task_008 (FailureCode 枚举) | 原因: task_001/002/003 全 PASS 解锁 task_004；task_008 无依赖；两 task 修改文件零交集（task_004=scripts/* + tauri.conf.json，task_008=Rust extraction/db + 前端 i18n）；并行启 dev-pack + dev 两子代理 | 风险: 低
[2026-05-13] STATE: DEVELOPING → REVIEW | Task: task_004 | 原因: dev-pack 交付 output.md（6 AC mock PASS + 1 真签名 PENDING-USER-MACHINE 合理推迟 task_005/013）；4 项 Reviewer 关注（grep-gate vs verify-deep 字面冲突 / ad-hoc 分支整体删除 / env var 兼容 / sort 算法 NUL 安全）；3 文件改动未越权 | 风险: 低
[2026-05-13] STATE: DEVELOPING → REVIEW | Task: task_008 | 原因: dev 交付 output.md（AC-1~6 PASS + 24 单测全过 + cargo check 通过）；6 文件改动未越权 + 未触 audio_asr_iflytek.rs（PRD 底线 #4）；4 项 Reviewer 关注（classify_output 边界 / image fallback 字段位置 / update_failure_code 锚定策略 / V12 PRAGMA 幂等）；12 个 pre-existing db::knowledge/co_occurrence 失败已 spawn_task 分流 | 风险: 低
[2026-05-13] STATE: REVIEW → PASS | Task: task_004 | 原因: Reviewer 判决 PASS (4.65/5)；4 项关注点全过（grep-gate 规避接受 + ad-hoc 删除合理 + env var 兼容 + sort MINOR 不阻塞）；AC-1~6 PASS + AC-7 真签端到端 PENDING 合理（task_005/006/013 接力）；红线四项全过；scorecard.md 已落盘 | 风险: 极低
[2026-05-13] STATE: ARCHITECTURE → DEVELOPING | Task: task_005 (T-E 公证+钉合+Gatekeeper) | 原因: task_004 PASS 解锁；与 task_008 Reviewer 无冲突（task_005=scripts/notarize.sh+build-macos-dmg.sh+CI workflow，task_008=Rust+i18n）；dev-pack 子代理启动；AC-3/4 真 VM 必 PENDING（task_013 接力） | 风险: 低
[2026-05-13] STATE: REVIEW → PASS | Task: task_008 | 原因: Reviewer 判决 PASS (4.90/5)；4 关注点全过（90s 边界 / image fallback / update_failure_code 锚定 / V12 PRAGMA 守卫）；AC-1~6 PASS；红线 6 项全过；scorecard.md 已落盘 | 风险: 极低
[2026-05-13] DISCOVERY | Reviewer 副产物: audio_asr_iflytek.rs 工作树 dirty 实际来自 task_014 Fix-A3（diff 注释字面证据），非 task_008 引入；PRD 底线 #4"严禁改 audio"指业务功能不动，task_014 注释级修订未越界；可在 task_014 input.md 中正式登记该 patch | 风险: 低
[2026-05-13] UNLOCK | Tasks 解锁: task_007 (manifest self-check, 依赖 task_002✅ + task_008✅) / task_009 (scan PDF guard, 依赖 task_008✅) / task_010 (audio route guard, 依赖 task_008✅) | task_011 仍阻塞 task_010；task_006 仍阻塞 task_005
[2026-05-13] STATE: DEVELOPING → REVIEW | Task: task_005 | 原因: dev-pack 交付 output.md（AC-1/2/5/6 PASS + AC-3/4 PENDING-CLEAN-VM 合理 task_013 接力）；secret 不泄漏 T8 三 sentinel 0 行验证；3 文件改动（notarize.sh 新建 + build-macos-dmg.sh 28+/22- + notarize-dmg.yml 新建）未越权 | 风险: 低
[2026-05-13] STATE: ARCHITECTURE → DEVELOPING | Task: task_007 (runtime manifest self-check) | 原因: task_002+008 PASS 解锁；与 task_005 Reviewer 零冲突（007=Rust extraction 新增模块，005=mac 脚本+CI workflow）；dev 子代理启动 | 风险: 低
[2026-05-13] MODE | Conductor 自主推进模式（用户授权 2026-05-13 markitdown_fix session）：逐 task 自主启 dev/Reviewer，简单/独立任务可并行，不每步问询；已存 feedback_conductor_autonomy.md
[2026-05-13] STATE: REVIEW → PASS | Task: task_005 | 原因: Reviewer 判决 PASS (4.65/5)；AC-1/2/5/6 + 4 项关注点全过；AC-3/4 PENDING-CLEAN-VM 合理 task_013 接力；3 MINOR 不阻塞；红线四项全过；scorecard 已落盘 | 后置: PM/Owner 注入 NOTARY_KEY_ID/ISSUER_ID/P8_BASE64 三 secret | 风险: 极低
[2026-05-13] SIDE-FIX | concepts 基表 V13 补建 | 原因: 用户启动 spawn_task 修 pre-existing 12 fail；db/knowledge.rs + co_occurrence.rs INSERT/SELECT 字段反推 4 张表 schema（concepts + concept_viewpoints/cases/extensions）；新增 v13_concepts_base_tables，dispatcher 注册 < 13 守卫；task_008 写的 3 个 v12 断言同步升至 13（保留函数名 v12 因核心契约不变）；cargo test --lib 全 PASS（无回归 181/0） | 风险: 低
[2026-05-13] STATE: ARCHITECTURE → DEVELOPING | Task: task_006 (T-F DMG 集成打包) | 原因: task_001..005 全 PASS 解锁；与 task_007 dev (Rust extraction) 零文件冲突（task_006=scripts/build-macos-dmg.sh+dist/SHA+可能 CI workflow，task_007=Rust）；dev-pack 子代理启动；AC-5 CI 可能 PENDING（无 macOS runner） | 风险: 低
[2026-05-13] STATE: DEVELOPING → REVIEW | Task: task_007 | 原因: dev 交付 runtime_check.rs+10 单测+lib.rs setup hook+AppState 缓存；AC-1/2/5/6 PASS；AC-3 调用方短路仅暴露基础设施未接入（dev 误读 prompt 红线，input.md 字面授权 scheduler.rs+markitdown::extract 入口）；AC-4 UI N/A 待 Reviewer 判定；6 项 import 归 ERuntimeMissing（task_008 枚举集合事实）；2 pre-existing fail 实为 spawn_task 路径已修 | 风险: 中（FIX 概率高） | Conductor-Err: prompt 写"禁改 markitdown.rs"过紧与 input.md AC-3 冲突，已在 Reviewer prompt 中纠偏
[2026-05-13] STATE: DEVELOPING → REVIEW | Task: task_006 | 原因: dev 交付 build-macos-dmg.sh 重写 ~340 行；AC-1/2/3/4/6 PASS（10 步顺序 + 体积门禁 du -sk KB 比较 mock 矩阵全过 + symlink 自检 + trap + SHA256）；AC-5 PENDING-CI 合理（无 macOS self-hosted runner，task_013 接力）；红线全过；prepare-*.sh git status M 实为 task_001/002 预存量与本 task 无关 | 风险: 低
[2026-05-13] STATE: REVIEW → FIXING | Task: task_007 | 原因: Reviewer 判 FIX (4.65/5)；AC-3 调用方短路（scheduler.rs+markitdown.rs）需补 ≤30 行+2 单测；裁决 2（6 项归 ERuntimeMissing）合理；AC-4 UI N/A 切独立前端 task；红线全过；cargo test 0 fail（V13 同步生效） | 风险: 低
[2026-05-13] STATE: ARCHITECTURE → DEVELOPING | Task: task_014 (legacy_unverified V14 migration) | 原因: 仅依赖 task_008✅；与 task_007 FIX 零冲突（014=db/migration.rs+conversion_meta.rs+前端 badge，007 FIX=scheduler.rs+markitdown.rs）；input.md AC-1 SQL 引用 status/content 列可能与 conversion_meta 实际 schema 不一致，已要求 dev 遇到时 ESCALATE 不硬塞 | 风险: 中（schema 不一致风险）
[2026-05-13] FIX-DISPATCH | Task: task_007 | 原因: dev-pack FIX 子代理启动，按 Reviewer 验证标准 4 项闭环：①scheduler+markitdown 各 1 处 RuntimeCheckState 引用；②2 新单测；③cargo test 181/0 不退步；④不触 classify_output/failure_code.rs 业务核心 | 风险: 低
[2026-05-13] STATE: REVIEW → PASS | Task: task_006 | 原因: Reviewer 判决 PASS (4.85/5)；AC-1/2/3/4/6 PASS + AC-5 PENDING-CI 合理（input.md 豁免条款 + task_013 接力）；4 红线全过（cp -RL 0 命中 / 触越权 0 / T-A..E 全在 / 体积门禁矩阵 5/5）；4 关注点全 OK（KB 阈值 307200=300MiB 合理 / 重复 staple 规避 / python3 健壮 / mktemp 安全）；3 MINOR 不阻塞；scorecard 已落盘 | 风险: 极低
[2026-05-13] ESCALATE→RESOLVE | Task: task_014 | 原因: R1 dev ESCALATE schema 不一致（conversion_meta 无 status/content 列）；Conductor 裁决方案 A + 最新一行约束（与 task_008 锚定策略一致）；AC-6 消费侧 filter 不归本 task（input.md 字面仅要求"标注"，已做；filter 改造 follow-up spawn_task）；input.md 末尾追加"AC-1 字面修订"段（保留原文为历史）；R2 dev 子代理启动 | 风险: 低
[2026-05-13] STATE: FIXING → REVIEW-R2 | Task: task_007 | 原因: dev-pack FIX 交付 195/0 PASS（baseline 181 +14 新测）；scheduler+markitdown 各短路（grep 21+9 命中）；4 短路单测全过；未触业务核心；仅改 models.rs+scheduler.rs+markitdown.rs 3 文件 | 风险: 低
[2026-05-13] DEV-R2-IMPL | Task: task_014 | 原因: system-reminder 显示 migration.rs 已含 V14 函数（含 tables_ready 守卫 + 方案 A 字面 SQL + 最新一行约束 + dispatcher 注册）；dev 通知应即将到达 | 风险: 低
[2026-05-13] STATE: DEVELOPING → REVIEW | Task: task_014 | 原因: R2 dev 交付 V14 + 三态查询 + 前端 badge；6 AC PASS（AC-6 仅标注，filter 改造 follow-up）；195/0 cargo test（含 10 新测）；SQL 字面与 Conductor 裁决段一致；8 文件改动未触越权（V1~V13/failure_code.rs/scheduler/markitdown/extractors/runtime_check/audio_asr_iflytek/commands/knowledge*/scripts 零触动） | 风险: 低
[2026-05-13] STATE: REVIEW-R2 → PASS | Task: task_007 | 原因: Reviewer R2 判决 PASS (5.00/5 满分，从 4.65 升)；A/B/C/D 四项验证标准全过（grep 22+9 / 4 短路单测 / 195+0 / 业务核心零触）；红线全过；scorecard R2 段已追加；功能正确性 4→5 因 AC-3 字面要求完整达成 | 风险: 极低
[2026-05-13] UNLOCK | Task: task_009 (scan PDF guard) + task_010 (audio route guard) | 解锁原因: task_007 PASS；两者均动 scheduler.rs 不同分支 + task_010 动 markitdown.rs（与 task_007 6 行短路紧邻 — prompt 须明示保护） | task_011 仍阻塞 task_010
[2026-05-13] STATE: ARCHITECTURE → DEVELOPING | Task: task_009 + task_010 并行启 | 原因: task_007 PASS 解锁；scheduler.rs 严格分段（009=PDF 分支 / 010=audio+video 分支）；task_007 6 行短路保护 + audio_asr_iflytek.rs 零触动 在 prompt 中明示；3 后台并发（含 task_014 Reviewer） | 风险: 中（同文件并行 dev，靠分段约束规避）
[2026-05-13] STATE: REVIEW → PASS | Task: task_014 | 原因: Reviewer 判决 PASS (4.85/5)；6 AC PASS（含 AC-6 标注完成 + Conductor 裁决方案 A SQL 字符级一致 + tables_ready 守卫加固合理）；8+1 红线全过；4 关注点全 OK（parse_failure_code 本地 helper 维护点单一 / 三态优先级正确收敛 / AssetListJoinRow 字段顺序未破坏 / "重新转录"按钮真接入 retryAssetConversion + 1s 防抖 + aria-label）；195/0 cargo test 无退步；2 MINOR（FailureCode 反向手抄 + 前端 vitest 待补）非阻塞 | 风险: 极低
[2026-05-13] STATE: DEVELOPING → REVIEW | Task: task_010 | 原因: dev 交付 AC-1~5 PASS；mime helper 自建无新依赖；video 错误码复用 EAudioWrongRoute（同义未扩枚举）；task_007 6 行短路保留并新增优先级测试；audio_asr_iflytek.rs 0 改动；cargo test 213/0（含 task_009 工作树并行 +10）；3 Reviewer 关注（grep gate 用 awk 限定数组段 / video 错误码复用 / release 路径未直接测） | 风险: 低
[2026-05-13] STATE: DEVELOPING → REVIEW | Task: task_009 | 原因: dev 交付 AC-1~6 PASS（mock 10/10 + 真实样本测 PENDING-OPERATOR 等 task_012）；lopdf 0.34 直接依赖（Cargo.lock 已有传递无重复编译）；scheduler.rs PDF 段集中改动（audio/video 段零触动）；215/0 cargo test（baseline 195 + task_010 +8 + task_009 +12 一致）；grep gate 双校验过；未触 markitdown/audio_asr_iflytek/runtime_check/failure_code/db | 风险: 低 | 并行 dev 协作: task_009/010 scheduler.rs 分段无冲突
[2026-05-13] STATE: REVIEW → PASS | Task: task_010 | 原因: Reviewer 判决 PASS (4.70/5)；5 AC 全过；3 关注点全 PASS（grep gate awk 限定接受 / video 复用 EAudioWrongRoute 同义 / release 路径 MINOR 不阻塞）；11 红线全过；task_007 6 行短路完整保留 line 129-136；audio_asr_iflytek.rs 0 改动证据明确；scheduler.rs PDF 段零触动；215/0；scorecard 落盘；3 MINOR（release CI / mime helper 抽公共 / dead_code 清理）非阻塞 | 风险: 极低
[2026-05-13] STATE: ARCHITECTURE → DEVELOPING | Task: task_011 (preserve vs modify matrix) | 原因: task_008+task_010 PASS 解锁；与 task_009 Reviewer（只读）零冲突；任务范围最低侵入：preserve_matrix.md 新建 + markitdown.rs 仅注释+测试段（≤30 行注释字面约束） | 风险: 低
[2026-05-13] STATE: REVIEW → PASS | Task: task_009 | 原因: Reviewer 判决 PASS (4.85/5)；6 AC + 红线 + 4 关注点全过；215/0 cargo test 一致；lopdf 0.34 纯 Rust 验证（cargo tree 无 *sys C 编译）；父链继承含循环检测 16 层；scan_pdf_route_decision 纯函数三态抽象；2 MINOR（mock fixture 多样性 / encrypted_pdf 防御性 fallback）非阻塞；scorecard 已落盘 | 风险: 极低
[2026-05-13] UNLOCK | Task: task_012 (real sample matrix CI secret) | 原因: 全部依赖 task_000/002/006/007/008/009/010 PASS；task_011/013/015 链式（011=DEVELOPING 不阻塞 012，013 等 012，015 等 012/013）
[2026-05-13] STATE: ARCHITECTURE → DEVELOPING | Task: task_012 | 原因: 验收脚本 task 不动业务代码；与 task_011 dev (markitdown.rs 注释) 零冲突；多 AC 必 PENDING（真实样本入库 task_000 PENDING-OPERATOR + CI macOS runner PENDING-CI）合理 | 风险: 中（大量 PENDING）
[2026-05-13] STATE: DEVELOPING → REVIEW | Task: task_011 | 原因: dev 交付 10 项矩阵（超 6 字面，+4 grep 发现）+ 9 preserve 注释 + 1 modify + 4 单测（含 AC-4 反向反例验证 image fallback 不被 classify_output 误判）；219/0 cargo test（215+4）；未触业务核心 | 风险: 低
[2026-05-13] STATE: DEVELOPING → REVIEW | Task: task_012 | 原因: dev 交付验收脚本 + 共用断言库 + CI workflow + KPF 强制 fail 编码；6 文件全新建零业务改动；AC-1/3/5 PASS + AC-2/4/6 合理 PENDING（task_000 PM 样本入库 + macOS runner）；self-test 10/10 + dry-run 全过；缺 secret 退出 2 文案明确 | 风险: 低
[2026-05-13] STATE: REVIEW → PASS | Task: task_011 | 原因: Reviewer 判决 PASS (5.00/5 满分 — 第二个满分继 task_002)；AC-1~5 全过；10 项矩阵覆盖 input.md 字面 6 + grep 发现 4；10 行注释（9 preserve + 1 modify ≤ 30 预算）；4 单测含 AC-4 反向；红线全过；219/0 cargo test 一致；4 关注点全 OK（含 95s 用 Duration::from_secs 不真 sleep + /usr/bin/true mock 带 fallback）；2 MINOR 非阻塞 | 风险: 极低
[2026-05-13] STATE: REVIEW → PASS | Task: task_012 | 原因: Reviewer 判决 PASS (4.475/5)；AC-1/3/5 PASS + AC-2/4/6 PENDING-OPERATOR/PENDING-CI/PENDING-SAMPLES 合理；红线全过；4 关键断言全过（secret 不入日志 / 样本明文不进 log / 100s wall-clock 上限 / 不跳过 epub-scan）；4 关注点全 OK；3 MINOR（known-fail-list grep 解析 / bash -x trace caveat / macOS CI gtimeout）非阻塞 | 风险: 极低
[2026-05-13] UNLOCK | Task: task_013 (clean VM smoke) | 解锁: task_005/006/012 全 PASS；task_015 仍阻塞 task_013
[2026-05-13] STATE: ARCHITECTURE → DEVELOPING | Task: task_013 | 原因: 验收脚本 task；与 task_012 共用 sample-assertions.sh（call/source 引用，不修改）；多 AC 必 PENDING（VM 资源 + 3 次冷启 + tart 工具链需用户机器） | 风险: 中（大量 PENDING-USER-MACHINE）
[2026-05-13] STATE: DEVELOPING → REVIEW | Task: task_013 | 原因: dev 交付 vm-smoke.sh 512 行 + vm-base-image.md + artifacts/ 占位；AC-1~5 PASS（静态）+ AC-6 PENDING-USER-MACHINE；tart 工具选定；AppleScript 双保险超时（内层 with timeout + host timeout +5s）；4 序列 mount/copy/launch/drop 完整；dev 主动修 macOS BSD date %N bug（改 python3 毫秒）；复用 task_012 sample-assertions.sh 仅 source 不改；幂等 rm 防 stale；4 文件全新建零越权 | 风险: 低
[2026-05-13] STATE: REVIEW → PASS | Task: task_013 | 原因: Reviewer 判决 PASS (4.60/5)；AC-1~6 全过（AC-6 PENDING-USER-MACHINE 合理）；6 红线全过；4 关注点全 OK（双保险超时 / P95 n=3 取 max 保守 / source 复用 self-test 仍 10/10 / artifacts 4 子类清晰）；macOS BSD date %N bug fix L378-381 验证到位（python3 stub 不破红线）；4 MINOR 非阻塞 | 风险: 极低
[2026-05-13] UNLOCK | Task: task_015 (rollback SOP) | 解锁: task_006/012/013 全 PASS；**最后一个 task** | 范围: docs/rollback_sop.md + docs/manifest_schema_versioning.md + scripts/archive-dmg.sh + verify-archive-presence.yml + drill_report.md 占位
[2026-05-13] STATE: ARCHITECTURE → DEVELOPING | Task: task_015 | 原因: 文档+归档脚本+CI+演练报告 task 不动业务代码；多 AC 必 PENDING（演练 PENDING-USER-MACHINE + PM/TechLead 双签 PENDING-PM）合理 | 风险: 低
[2026-05-13] STATE: DEVELOPING → REVIEW | Task: task_015 | 原因: dev 交付 6 文件全新建零修改（rollback_sop.md / manifest_schema_versioning.md / archive-dmg.sh / verify-archive-presence.yml / drill_report.md / output.md）；选择独立步骤方案不动 task_006 PASS 流水（保守合理）；archive-dmg.sh dry-run 矩阵 5/5 全过（正常归档+清理 / archive_report / sha256 一致 / dry-run 跳过 prune / hotfix backport 当前版本保护）；用 gh release list 不依赖 git 内 dist/archive | 风险: 低 — **最后一个 task** | 等 Reviewer 收官
[2026-05-13] STATE: REVIEW → PASS | Task: task_015 | 原因: Reviewer 判决 PASS (4.75/5)；6 AC 全过（AC-5 PENDING-USER-MACHINE 演练 + AC-6 PENDING-PM 实名签字 合理）；红线全过；4 关注点全 OK（独立步骤接受 + GH Releases 接受 + 双签 SOP 转 branch protection PM 启用 + schema enum 切换由 task_007 配套）；archive-dmg.sh dry-run 抽测 2/2（正常路径 + hotfix backport）；mtime 验证 build-macos-dmg.sh 真未触；4 MINOR（release upload 自动化 / 首次 schema bump 配套 / branch protection 文档化 / release-checklist 串接）非阻塞 | 风险: 极低

================================================================
[2026-05-14] 🚨 HOTFIX 启动 | 用户实测发现 2 个 P0 bug
================================================================
- 用户 2026-05-14 01:39 跑 cargo tauri dev 拖文件测试
- 现象 A: PDF/PNG → E_RUNTIME_MISSING（DB conversion_meta 实测）
- 现象 B: CSV/EPUB/HTML → mime application/octet-stream → unsupported
- 根因 A: tauri.conf.json bundle.resources = []，Tauri 2.x dev 模式 resource_dir = target/debug/ 不指向 src-tauri/resources/，runtime_check.rs 路径解析失败
- 根因 B: commands/sync.rs:240-256 guess_mime 只硬编码 11 类扩展名
- DMG 模式不受影响（build-macos-dmg.sh:199-208 手工 cp 救回）；dev 模式完全无法用 markitdown
- venv 实际正确（7 imports 全 PASS），不是依赖问题
- 启 task_H1 (Tauri resource_dir fix) + task_H2 (mime sniff fix) 并行 dev；文件零交集
[2026-05-14] STATE: DEVELOPING → REVIEW | Task: task_H2 | 原因: dev 交付 40+ 扩展名覆盖 + infer 0.19 内容嗅探兜底（对齐 Tauri 间接依赖避免双版本）+ 大小写不敏感 + 7 新单测；229/0 cargo test；单点调用方改 Path；3 偏差（m4a→audio/mp4 RFC 标准 / infer 0.19 vs 0.16 / AC-5 PENDING-USER-MACHINE）
[2026-05-14] STATE: DEVELOPING → REVIEW | Task: task_H1 | 原因: dev 交付 tauri.conf.json bundle.resources +1 + runtime_check.rs select_runtime_paths 纯函数抽取 + dev fallback（#[cfg(debug_assertions)] 保护 prod）+ 3 新单测；229/0 cargo test 一致；env!(CARGO_MANIFEST_DIR) 编译期注入 dev 源路径；零触越权区（包括 lib.rs 未动）
[2026-05-14] STATE: REVIEW → PASS | Task: task_H2 | 原因: Reviewer 判决 PASS (5.00/5 满分 — 第 4 个满分)；6 AC + 3 偏差 + 红线 + 4 关注点全过；infer 0.19 纯 Rust 验证（依赖链无 *-sys）；m4a → audio/mp4 不影响 task_010（前缀匹配命中）；229 = 222 baseline + 7 新测；scorecard 已落盘 | 风险: 极低
[2026-05-14] STATE: REVIEW → PASS | Task: task_H1 | 原因: Reviewer 判决 PASS (5.00/5 满分 — 第 5 个满分)；6 AC + 红线 + 4 关注点全过；select_runtime_paths 纯函数极简签名（&Path, Option<&Path>）→(PathBuf, PathBuf, bool) 单测友好；env! 三层保护安全；cargo test --lib --release 13/13 PASS 验证 prod #[cfg(debug_assertions)] 正确剥离；scorecard 已落盘 | 风险: 极低

================================================================
[2026-05-14] 🎉 HOTFIX COMPLETE | 2/2 task PASS | dev 模式 markitdown 转录链路修复
================================================================
- task_H1 PASS 5.00 (Tauri resource_dir + dev fallback)
- task_H2 PASS 5.00 (mime sniff 40+ 扩展名 + infer crate 兜底)
- 累计满分: 5 个（task_002, task_007, task_011, task_H1, task_H2）
- AC-5 (实测) PENDING-USER-MACHINE: 用户重跑 cargo tauri dev 后拖 PDF/PNG/CSV/EPUB/HTML 验证
================================================================

[2026-05-13] 🎉 SESSION COMPLETE | 15/15 task PASS | markitdown_fix 流水线全闭环
================================================================
- 总进度: 15 PASS, 0 阻塞, 0 FIX-AGAIN
- 满分 (5.00): task_002 (T-B markitdown extras) / task_007 R2 (manifest self-check) / task_011 (preserve matrix)
- 评分中位数: 4.85
- FIX 闭环: task_003 (HOME 透传) / task_007 (AC-3 短路接入) — 均 R1→FIX→R2 一轮闭合
- ESCALATE: task_014 (conversion_meta schema 不一致) — Conductor 裁决方案 A + 最新一行约束，input.md 末尾追加修订段
- Side-fix: V13 concepts 基表补建（spawn_task 修 12 pre-existing fail 来自 task_008 评审副产物）
- Conductor 错误自纠 2 次: task_003 Reviewer prompt 范围误描 / task_007 dev prompt 红线过紧（input.md AC-3 字面授权 scheduler.rs+markitdown::extract 入口）
- 并行最大化: 多轮 2-3 后台 agent 并发（dev+Reviewer / 2 Reviewer / 2 dev 分段 scheduler.rs）

下游 PM/Tech Lead 接力清单:
1. 注入 3 个公证 secret (NOTARY_KEY_ID / ISSUER_ID / P8_BASE64) → 解锁 task_005 真公证端到端
2. 真实样本入库 samples-private (≥35) → 解锁 task_009/012 真测
3. 准备干净 macOS 12+14 arm64 VM + tart → 解锁 task_013 实测
4. 执行 task_015 AC-5 回滚演练 (N-1 DMG 取回 + 干净 VM 安装 + 转录验证)
5. PM + Tech Lead 双签 rollback_sop.md + 启用 GitHub branch protection
6. Follow-up: commands/knowledge*.rs 加 filter legacy_unverified (task_014 AC-6 标注转的 follow-up，已通过 spawn_task 提单)
================================================================
