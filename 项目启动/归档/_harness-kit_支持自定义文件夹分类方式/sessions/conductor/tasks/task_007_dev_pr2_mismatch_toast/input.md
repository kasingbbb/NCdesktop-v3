# Task 输入 — task_007_dev_pr2_mismatch_toast

## 目标
F5 本地启发式 mismatch toast：在子目录直接导入后，若本地启发式判断"该文件可能不属于当前分类"，弹出非阻塞 toast 提示并提供"点此重选"。

## 前置条件
- 依赖 task：task_006
- 必须先存在的文件/接口：`categoryStore` (task_009 之前可临时调命令读)、`commands/list_workspace_assets`（task_008）—— **注**：task_007 可在 task_006 完成后即开发，但启发式数据源依赖部分由 PR-3 提供；建议拆为：
  - 7a：算法实现（不依赖 PR-3）
  - 7b：UI 集成（依赖 task_008 的命令）
  > 本 task 内顺序执行 7a → 7b，整体仍 ≤ 200 行

## 验收标准（AC）
1. 实现 `compute_category_match(file_name, category_slug, project_id) -> f32`：
   - 中英文 token 切分（CJK n-gram + ascii word）
   - 与同分类既有资产 tags 词袋（前 50）做 Jaccard
   - 无既有资产时返回 1.0（跳过判定）
2. 阈值 `< 0.05` 触发 toast；阈值常量化便于调参
3. `MismatchToast` 组件：标题"这个文件似乎不属于「{label}」"，body 文件名，按钮"点此重选"打开分类选择 modal
4. 不阻塞导入流程；toast 自动 8s 消失
5. 单测：(a) 中文 token (b) 英文 token (c) 无既有资产 short-circuit (d) 阈值边界

## 技术约束
- 算法实现 ≤ 80 行
- 不引入新依赖（无 jieba、无 TF-IDF）
- 前端 toast 复用现有 toast 系统

## 参考文件
- task_001 output.md ADR-007
- 现有 toast 组件位置（grep `useToast`）

## 预估影响范围
- 新建：`src-tauri/src/heuristic.rs`（~80）、`src/components/features/MismatchToast.tsx`（~80）
- 修改：`commands/dropzone.rs`（导入完成后返回 match_score）、`src/components/features/Dropzone.tsx`（订阅返回值弹 toast）
- 测试：`src-tauri/tests/heuristic.rs`（~50）

## Reviewer 重点关注
- CJK n-gram 长度选择（建议 2-gram）
- 用户连续触发 toast 的合并策略
- 阈值是否需要对极短文件名（< 4 字符）放宽
