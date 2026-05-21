# Task 输入 — task_010_frontend_extraction

## 目标
前端提取功能基础设施：TypeScript 类型定义、extractionStore（Zustand）、Tauri IPC 封装、Tauri Event 监听。

## 前置条件
- 依赖 task：task_002（后端数据模型确定）
- 必须先存在的文件/接口：后端 IPC 命令签名（可参考 Architect output.md）

## 验收标准（Acceptance Criteria）
1. AC-1：`src/types/extraction.ts` 定义了 `ExtractedContent`, `ExtractionStatus`, `PipelineTask`, `PipelineProgress` 等类型
2. AC-2：`src/lib/tauri-commands.ts` 新增 `extractAsset`, `extractProjectAssets`, `getExtractionStatus`, `getExtractedContent`, `retryExtraction`, `getPipelineProgress` 封装
3. AC-3：`src/stores/extractionStore.ts` 实现了 Zustand store，管理提取状态、进度、提取内容缓存
4. AC-4：store 内置 Tauri Event 监听器（`extraction:progress`, `extraction:completed`, `extraction:failed`, `extraction:batch_progress`），自动更新状态
5. AC-5：`extractionStore` 导出并在 `stores/index.ts` 中 re-export
6. AC-6：类型与后端 Rust 模型一一对应（字段名 camelCase 转换正确）

## 技术约束
- 遵循现有 store 模式（参考 `assetStore.ts`）
- 使用 `@tauri-apps/api/core` 的 `invoke` 和 `@tauri-apps/api/event` 的 `listen`
- Event 监听器在 store 初始化时注册，组件卸载时不需要清理（全局状态）
- 禁止使用 `any` 类型

## 参考文件
- `src/stores/assetStore.ts` — store 模式参考
- `src/types/index.ts` — 类型定义导出模式
- `src/lib/tauri-commands.ts` — IPC 封装模式
- Architect output.md §API 设计 — 完整命令和事件列表

## 预估影响范围
- 新建文件：`src/types/extraction.ts`, `src/stores/extractionStore.ts`
- 修改文件：`src/types/index.ts`（导出）、`src/stores/index.ts`（导出）、`src/lib/tauri-commands.ts`（IPC 封装）
