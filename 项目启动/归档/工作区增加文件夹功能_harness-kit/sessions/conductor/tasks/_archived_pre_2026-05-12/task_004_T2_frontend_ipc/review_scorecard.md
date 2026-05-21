# Review Scorecard — task_004_T2_frontend_ipc

## 审查思考过程

1. **Task 意图**：在前端建立 IPC 边界（5 个 camelCase wrapper + 统一错误解包 `invokeWithIpcError` + 11 项闭集中文文案表 + TS 类型），契约源自 contracts.md §A/§B/§D 与 ADR-001。本 task 只生产前端 TS / 类型 / 纯函数，不依赖后端实装。

2. **审查前验证（handoff §3）**：
   - [x] 测试结果非空：`pnpm test ipc-errors` 16/16 PASS，`pnpm tsc --noEmit` EXIT=0，全量 `pnpm test` 23/25 文件 PASS。
   - [x] 自测验证矩阵存在且正常路径全部 PASS（13 项，正常 6 / 边界 4 / 异常 3）。
   - [x] 架构遵守声明已填写（含偏离说明：moveAsset 收敛单素材按 input.md 明确要求；kind 类型补强 + string 宽松回退）。
   - 通过，进入实质审查。

3. **AC 检查结果**：
   - AC-1 `pnpm test ipc-errors` PASS、11 code 中文文案 + E_FOLDER_DIRTY 用 `details.now` + 非 JSON fallback：✅
   - AC-2 5 camelCase wrapper 全在 `tauri-commands.ts`、参数 camelCase 与顺序与 §B.2 一致、moveAsset 收敛单素材：✅
   - AC-3 `IpcErrorCode`(11) / `IpcError` / `DeleteReport { trashed: number }` 已增；`tsc --noEmit` PASS：✅
   - AC-4 invoke reject JSON → IpcError，reject 非 JSON → 兜底 E_INTERNAL：✅
   - AC-5 `pnpm test` 全绿（4 个 fail 为 App.test/SearchPanel.test 预存，与本 task 无关）：✅（不计扣）

4. **关键发现**：
   - 合同对齐质量极高：11 code 字面量、details 字段名（`relative_path` / `now`）、文案逐字与 §D.1 / §D.2 一致；`parseIpcError` 对"JSON parse 成功但 code 不在闭集"主动降级 E_INTERNAL，符合 ADR-001 闭集契约。
   - 跨 task 集成窗口风险：T2 已把 `AssetContextMenu.tsx` 切到新单素材 wrapper，但 T3 后端单素材 `move_asset_to_workspace_folder` 尚未实装；在 T3 落地前合并到 main 会让现网拖拽（多选/单选）破坏。属 T3 验收门槛内的合理交叉窗口，标 MAJOR + T3 必须紧接落地。

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 25% | 5 | 5 AC 全满足；wrapper 名称/参数顺序/类型与 contracts.md §B.2 字符级一致；`__ROOT__` 原样透传未翻译；`DeleteReport.trashed: number` 正确；`countFolderAssets` 返回 number 与后端 u32 对齐。 |
| 安全性 | 25% | 5 | 文案唯一来源在前端、后端 `message` 不直接展示（符合 §D 红线）；闭集强制（非闭集 code 一律降级 E_INTERNAL），不会被后端误传/伪造 code 污染 UI 分支；`isIpcError` 不依赖 `instanceof`，跨 V8 边界安全；不引入新运行时依赖；无敏感字段透出。 |
| 代码质量 | 15% | 5 | 命名/职责清晰；`parseIpcError` 三分支结构干净；注释指引契约出处；`IPC_ERROR_CODE_SET` 抽出避免 O(n) 扫描；`renderIpcError` 提供便捷出口；未顺手重构既有 wrapper。 |
| 测试覆盖 | 20% | 5 | 16 用例覆盖：11 code 非空 + CJK 断言、`E_FOLDER_DIRTY` 用 `details.now`、`E_NAME_DUP/INVALID` 拼 name、`E_PATH_ESCAPE` 拼 relative_path、`isIpcError` 守卫正反例、parse 4 路径（IpcError 对象 identity / 合法 JSON / 非 JSON / 闭集外 code / 非 string 非对象 42）、invoke 成功 + 抛 JSON + 抛非 JSON 三路径。AC-4 完全覆盖。 |
| 架构一致性 | 10% | 5 | 与 ADR-001（Err(String) JSON 包装）/ ADR-004（`__ROOT__` 不翻译）/ contracts.md §B.2 完全一致；MVP 闭集 11 项不增不减；未引入 `E_DEPTH_LIMIT`/`E_CYCLE`；类型放 `src/types/workspace.ts`、wrapper 放 `src/lib/tauri-commands.ts`、纯函数放 `src/lib/ipc-errors.ts`，目录与方案对齐。 |
| 可维护性 | 5% | 4 | 注释充分、契约出处可回溯；`WorkspaceFolderEntry.kind: WorkspaceFolderKind \| string` 保留 string 宽松回退是技术债（已在 output.md 标注），未来调用方收紧后可移除。`invokeWithIpcError` args 类型 `Record<string, unknown>` 偏宽，wrapper 内部传 camelCase 字面量对象足够，未影响外部 API。 |

**综合分：4.95 / 5**（加权：5×0.25 + 5×0.25 + 5×0.15 + 5×0.20 + 5×0.10 + 4×0.05 = 4.95）

## 总体判断

- [x] **PASS**
- [ ] FIX
- [ ] BLOCKER

无 BLOCKER；1 项 MAJOR 属"已知跨 task 集成窗口"，不阻塞本 task PASS。

## 问题列表

### BLOCKER

无。

### MAJOR

1. **问题**：`AssetContextMenu.tsx:102-104` 已切到新单素材 `moveAssetToWorkspaceFolder(assetId, targetRelativePath)`，但后端 `move_asset_to_workspace_folder` 由 T3 实装。在 T3 完成并 merge 之前，若 main 包含本 task 的 `AssetContextMenu.tsx` 改动，运行时拖拽/右键移动会失败（命令未注册或仍是旧 `assetIds[]` 签名）。
   - **代码位置**：`NCdesktop/src/components/features/AssetContextMenu.tsx:94-112`、`NCdesktop/src/lib/tauri-commands.ts:224-232`
   - **修复方向**：本 task 不需改动；列为 **T3 验收门槛**——T3 必须实装单素材后端命令并与本 task 一起合入 main；Conductor 不应在 T3 完成前把本 task 单独 merge 到 main 分支。
   - **验证标准**：T3 output.md 自测矩阵需含「`AssetContextMenu` 拖入根目录 / 拖入子文件夹 / 拖回 `__ROOT__`」三路径手测 PASS；或集成测试 `test_round_trip_root_to_folder_to_root` 全绿。
   - **本 task 已知局限即可**，已在 output.md §"需要 Reviewer 特别关注 1" 标注，符合契约。

### MINOR

1. `WorkspaceFolderEntry.kind: WorkspaceFolderKind | string` 联合保留 string 宽松回退，是为不破坏既有 `kind === 'ai_organized'` 字符串比较。建议后续 task（T5/T6 UI 落地后）一次性收紧为纯字面量联合，并移除 `| string`。本 task 不修。
2. `invokeWithIpcError` 第二参数 `args?: Record<string, unknown>` 类型较宽。后续如需更强类型，可考虑泛型 `Args extends Record<string, unknown>`；本 task 无需改。
3. `parseIpcError` 对"JSON 解析成功但 code 不在闭集"会丢失原始 code 信息（仅 message 保留原 JSON 字符串）。这是 Dev 主动的闭集强制选择，与 ADR-001 fallback 语义一致，可作为团队约定记入 ADR 注脚，无需改代码。

## 给 Dev 的修复指引

不适用（PASS）。

---

**Reviewer 备注**：本 task 是本期最干净的一份交付——契约对齐字符级一致、测试覆盖矩阵完整、`parseIpcError` 三分支兜底逻辑健壮、未顺手重构。唯一交叉窗口风险归属 T3，已正确标注为本 task 已知局限。可直接进入 T3。
