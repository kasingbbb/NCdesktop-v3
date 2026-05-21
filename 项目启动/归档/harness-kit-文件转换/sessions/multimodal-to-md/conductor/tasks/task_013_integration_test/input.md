# Task 输入 — task_013_integration_test

## 目标
端到端集成测试：验证完整的提取管道（导入 → 入队 → 提取 → 存储 → 搜索 → 预览）工作正常，修复发现的 Bug。

## 前置条件
- 依赖 task：全部（task_002 - task_012）
- 必须先存在的文件/接口：所有提取相关代码

## 验收标准（Acceptance Criteria）
1. AC-1：Rust 单元测试覆盖 `db/extraction.rs` 的 CRUD 操作
2. AC-2：Rust 集成测试：创建素材 → 入队 → 提取（使用 mock 提取器）→ 验证 extracted_content 写入正确
3. AC-3：PDF 文字提取端到端：准备一份测试 PDF → 调用提取器 → 验证输出 Markdown
4. AC-4：OCR 提取端到端（仅 macOS CI）：准备一张测试图片 → 调用 Vision OCR → 验证输出文字
5. AC-5：FTS 搜索集成：提取完成后搜索关键词 → 验证命中
6. AC-6：`cargo test` 全部通过（含新测试）
7. AC-7：`pnpm check`（TypeScript 类型检查）通过
8. AC-8：`pnpm tauri:build` 构建成功

## 技术约束
- 使用现有测试框架（`#[cfg(test)]`、`tempfile`）
- macOS 特定测试使用 `#[cfg(target_os = "macos")]` 条件编译
- 测试数据放在 `src-tauri/tests/fixtures/`
- 不测试前端 UI 交互（MVP 无 E2E 测试框架）

## 参考文件
- `src-tauri/src/testing.rs` — 现有测试基础设施
- 各 task 的 output.md — 实现细节
- PRD §5.1 性能指标 — 作为测试基准

## 预估影响范围
- 新建文件：`src-tauri/tests/` 下的集成测试文件、`src-tauri/tests/fixtures/` 测试数据
- 修改文件：可能修复任何发现的 Bug
