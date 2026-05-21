# 技术方案 — 多模态文件转换引擎 P0 MVP

> Architect 产出 | 2026-04-12
> 基于 PRD v1.0 + 代码库探索

---

## 项目概述

NCdesktop 当前能浏览和管理多模态素材（照片、PDF、音频），但**无法提取素材内部内容**。用户的工作流在「素材 → 可被 AI 消费的结构化文本」环节断裂。

P0 MVP 目标：用户将照片和 PDF 拖入 NCdesktop，即可**离线**获得可搜索、可复制的结构化 Markdown。核心产出：`extracted_content` 表 + 管道调度器 + PDF/OCR 提取器 + FTS5 索引 + 前端预览/状态 UI。

---

## 技术选型

### Architecture Decision Records (ADR)

#### ADR-001: macOS Vision Framework OCR 桥接方案

- **状态**：已接受
- **上下文**：需要在 Rust 中调用 macOS Vision Framework 的 `VNRecognizeTextRequest` 进行图片 OCR。Debate 中标注此为最高风险项。
- **决策**：采用 **Swift 编译单元 + C ABI 桥接** 方案
  - 编写一个 Swift 文件 (`src-tauri/macos/ocr_bridge.swift`)，暴露 `extern "C"` 函数
  - 通过 `build.rs` 编译 Swift 文件并链接到 Rust
  - Rust 侧通过 `extern "C"` 调用
- **被排除项**：
  - `swift-bridge`：增加编译时依赖和构建复杂度，对于仅需 1-2 个函数的场景过重
  - `objc2` crate 族：Vision Framework 的 ObjC API 较复杂，纯 Rust 绑定调试困难
  - 独立 CLI 工具：进程间通信开销大，不适合批量 OCR
- **后果**：需要 Xcode CLI 工具支持；CI/CD 必须在 macOS 上运行；`build.rs` 增加维护成本
- **风险缓解**：Day 3 checkpoint — 如果 Swift 桥接失败，退化为 `osascript` + AppleScript 方案（性能降级但可用）

#### ADR-002: PDF 文字提取方案

- **状态**：已接受
- **上下文**：需要从 PDF 文件中提取文字，支持文字型和扫描型 PDF
- **决策**：
  - **文字型 PDF**：使用 `pdf-extract` crate（纯 Rust，零系统依赖）
  - **扫描型 PDF**：使用 `pdfium-render` 逐页渲染为图片 → Vision OCR
  - **混合型 PDF**：先尝试 `pdf-extract`，若提取文字量过少（< 50 字/页），回退为扫描型流程
- **被排除项**：
  - `poppler` bindings：需要系统依赖，违反零配置约束
  - `mupdf-rs`：C 绑定稳定性存疑
- **后果**：`pdf-extract` 对复杂布局的提取质量有限（Level 1-2），复杂排版需 LLM 增强（P1）

#### ADR-003: 管道调度器架构

- **状态**：已接受
- **上下文**：提取任务需后台执行、不阻塞 UI、持久化到 DB、支持崩溃恢复
- **决策**：
  - `pipeline_tasks` 表持久化任务状态
  - `tokio::spawn` 运行独立 async task，通过 `AppHandle` 的 `emit` 推送 Tauri 事件
  - **顺序执行**（MVP 不做并发，降低 SQLite 并发写冲突风险）
  - 应用启动时扫描 `status = 'running'` 的任务，重置为 `queued` 并重新入队
- **被排除项**：
  - 独立进程/Worker：过度架构，MVP 不需要
  - Channel-based queue：不持久化，崩溃丢失
- **后果**：大批量导入时提取速度受限于顺序执行（P1 优化为并发）

#### ADR-004: FTS5 全文索引策略

- **状态**：已接受
- **上下文**：提取的文字需要全文检索，现有搜索仅覆盖素材名和笔记内容
- **决策**：
  - 新建 `fts_content` 虚拟表，索引 `extracted_content.raw_text`
  - 扩展 `search_all` 函数增加 `search_content` 分支
  - P0 使用 FTS5 默认 tokenizer（对中文有限支持），P1 引入 jieba-rs
- **被排除项**：
  - 替换现有 `fts_assets`/`fts_notes`：破坏性变更，不合理
- **后果**：中文搜索在 P0 质量有限（按字符匹配），P1 通过预分词提升

---

## 系统架构

```
┌─────────────────────────────────────────────────────────────┐
│                    NCdesktop 提取管道架构                      │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  前端层 (React/TypeScript)                                   │
│  ┌──────────┐  ┌──────────────┐  ┌───────────────┐          │
│  │ Dropzone │→ │ AssetListView│  │   Inspector   │          │
│  │ 自动入队  │  │ 状态角标     │  │ 内容预览/重试 │          │
│  └──────────┘  └──────────────┘  └───────────────┘          │
│       ↕ invoke              ↕ listen events                  │
│  ┌───────────────────────────────────────────────────┐      │
│  │            extractionStore (Zustand)               │      │
│  │  taskProgress | extractionStatus | extractedContent│      │
│  └───────────────────────────────────────────────────┘      │
│                                                              │
├─────── IPC (Tauri Commands + Events) ───────────────────────┤
│                                                              │
│  后端层 (Rust)                                               │
│  ┌───────────────────────────────────────────────────┐      │
│  │              commands/extraction.rs                 │      │
│  │  extract_asset | get_extraction_status |            │      │
│  │  get_extracted_content | retry_extraction |          │      │
│  │  extract_project_assets                             │      │
│  └────────────────────┬──────────────────────────────┘      │
│                       ↓                                      │
│  ┌───────────────────────────────────────────────────┐      │
│  │           extraction/scheduler.rs                   │      │
│  │  PipelineScheduler — 任务队列 + 顺序执行            │      │
│  │  启动恢复 | 失败重试 | Event 进度推送               │      │
│  └────────────────────┬──────────────────────────────┘      │
│                       ↓                                      │
│  ┌───────────────────────────────────────────────────┐      │
│  │         extraction/extractors/* — 提取器            │      │
│  │  trait Extractor {                                  │      │
│  │    fn can_handle(&self, mime: &str) -> bool;        │      │
│  │    async fn extract(&self, path, opts) -> Result;   │      │
│  │  }                                                  │      │
│  │  ┌────────────┐ ┌───────────┐ ┌────────────────┐  │      │
│  │  │ PdfText    │ │ ImageOcr  │ │ PdfScanOcr     │  │      │
│  │  │(pdf-extract)│ │(Vision)   │ │(render+Vision) │  │      │
│  │  └────────────┘ └───────────┘ └────────────────┘  │      │
│  └───────────────────────────────────────────────────┘      │
│                       ↓                                      │
│  ┌───────────────────────────────────────────────────┐      │
│  │                 db/extraction.rs                     │      │
│  │  CRUD: extracted_content + pipeline_tasks           │      │
│  │  FTS 索引更新                                       │      │
│  └───────────────────────────────────────────────────┘      │
│                                                              │
│  ┌───────────────────────────────────────────────────┐      │
│  │             macos/ocr_bridge.swift                  │      │
│  │  recognize_text_in_image(path) → JSON              │      │
│  │  macOS Vision Framework (VNRecognizeTextRequest)    │      │
│  └───────────────────────────────────────────────────┘      │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## 数据模型

### V5 迁移 — 新增表

#### `extracted_content`

```sql
CREATE TABLE IF NOT EXISTS extracted_content (
    id              TEXT PRIMARY KEY,
    asset_id        TEXT NOT NULL UNIQUE REFERENCES assets(id) ON DELETE CASCADE,
    status          TEXT NOT NULL DEFAULT 'pending'
                    CHECK(status IN ('pending','extracting','extracted','failed','unsupported')),
    error_message   TEXT,
    retry_count     INTEGER NOT NULL DEFAULT 0,
    raw_text        TEXT,
    structured_md   TEXT,
    quality_level   INTEGER NOT NULL DEFAULT 0,
    extractor_type  TEXT NOT NULL DEFAULT '',
    segments_json   TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_ec_asset ON extracted_content(asset_id);
CREATE INDEX IF NOT EXISTS idx_ec_status ON extracted_content(status);
```

#### `pipeline_tasks`

```sql
CREATE TABLE IF NOT EXISTS pipeline_tasks (
    id              TEXT PRIMARY KEY,
    asset_id        TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    task_type       TEXT NOT NULL CHECK(task_type IN ('extract','enhance','index')),
    status          TEXT NOT NULL DEFAULT 'queued'
                    CHECK(status IN ('queued','running','completed','failed','cancelled')),
    retry_count     INTEGER NOT NULL DEFAULT 0,
    max_retries     INTEGER NOT NULL DEFAULT 3,
    error_message   TEXT,
    priority        INTEGER NOT NULL DEFAULT 100,
    batch_id        TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    started_at      TEXT,
    completed_at    TEXT,
    UNIQUE(asset_id, task_type)
);
CREATE INDEX IF NOT EXISTS idx_pt_status ON pipeline_tasks(status);
CREATE INDEX IF NOT EXISTS idx_pt_batch ON pipeline_tasks(batch_id);
```

#### `v_asset_content` — 兼容 VIEW

```sql
CREATE VIEW IF NOT EXISTS v_asset_content AS
SELECT
    a.id AS asset_id,
    COALESCE(ec.structured_md, aa.ocr_text) AS content_md,
    COALESCE(ec.raw_text, aa.ocr_text) AS content_text,
    ec.status AS extraction_status,
    ec.quality_level,
    ec.extractor_type
FROM assets a
LEFT JOIN extracted_content ec ON ec.asset_id = a.id
LEFT JOIN ai_analyses aa ON aa.asset_id = a.id;
```

#### `fts_content` — 全文索引

```sql
CREATE VIRTUAL TABLE IF NOT EXISTS fts_content USING fts5(
    raw_text,
    content='extracted_content', content_rowid='rowid'
);

-- FTS 触发器
CREATE TRIGGER IF NOT EXISTS fts_content_ai AFTER INSERT ON extracted_content
WHEN new.raw_text IS NOT NULL BEGIN
    INSERT INTO fts_content(rowid, raw_text) VALUES (new.rowid, new.raw_text);
END;
CREATE TRIGGER IF NOT EXISTS fts_content_ad AFTER DELETE ON extracted_content BEGIN
    INSERT INTO fts_content(fts_content, rowid, raw_text) VALUES ('delete', old.rowid, old.raw_text);
END;
CREATE TRIGGER IF NOT EXISTS fts_content_au AFTER UPDATE OF raw_text ON extracted_content
WHEN new.raw_text IS NOT NULL BEGIN
    INSERT INTO fts_content(fts_content, rowid, raw_text) VALUES ('delete', old.rowid, old.raw_text);
    INSERT INTO fts_content(rowid, raw_text) VALUES (new.rowid, new.raw_text);
END;
```

---

## API 设计（IPC 命令）

### 新增 Tauri Commands

| 命令 | 签名 | 描述 |
|------|------|------|
| `extract_asset` | `(asset_id: String) → Result<String, String>` | 手动触发单个素材提取，返回 task_id |
| `extract_project_assets` | `(project_id: String) → Result<String, String>` | 批量提取项目内所有待提取素材，返回 batch_id |
| `get_extraction_status` | `(asset_id: String) → Result<ExtractionStatus, String>` | 查询素材提取状态 |
| `get_extracted_content` | `(asset_id: String) → Result<Option<ExtractedContent>, String>` | 获取提取内容（Markdown + 元数据） |
| `retry_extraction` | `(asset_id: String) → Result<String, String>` | 重试失败的提取任务 |
| `get_pipeline_progress` | `() → Result<PipelineProgress, String>` | 获取管道全局进度（活跃任务数、完成数、失败数） |

### Tauri Events

| 事件名 | Payload | 描述 |
|--------|---------|------|
| `extraction:progress` | `{ asset_id, status, progress_pct, message }` | 单个素材提取进度 |
| `extraction:completed` | `{ asset_id, quality_level, extractor_type }` | 提取完成 |
| `extraction:failed` | `{ asset_id, error_message, retry_count }` | 提取失败 |
| `extraction:batch_progress` | `{ batch_id, total, completed, failed, remaining }` | 批量进度 |

---

## 目录结构规划

```
src-tauri/src/
├── extraction/                    # 新模块：提取管道
│   ├── mod.rs                     # 模块声明 + Extractor trait 定义
│   ├── scheduler.rs               # PipelineScheduler 实现
│   ├── models.rs                  # ExtractionResult, ExtractionStatus 等模型
│   └── extractors/
│       ├── mod.rs                 # 提取器注册表
│       ├── pdf_text.rs            # PDF 文字型提取器
│       ├── image_ocr.rs           # 图片 OCR 提取器
│       └── pdf_scan.rs            # PDF 扫描型提取器
├── macos/
│   └── ocr_bridge.swift           # Vision Framework OCR Swift 桥接
├── commands/
│   └── extraction.rs              # 新增：提取相关 IPC 命令
├── db/
│   └── extraction.rs              # 新增：extracted_content + pipeline_tasks CRUD

src/
├── stores/
│   └── extractionStore.ts         # 新增：提取状态管理
├── types/
│   └── extraction.ts              # 新增：提取相关类型定义
├── components/features/
│   └── extraction/                # 新增：提取相关 UI 组件
│       └── ExtractionBadge.tsx    # 提取状态角标组件
```

---

## 安全考量

1. **文件路径校验**：所有传入的文件路径必须验证在 NoteCaptWorkPlace 工作区内，防止路径遍历
2. **SQLite 并发**：使用现有 `Database` 的 `Mutex<Connection>` 单连接模式，`busy_timeout` 设为 5000ms
3. **Vision Framework 权限**：macOS Vision Framework 不需要额外权限声明（使用本地文件）
4. **FTS 注入防御**：搜索查询参数化，FTS5 MATCH 表达式转义双引号

---

## 风险登记表

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|----------|
| Swift OCR 桥接编译失败 | 中 | 高 | Day 3 checkpoint；降级为 `osascript` 调用 |
| pdf-extract 对中文 PDF 提取质量差 | 中 | 中 | 加入字数阈值检测，不足则自动切换扫描型流程 |
| pdfium-render 体积过大 | 中 | 中 | 评估替代方案：`pdf` crate 逐页渲染 or Vision 直接处理 PDF |
| FTS5 默认 tokenizer 中文搜索体验差 | 高 | 低 | MVP 接受，P1 引入 jieba-rs |
| 大 PDF（>500页）提取超时 | 低 | 中 | 分页提取 + 进度推送 |

---

## Task 清单

- [ ] task_002_db_v5_migration — V5 迁移：extracted_content + pipeline_tasks + VIEW + FTS5
- [ ] task_003_extractor_trait — 提取器 trait 定义 + 数据模型 + extraction 模块骨架
- [ ] task_004_pdf_text_extractor — PDF 文字型提取器（pdf-extract 集成）
- [ ] task_005_vision_ffi — macOS Vision OCR Swift-Rust 桥接
- [ ] task_006_image_ocr_extractor — 图片 OCR 提取器（封装 Vision FFI）
- [ ] task_007_pdf_scan_extractor — PDF 扫描型提取器（渲染 + OCR）
- [ ] task_008_pipeline_scheduler — 管道调度器（任务队列 + 顺序执行 + 恢复 + 事件）
- [ ] task_009_fts_content_search — FTS5 内容索引 + search_all 扩展
- [ ] task_010_frontend_extraction — 前端类型 + extractionStore + IPC 封装
- [ ] task_011_inspector_content — Inspector 内容预览 + 提取状态角标
- [ ] task_012_dropzone_auto_extract — Dropzone 导入自动入队 + Toolbar 进度
- [ ] task_013_integration_test — 端到端集成测试 + Bug 修复

## Task 依赖拓扑

```
task_002 (DB V5) ──→ task_003 (Trait) ──→ task_004 (PDF Text)
                                      ├──→ task_006 (Image OCR) ← task_005 (Vision FFI)
                                      └──→ task_007 (PDF Scan)  ← task_005, task_004
                 ──→ task_008 (Scheduler) ← task_003
                 ──→ task_009 (FTS) ← task_002
                 ──→ task_010 (Frontend) ← task_002
                                      └──→ task_011 (Inspector) ← task_010
                                      └──→ task_012 (Dropzone)  ← task_008, task_010

task_013 (Integration) ← 全部

关键路径: task_002 → task_003 → task_005(4d) → task_006 → task_007 = 最长
并行分支:
  A: task_002 → task_003 → task_004 (PDF Text)
  B: task_005 (Vision FFI) — 可与 A 并行
  C: task_002 → task_009 (FTS) — 可与 A/B 并行
  D: task_002 → task_010 → task_011 (前端) — 可与 A/B 并行
```

---

## Conductor 可用信息

### 新增 Cargo 依赖

```toml
pdf-extract = "0.7"    # PDF 文字提取，纯 Rust
```

### build.rs 需要修改

添加 Swift 编译步骤（task_005 负责实现）

### capabilities/default.json 需要修改

添加 `extraction:*` 事件权限（task_008 负责）
