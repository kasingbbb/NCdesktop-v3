# Task 输入 — task_010_architecture_guard

## 目标

对全流程交付物做 Architecture Guard 终审：核对实际代码与 Architect output.md 各章节、ADR、风险登记表的逐项一致性；扫描"现状勘察发现 → 是否在实现中已收敛"；产出一份可阅读的最终验收报告。L 复杂度强制环节。

## 前置条件

- 依赖 task：`task_009_ux_review` 已 DONE（含 findings.md），且如有 BLOCKER 已被 task_007 二轮修复
- 必须先存在的文件/接口：所有 task_002 ~ task_009 的 output.md 与代码产物

## 验收标准（Acceptance Criteria）

1. **AC-1（ADR 落地验证）** — 对 Architect output.md `§ 3` 5 个 ADR 逐条验证：

   | ADR | 验证清单 |
   |---|---|
   | ADR-001（内置 fallback） | ① `runtime_prompt_for` 在 is_custom=0 时返回 default ② 升级源码字面值后旧用户自定义不被覆盖 ③ `default_for(module)` 是唯一对外入口 |
   | ADR-002（SQLite 表） | ① migration V15 已落 ② 表结构与 § 5.1 一致（包含 builtin_version 字段）③ migration 测试推到 15 |
   | ADR-003（输出守卫三层防御） | ① `output_format_addon` 三常量字面值与 § 4.2 一致 ② messages 中守卫永远最后 ③ Layer B `validate_required_placeholders` 在 save 路径被调用 ④ R1 对抗测试 PASS |
   | ADR-004（双层字节校验） | ① `MAX_USER_PROMPT_BYTES = 16384` ② `MAX_TOTAL_PROMPT_CHARS = 65536` ③ 保存时校验在 `commands/user_prompt.rs::save_user_prompt` ④ 调用前校验在 `assemble_messages_for_*` |
   | ADR-005（独立 store） | ① 未修改 `src/stores/promptStore.ts` ② 未修改 `src/components/settings/PromptEditor.tsx` ③ 新建 `userPromptStore.ts` + `PromptCustomizationPanel.tsx` |

2. **AC-2（目录结构一致性）** — 对 Architect output.md `§ 7` 列出的新建/修改文件清单，逐一 `ls`/`stat` 验证：
   - 所有 § 7 列出的新建文件存在
   - 所有 § 7 列出的修改文件确实被修改（git diff 中可见）
   - 没有计划外的新文件（grep `src-tauri/src/commands/*.rs` 与 `src/components/settings/*.tsx` 比对 PR 起点）

3. **AC-3（契约一致性）** — 比对：
   - 后端 `PromptInfo` 结构（含 serde rename_all="camelCase"）vs 前端 `types/user-prompt.ts::PromptInfo` 字段一一对应
   - 4 个 Tauri command 名称（`list_user_prompts / get_user_prompt / save_user_prompt / reset_user_prompt`）在 `lib.rs invoke_handler!` / `commands/mod.rs` / `tauri-commands.ts` 三处命名一致
   - `PromptModule` 字面量在前后端三处（`commands/user_prompt.rs::KINDS`、`prompt_runtime.rs::default_for` match、`types/user-prompt.ts::PROMPT_MODULES`）严格一致

4. **AC-4（风险登记表收敛验证）** — 对 § 9 的 R1~R9 逐项验证落地：

   | 风险 | 验证 |
   |---|---|
   | R1（对抗式 prompt） | task_008 AC-1 中"R1 对抗式 prompt 模拟"测试 PASS |
   | R2（token 超限） | task_008 AC-1 中"字节超限"+"调用前字符数超限"两测 PASS |
   | R3（版本落后） | `builtin_version` 字段在表中存在；MVP 不需要主动提示，验证仅"字段存在" |
   | R4（PRD 4 ↔ 后端 3 映射） | task_009 findings.md 给出明确决议（A/B/C 之一），且决议落地 |
   | R5（AppMode 未注册） | `lib.rs` setup 中 `app.manage(AppMode::Normal)` 存在 |
   | R6（PR-4 命名隔离） | 未触碰 PR-4 半成品代码 |
   | R7（migration V15 残留 schema） | task_002 测试覆盖 fresh DB 与 v14→v15 路径，PASS |
   | R8（classify_prompt 签名兼容） | 旧 wrapper 存在 + 调用方已全部切换到 v2 + 既有未迁移调用（如 summarize）仍能跑 |
   | R9（dry-run 缺失） | 已在 output.md § 9 R9 注明 MVP 不实现，task_009 评审未要求新增 |

5. **AC-5（产出最终报告）** — 在 `sessions/custom_prompt_v1/conductor/tasks/task_010_architecture_guard/output.md` 产出：

   ```markdown
   # Architecture Guard 终审报告

   ## 1. ADR 落地一致性
   [按 AC-1 表格逐项 ✓/✗]

   ## 2. 目录结构一致性
   [按 AC-2 逐文件 ✓/✗]

   ## 3. 契约一致性
   [字段对照表 + ✓/✗]

   ## 4. 风险闭环
   [按 AC-4 表格逐项 ✓/✗]

   ## 5. 偏差登记
   [若有任何偏离 Architect 方案的实现选择，逐项列出 + 原因 + 是否影响验收]

   ## 6. 终审结论
   ☐ PASS：方案可视为 DONE
   ☐ FIX：列出修复项指派给具体 task（task_002~007）

   ## 7. 已知遗留 (移交 Conductor 决策)
   [§ 12 待 Conductor / PM 决策的偏离点的处置追踪]
   ```

6. **AC-6（无代码变更）** — 本 task 不写代码；如发现 BLOCKER 必须由 Conductor 主对话决定指派回某个 task 修复

## 技术约束

- 仅 read / grep / git diff 操作；不修改任何代码或文档（除 task_010 自身 output.md 与可能的 progress.md 更新）
- 验证用 grep / Read 工具读取实际代码，不依赖记忆或 task output.md 自述

## 参考文件

**必读**：
- Architect output.md 全文
- 所有 task_002 ~ task_009 的 output.md
- task_009 findings.md
- `core/handoff_contracts.md` § 5（progress.md 字段要求）

## 预估影响范围

- **新建文件**：
  - `sessions/custom_prompt_v1/conductor/tasks/task_010_architecture_guard/output.md`
- **修改文件**：原则上无；如发现 BLOCKER 由 Conductor 决策走 task_002~007 fix-round
- **预估变更**：终审报告 ~300 行
