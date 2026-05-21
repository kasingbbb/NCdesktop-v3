# Task 输入 — task_010_ux_review

## 目标
UX 体验审查：状态文案一致性、拖拽禁用反馈、失败/重试动线流畅性、键盘可达性、空态与错误态文案。

## 前置条件
- 依赖 task：task_008 前端集成完成（task_009 通过更好但非强依赖）
- 必须先存在的文件/接口：AssetListView 已切到 WorkspaceAssetView；useDragAssets 走 prepare_outbound_payload。

## 验收标准（AC）
1. **AC-1**：四态文案与图标一致：done=已就绪 / converting=转化中 / failed=失败 / offline=离线待转化；与 AssetContextMenu、toast、错误对话框统一。
2. **AC-2**：非 done 态拖拽用户感知：(a) 鼠标 hover 时 cursor 是否提示禁用？(b) startDrag 失败时 toast 文案是否区分单/混合态？(c) toast 是否会同时多条堆积？
3. **AC-3**：失败重试动线：失败行的"重试"按钮位置（行尾 vs 行内）、点击后即时反馈（btn loading 状态）、连续点击是否有视觉抖动。
4. **AC-4**：source-missing：列表中该资产是否有 source-missing 角标？"查看原文件"按钮是否置灰？
5. **AC-5**：键盘可达性：Cmd+A 多选、Enter 重命名（如已实现）、Backspace 删除（确认对话框中文）。
6. **AC-6**：空态文案：空项目导入前是否有引导（拖入文件 / 启动悬浮窗）？空筛选结果是否区分"无资产" vs "筛选无匹配"？
7. **AC-7**：交付一份 `ux_review.md` 报告到本 task 目录，含至少 5 条具体改进建议（每条标"已修复 / 需 P1 跟进 / 接受现状"）。

## 技术约束
- 仅 UX 审查；如需修复一律建 follow-up task，不在本 task 中改代码（避免破坏 task_009 已通过的集成测试）。
- 不引入 a11y 工具链外的新依赖。

## 参考文件
- `src/components/features/AssetListView.tsx`
- `src/components/features/AssetContextMenu.tsx`
- `src/hooks/useDragAssets.ts`
- session_context.md §4 质量偏好（用户体验权重 25%）

## 预估影响范围
- 新建文件：
  - `sessions/conductor/tasks/task_010_ux_review/ux_review.md`
- 估算变更：文档型，无代码改动
