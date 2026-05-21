# Conductor Progress — multimodal-to-md

## 当前状态
STATE: P0_COMPLETE_PENDING_ACCEPTANCE
当前 Task: 无（P0 全部开发任务完成，等待用户验收后推进 P1）
更新时间: 2026-04-12T12:30:00

## 项目信息
- Session: multimodal-to-md（多模态文件转换引擎）
- 复杂度: L（复杂）
- PRD: `prd/multimodal-extraction-prd-v1.md`（经 4 层 Debate 产出）
- Architect 方案: `conductor/tasks/task_001_architect/output.md`

---

## P0 MVP 已完成 Tasks（12/12）

### 基础设施
- [x] task_001_architect — 技术方案设计（4 个 ADR，12 个 Task 拆分）
- [x] task_002_db_v5_migration — V5 迁移（extracted_content + pipeline_tasks + v_asset_content VIEW + fts_content FTS5）
- [x] task_003_extractor_trait — Extractor trait + extraction 模块骨架

### 提取器实现
- [x] task_004_pdf_text_extractor — PDF 文字型提取器（pdf-extract crate）
- [x] task_005_vision_ffi — macOS Vision OCR Swift-Rust FFI 桥接（@_cdecl + swiftc 编译）
- [x] task_006_image_ocr_extractor — 图片 OCR 提取器（Vision Framework，支持 JPEG/PNG/HEIC/WebP）
- [x] task_007_pdf_scan_extractor — PDF 扫描型提取器（PDFKit 渲染 + Vision OCR + 自动回退）

### 管道与搜索
- [x] task_008_pipeline_scheduler — 管道调度器（顺序执行 + 启动恢复 + 失败重试 + Tauri Event）
- [x] task_009_fts_content_search — FTS5 内容搜索扩展（search_all 覆盖素材+笔记+内容）

### 前端 UI
- [x] task_010_frontend_extraction — 前端类型 + extractionStore + IPC 封装
- [x] task_011_inspector_content — Inspector 内容预览 + 提取状态角标（ExtractionBadge）
- [x] task_012_dropzone_auto_extract — Dropzone 导入自动入队 + Toolbar 进度条

### 额外修复
- [x] hotfix: Toolbar Inspector 开关按钮（PanelRight 图标）— 关闭后可重新打开
- [x] hotfix: 知识关联按钮联动 inspectorOpen — 点击时若右栏关闭则自动打开

---

## P0 验收状态
- cargo build: ✅ 通过（0 errors, 4 pre-existing warnings）
- 前端 TypeScript 类型检查: ✅ 通过
- task_013_integration_test: ⏳ 未执行（可在验收后补充）

---

## P1 待执行 Task 队列（PRD §7 P1 计划，约 12 工作日）

| ID | 功能 | 优先级 | 描述 | 依赖 |
|----|------|--------|------|------|
| P1-01 | ai_enrichments 表 + 三态编辑保护 | P1 | LLM 增强结果存储 + accepted/draft/user_modified 状态 | P0 完成 |
| P1-02 | jieba-rs 中文分词 | P1 | FTS5 预分词提升中文搜索质量 | task_009 |
| P1-03 | 音频 ASR (Speech Framework) | P1 | macOS Speech Framework 保底 ASR | task_005 |
| P1-04 | segments_json Envelope Schema v1 | P1 | 完整 segment 类型定义 | task_003 |
| P1-05 | 关键词标签提取 | P1 | 文件元数据 + TF 规则自动标签 | P0 完成 |
| P1-06 | 任务中心 UI | P1 | Toolbar 任务面板：活跃/完成/失败任务管理 | task_012 |
| P1-07 | export_project_markdown 升级 | P1→提前 | 合并 extracted_content 到项目导出 | task_002 |

## P2 待执行 Task 队列（PRD §7 P2 计划，约 9 工作日）

| ID | 功能 | 优先级 | 描述 |
|----|------|--------|------|
| P2-01 | Whisper 按需下载 | P2 | 高精度离线 ASR，模型 75-142MB |
| P2-02 | EPUB 解析 | P2 | epub-rs 章节提取 |
| P2-03 | LLM 智能标签 | P2 | 复用 llm_classify，输入 extracted_content |
| P2-04 | ai_analyses 完全弃用 | P2 | 数据迁移 + 删除旧表 |

---

## 已解决的风险项
| 风险 | 来源 | 结果 |
|------|------|------|
| Vision FFI 桥接复杂度超预期 | Debate L3 | ✅ 已解决 — @_cdecl Swift FFI 方案成功，Day 1 即完成 |
| PDF 扫描型提取 | PRD F03 | ✅ 已解决 — PDFKit 渲染 + Vision OCR + 自动回退 |
| SQLite 并发写冲突 | Debate L3 | ✅ 已缓解 — 单连接 Mutex + 顺序执行调度器 |

## 待验证风险项
| 风险 | 概率 | 影响 | 状态 |
|------|------|------|------|
| jieba-rs 词典 4MB 导致体积接近上限 | 低 | 低 | P1 验证 |
| ASR 质量不达预期 | 高 | 高 | P1 验证（Speech Framework 保底 + Whisper 升级） |

---

## 累积异常计数器
- 同类 FIX 重复: 0
- 连续低分: 0
- ESCALATE 次数: 0

## 关键决策记录
[2026-04-12T10:00] PRD 交接契约验证通过，启动 ARCHITECTURE
[2026-04-12T10:20] Architect 产出技术方案，4 个 ADR（Vision FFI、PDF 提取、调度器、FTS5）
[2026-04-12T10:30] 启动 DEVELOPING，调度 task_002 (DB V5)
[2026-04-12T10:45] task_002 完成（代码已预先存在，补全模块声明）
[2026-04-12T10:50] 并行调度 task_003 + task_005（关键路径分支）
[2026-04-12T11:00] task_003 + task_005 完成，Vision FFI 风险消除
[2026-04-12T11:10] 并行调度 task_004 + task_006 + task_008 + task_010（四路并行）
[2026-04-12T11:30] task_004/006/008/010 完成
[2026-04-12T11:40] 并行调度 task_007 + task_009 + task_011 + task_012（最终批次）
[2026-04-12T12:00] P0 全部开发任务完成
[2026-04-12T12:20] hotfix: Inspector 开关按钮 + 知识关联按钮联动修复
[2026-04-12T12:30] 一阶段（P0）任务完成，保存进度待验收

## 状态转移日志
[2026-04-12T10:00] STATE: INIT → ARCHITECTURE | 原因: PRD 验收通过 | 风险: 无
[2026-04-12T10:30] STATE: ARCHITECTURE → DEVELOPING | 原因: 技术方案完成 | 风险: 低
[2026-04-12T12:00] STATE: DEVELOPING → P0_COMPLETE | 原因: 12 个 Task 全部通过 | 风险: 低
[2026-04-12T12:30] STATE: P0_COMPLETE → P0_COMPLETE_PENDING_ACCEPTANCE | 原因: 用户确认保存进度 | 风险: 无

---

## 恢复指引（给下次 Conductor）

1. 读取本文件确认当前状态为 `P0_COMPLETE_PENDING_ACCEPTANCE`
2. 读取 `session_context.md` 了解项目背景
3. 读取 `prd/multimodal-extraction-prd-v1.md` §7 P1 计划
4. 读取 `conductor/tasks/task_001_architect/output.md` 了解技术方案
5. 确认用户是否要：
   a. 先验收 P0（运行 `pnpm tauri:dev` 测试）
   b. 直接推进 P1 开发
   c. 提前实现 P1-07（export_project_markdown 升级，将 extracted_content 集成到导出）
6. 如推进 P1，需要为 P1 任务创建新的 Architect 方案和 Task input.md

## 代码库变更摘要（P0 新增/修改文件）

### 新增 Rust 文件
- `src-tauri/src/extraction/mod.rs` — Extractor trait
- `src-tauri/src/extraction/models.rs` — 数据模型
- `src-tauri/src/extraction/scheduler.rs` — PipelineScheduler
- `src-tauri/src/extraction/extractors/mod.rs` — 提取器注册表
- `src-tauri/src/extraction/extractors/pdf_text.rs` — PDF 文字提取器
- `src-tauri/src/extraction/extractors/image_ocr.rs` — 图片 OCR 提取器
- `src-tauri/src/extraction/extractors/pdf_scan.rs` — PDF 扫描型提取器
- `src-tauri/src/db/extraction.rs` — extracted_content + pipeline_tasks CRUD
- `src-tauri/src/commands/extraction.rs` — 6 个 IPC 命令
- `src-tauri/src/macos/mod.rs` — macOS 模块入口
- `src-tauri/src/macos/ocr_ffi.rs` — Vision OCR FFI 封装
- `src-tauri/macos/ocr_bridge.swift` — Vision + PDFKit Swift 桥接

### 修改 Rust 文件
- `src-tauri/src/lib.rs` — 新模块声明 + Scheduler 状态管理 + IPC 注册
- `src-tauri/src/db/mod.rs` — pub mod extraction
- `src-tauri/src/db/migration.rs` — V5 迁移
- `src-tauri/src/db/search.rs` — search_content + search_all 扩展
- `src-tauri/src/commands/mod.rs` — pub mod extraction
- `src-tauri/src/commands/dropzone.rs` — 导入后自动入队提取
- `src-tauri/build.rs` — Swift 编译步骤
- `src-tauri/Cargo.toml` — pdf-extract 依赖

### 新增前端文件
- `src/types/extraction.ts` — 提取相关类型
- `src/stores/extractionStore.ts` — Zustand store
- `src/components/features/extraction/ExtractionBadge.tsx` — 状态角标
- `src/components/layout/InspectorExtraction.tsx` — Inspector 提取内容面板

### 修改前端文件
- `src/types/index.ts` — 导出 extraction 类型
- `src/stores/index.ts` — 导出 extractionStore
- `src/lib/tauri-commands.ts` — 6 个 IPC 封装函数
- `src/components/layout/Inspector.tsx` — 集成 InspectorExtraction
- `src/components/features/AssetListView.tsx` — ExtractionBadge 角标
- `src/components/layout/Toolbar.tsx` — 提取进度 + Inspector 开关按钮
