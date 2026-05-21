# NCdesktop 多模态文件转换引擎 PRD v1.0

> 产出自 Debate Session `multimodal-to-md`
> 日期: 2026-04-11
> 经过 4 层辩论（问题定义 → 理想态 → 差距分析 → 策略），共 8 轮交锋

---

## 第一章 · 项目概述

### 1.1 核心定位

NCdesktop 多模态文件转换引擎是一个 **本地可执行的知识底座 + LLM 增强管道的输入端**。

它不是一个简单的格式转换工具，而是将多模态素材（照片、PDF、录音、EPUB）的内容提取为结构化中间表示，使其：
- **可搜索**：FTS5 全文索引，中文分词支持
- **可引用**：每个搜索结果可溯源到原始素材的精确位置
- **可导出**：结构化 Markdown，可直接喂给 NotebookLM / ChatGPT / Claude
- **离线可用**：核心提取层完全本地运行，不依赖网络

### 1.2 核心痛点

NCdesktop 当前的素材管理系统能浏览素材但**无法提取素材内容**。用户的工作流在「素材 → 可被 AI 消费的结构化文本」这个环节断裂：

- PDF 课件只能预览，无法提取文字
- 照片里的板书无法被搜索和引用
- 课堂录音无法转为文字
- EPUB 教材无法处理

### 1.3 核心价值（vs 竞品 LLM 原生多模态）

在 GPT-4o / Gemini 原生支持多模态输入的时代，本引擎的差异化价值在于：

| 维度 | LLM 原生多模态 | NCdesktop 本地底座 |
|------|---------------|-------------------|
| 离线可用 | ❌ 必须在线 | ✅ 完全离线 |
| 内容可索引 | ❌ 一次性消费 | ✅ FTS5 持久化索引 |
| 内容可溯源 | ❌ 无原始定位 | ✅ 页码/时间戳/bbox |
| 成本 | 按 token 计费 | 零成本（本地计算） |
| 隐私 | 数据上传云端 | 数据不离开本地 |

---

## 第二章 · 用户定义与核心场景

### 2.1 目标用户

大学生和研究生，使用 Omni/Arca 硬件或手机在课堂/实验室/研讨会中采集多模态知识碎片。

### 2.2 核心场景（按优先级）

**P0 场景 #1：课堂照片批量 OCR**

> 小明上完高数课，拍了 15 张板书照片。拖入 NCdesktop，自动 OCR，每张 1-2 秒出结果。搜索"微积分"立刻命中包含该词的照片。复制提取文字粘贴到 NotebookLM。

**P0 场景 #2：PDF 课件文字提取**

> 小红拖入 50 页 PDF 课件，一键提取全文，保留标题层级和列表结构，得到结构化 Markdown。直接导出喂给 ChatGPT 做章节总结。

**P1 场景 #3：课堂录音转文字**

> 小刚拖入 90 分钟录音，后台转录，带时间戳的转录文本可搜索、可跳转播放。选取重点段落导出 Markdown。

**P1 场景 #4：混合素材一键导出**

> 小李有一个项目，包含录音转录、照片 OCR、PDF 提取、手写笔记。点击"导出项目 Markdown"，所有内容按时间轴自动组装为完整文档。

**P2 场景 #5：EPUB 教材提取**

> 小王导入 EPUB 电子教材，按章节提取为 Markdown，喂给 AI 做知识图谱。

---

## 第三章 · 功能需求

### 3.1 三层管道架构

```
┌─────────────────────────────────────────────────────────────┐
│ Layer 1: EXTRACT（离线，确定性）                              │
│ ─ PDF 文字提取 (pdf-extract)                                 │
│ ─ PDF 扫描型 → 逐页渲染 → OCR                               │
│ ─ 图片 OCR (macOS Vision Framework)                         │
│ ─ 音频 ASR (Speech Framework / Whisper)          [P1]       │
│ ─ EPUB 解析 (epub-rs)                            [P2]       │
│                                                              │
│ 产出：extracted_content 表                                    │
├─────────────────────────────────────────────────────────────┤
│ Layer 2: ENHANCE（在线，LLM，用户主动触发）       [P1]       │
│ ─ OCR 纠错 + 标点补全                                        │
│ ─ 段落/标题结构重组                                          │
│ ─ 摘要生成                                                   │
│ ─ 语义标签推断                                               │
│                                                              │
│ 产出：ai_enrichments 表 + concepts 更新                      │
├─────────────────────────────────────────────────────────────┤
│ Layer 3: INDEX（离线，自动触发）                              │
│ ─ FTS5 全文索引更新                                          │
│ ─ 关键词标签提取                                    [P1]     │
│                                                              │
│ 产出：fts_content + asset_tags                               │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 功能清单（带优先级）

| ID | 功能 | 优先级 | 层级 | 描述 |
|----|------|--------|------|------|
| F01 | PDF 文字提取 | P0 | Extract | pdf-extract 提取文字型 PDF → raw_text + structured_md |
| F02 | 图片 OCR | P0 | Extract | macOS Vision Framework 识别中英文，返回文字 + 置信度 |
| F03 | PDF 扫描型提取 | P0 | Extract | 逐页渲染为图片 → Vision OCR，支持混合型 PDF |
| F04 | 管道调度器 | P0 | 基础设施 | 顺序执行任务队列，启动恢复，失败重试，进度事件 |
| F05 | FTS5 全文索引 | P0 | Index | extracted_content.raw_text 自动索引，接入全局搜索 |
| F06 | Inspector 内容预览 | P0 | UI | 提取的 Markdown 渲染 + 提取状态指示 |
| F07 | 提取状态角标 | P0 | UI | AssetListView 素材卡片显示 pending/extracting/extracted/failed |
| F08 | Dropzone 自动提取 | P0 | 集成 | 素材拖入后自动入队提取 + Toolbar 进度 |
| F09 | 音频转录 (Speech) | P1 | Extract | macOS Speech Framework 保底 ASR |
| F10 | jieba-rs 中文分词 | P1 | Index | 预分词写入 FTS5，中文搜索质量提升 |
| F11 | ai_enrichments 审计 | P1 | Enhance | LLM 增强结果审计日志 |
| F12 | 三态编辑保护 | P1 | Enhance | ai_draft → accepted → user_modified |
| F13 | 关键词标签提取 | P1 | Index | 文件元数据 + TF 规则自动标签 |
| F14 | segments_json 完整 Schema | P1 | 基础设施 | Envelope v1 + 5 种 segment 类型 |
| F15 | 任务中心 UI | P1 | UI | Toolbar 任务面板，活跃/完成/失败任务管理 |
| F16 | Whisper 按需下载 | P2 | Extract | 高精度离线 ASR，模型 75-142MB |
| F17 | EPUB 解析 | P2 | Extract | epub-rs 章节提取 |
| F18 | LLM 智能标签 | P2 | Enhance | 复用 llm_classify，输入 extracted_content |
| F19 | ai_analyses 完全弃用 | P2 | 基础设施 | 数据迁移 + 删除旧表 |
| F20 | export_project_markdown 升级 | P2 | 集成 | 合并提取内容到导出 |

### 3.3 输入格式支持

| 格式 | 细分 | 优先级 | 提取方式 | 离线保底质量 |
|------|------|--------|----------|-------------|
| 图片 | JPEG, PNG, HEIC, WebP | P0 | macOS Vision OCR | Level 1（段落分割） |
| PDF（文字型） | application/pdf | P0 | pdf-extract（纯 Rust） | Level 2（标题+列表+表格） |
| PDF（扫描型） | application/pdf | P0 | Vision OCR 逐页 | Level 1 |
| TXT/MD | text/* | P0 | 直接读取（已实现） | Level 2 |
| 音频 | MP3, M4A, WAV, AAC | P1 | Speech Framework | Level 0（纯文本+时间戳） |
| EPUB | application/epub+zip | P2 | epub-rs | Level 2（章节结构） |

### 3.4 质量分级定义

| Level | 名称 | 描述 | 依赖 |
|-------|------|------|------|
| Level 0 | 纯文本 | 连续无格式字符串 | 离线 |
| Level 1 | 段落分割 | 段落分隔正确（`\n\n`），无标题层级 | 离线 |
| Level 2 | 结构化 | `#` 标题、`-` 列表、`|` 表格 | 离线 |
| Level 3 | 语义理解 | 纠错、标点补全、主题分段、摘要 | LLM 在线 |

---

## 第四章 · 数据模型

### 4.1 新增表（V4 迁移）

#### `extracted_content` — Extract 层产物

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
```

#### `pipeline_tasks` — 持久化任务队列

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
```

#### `v_asset_content` — 兼容性视图

```sql
CREATE VIEW v_asset_content AS
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

#### `fts_content` — FTS5 全文索引

```sql
CREATE VIRTUAL TABLE IF NOT EXISTS fts_content USING fts5(
    raw_text,
    content='extracted_content', content_rowid='rowid'
);
```

### 4.2 `ai_analyses` 过渡策略

- **P0**：新增 `extracted_content`，`ai_analyses` 只读保留，通过 `v_asset_content` VIEW 统一读取
- **P1**：提供"重新提取"功能，用户可将旧素材迁移到新表
- **P2**：`ai_analyses` 完全弃用，数据迁移后删除

### 4.3 segments_json Schema（P0 最简版）

P0 使用最简 segment 数组，P1 升级为完整 Envelope Schema v1：

```json
// P0 最简版
[{"type": "text", "content": "...", "page": 1}]

// P1 Envelope Schema v1
{
  "schema_version": 1,
  "extractor": "pdf_text",
  "total_segments": 42,
  "segments": [
    { "type": "text_block", "index": 0, "content": "...", "page": 1, "confidence": null },
    { "type": "ocr_region", "index": 1, "content": "...", "bbox": [120,340,580,720], "confidence": 0.87 }
  ]
}
```

### 4.4 Asset 内容处理生命周期

```
imported → extracting → extracted → [enhancing → enhanced] → indexed
                      → failed (可重试)
                      → unsupported (终态)
```

---

## 第五章 · 非功能需求

### 5.1 性能

| 指标 | 目标 | 约束级别 |
|------|------|----------|
| 冷启动 | < 500ms | **硬约束** |
| UI 响应 | 提取不阻塞 UI | **硬约束** |
| OCR 速度 | ≤ 2s/张 (1200 万像素, M1) | 软目标 |
| PDF 提取速度 | ≤ 1s/10 页 (文字型) | 软目标 |
| 搜索响应 | < 200ms (10,000 条记录) | 软目标 |
| 打包体积 | ≤ 15MB (不含 ASR 模型) | **硬约束** |
| 空闲内存 | < 80MB | 软目标 |

### 5.2 安全与隐私

- `extracted_content` 含用户素材的明文内容，需考虑本地加密存储（P2: SQLCipher）
- LLM 增强时发送的内容需用户知情同意
- Whisper 模型下载走 HTTPS + SHA256 校验

### 5.3 可靠性

- 管道任务持久化到 SQLite，进程崩溃后重启自动恢复
- 失败任务自动重试（最多 3 次）
- 四级优雅降级：完整提取 > 部分提取 > 元数据仅提取 > 标记失败

---

## 第六章 · 技术约束

| 约束 | 来源 | 影响 |
|------|------|------|
| Tauri v2 (Rust + React/TS) | 项目宪章 | 提取器用 Rust 实现 |
| macOS (Apple Silicon 优先) | 项目宪章 | 可用 Vision/Speech Framework |
| SQLite (rusqlite, bundled) | 项目宪章 | 数据存储和 FTS5 |
| 打包体积 ≤ 15MB | 项目宪章 | Whisper 模型不打包，按需下载 |
| 冷启动 < 500ms | 项目宪章 | jieba-rs 懒加载，不阻塞启动 |
| 现有 ai_analyses 表 | V1 遗产 | 需 VIEW 兼容过渡 |
| 现有 concepts/viewpoints 系统 | V3 遗产 | 作为 Enhance 层实体保留 |

---

## 第七章 · 分期计划

### P0 (MVP) — 离线内容提取底座 | 18 工作日

**目标**：用户将照片和 PDF 拖入 NCdesktop，即可离线获得可搜索、可复制的结构化 Markdown。

| Task ID | 名称 | 描述 | 依赖 | 工时 |
|---------|------|------|------|------|
| T01 | DB: extracted_content + VIEW | 建表 + V4 迁移 + v_asset_content VIEW | 无 | 1.5d |
| T02 | DB: pipeline_tasks | 任务队列表 + 迁移（与 T01 合并） | T01 | 0.5d |
| T03 | PipelineScheduler 骨架 | 顺序执行 + 启动恢复 + 失败重试 + Event emit | T02 | 2d |
| T04 | PDF 文字提取器 | pdf-extract 集成 + 扫描型检测 | T01 | 2d |
| T05 | macOS Vision OCR FFI | swift-bridge 桥接 + VNRecognizeTextRequest | 无 | 4d |
| T06 | 图片 OCR 提取器 | Vision 封装为标准提取器接口 | T05 | 1d |
| T07 | PDF 扫描型提取器 | 逐页渲染 + OCR，混合型支持 | T04, T06 | 1.5d |
| T08 | FTS5 全文索引 | fts_content + 触发器 + 搜索集成 | T01 | 1d |
| T09 | Inspector 预览 + 状态 UI | Markdown 渲染 + 提取状态角标 + 重试按钮 | T01 | 1.5d |
| T10 | Dropzone 集成 + 进度 | 自动入队 + Toolbar 进度 + Toast | T03 | 1d |
| T11 | 集成测试 + Bug 修复 | 端到端全链路测试 | 全部 | 2d |

**关键路径**：T05(4d) → T06(1d) → T07(1.5d) = 6.5d

**并行分支**：T01→T04(PDF) 可与 T05(Vision) 同时进行

### P1 (V1.1) — 增强体验 + ASR | 12 工作日

- ai_enrichments + 三态编辑保护
- jieba-rs 中文分词
- 音频 ASR (Speech Framework)
- segments_json Envelope Schema v1
- 关键词标签提取
- 任务中心 UI

### P2 (V2) — 完善生态 | 9 工作日

- Whisper 按需下载
- EPUB 解析
- LLM 智能标签
- ai_analyses 完全弃用
- export_project_markdown 升级

---

## 第八章 · 风险登记册

| 风险 | 概率 | 影响 | 来源 | 缓解策略 |
|------|------|------|------|----------|
| macOS Vision FFI 桥接复杂度超预期 | 中 | 高 | Debate L3 | swift-bridge 方案 + Day 3 checkpoint + CLI 降级备选 |
| SQLite 并发写冲突 | 中 | 中 | Debate L3 | 单连接池 Mutex + busy_timeout 5000ms |
| jieba-rs 词典 4MB 导致体积接近上限 | 低 | 低 | Debate L2 | 编译时嵌入 or 首次启动解压 |
| ASR 质量不达预期 | 高 | 高 | Debate L3 | Speech Framework 保底 + Whisper 升级路径 |
| extracted_content 含敏感信息 | 高 | 高 | Debate L3 | P2 SQLCipher 加密 + is_sensitive 标记 |

---

## Conductor 桥接摘要

### 核心功能清单（带优先级）

| 功能 | 优先级 | 核心用户场景 | 来自 Debate 的关键约束 |
|------|--------|-------------|----------------------|
| PDF 文字提取 | P0 | PDF 课件 → 可搜索 Markdown | 纯 Rust (pdf-extract), 零系统依赖 |
| 图片 OCR | P0 | 课堂照片 → 可搜索文字 | macOS Vision Framework, 零体积增量 |
| PDF 扫描型提取 | P0 | 扫描 PDF → OCR → Markdown | 逐页检测 + 混合型支持 |
| 管道调度器 | P0 | 后台异步提取, 不阻塞 UI | 顺序执行, 持久化, 启动恢复 |
| FTS5 全文索引 | P0 | 搜索提取的文字内容 | 替补现有 FTS, 中文 P1 优化 |
| Inspector 预览 | P0 | 查看提取的 Markdown | 状态角标 + 重试 UI |
| Dropzone 集成 | P0 | 拖入即提取, 零配置 | Tauri Event 进度推送 |
| 音频转录 | P1 | 课堂录音 → 文字 | Speech Framework 保底 |
| 中文分词 | P1 | 中文搜索质量 | jieba-rs 预分词 |
| Enhance 层 | P1 | LLM 增强提取结果 | 用户主动触发, 不自动消耗 API |
| Whisper ASR | P2 | 高精度离线转录 | 模型按需下载 75-142MB |
| EPUB 解析 | P2 | 电子教材提取 | epub-rs 纯 Rust |

### 不可妥协的技术底线

1. **打包体积 ≤ 15MB**（Whisper 模型不打包）
2. **冷启动 < 500ms**（jieba-rs 懒加载，Vision/Whisper 后台初始化）
3. **提取不阻塞 UI**（独立 async task，Tauri Event 推进度）
4. **ai_analyses 只读保留**（V4 迁移不删旧表，通过 VIEW 兼容）
5. **人工编辑永不被 AI 覆盖**（三态编辑保护 P1 实现）

### 已识别的高风险项

| 风险 | 来源 | 当前状态 | 缓解策略 |
|------|------|----------|----------|
| Vision FFI 桥接复杂度 | Debate L3 R1 | 待验证 | swift-bridge + Day 3 checkpoint + CLI 降级 |
| ASR 质量不达预期 | Debate L3 R2 | 待验证 | Speech 保底 + Whisper 升级 |
| SQLite 并发写冲突 | Debate L3 R1 | 待验证 | 单连接 Mutex + busy_timeout |
| jieba 词典体积 | Debate L2 R4 | 已搁置 | P1 验证，超限则外置词典 |

### MVP 边界声明

**做什么**：
- PDF 文字/扫描型提取 → extracted_content
- 图片 OCR → extracted_content
- FTS5 全文索引 + 搜索集成
- 管道调度器（顺序执行 + 恢复 + 重试）
- Inspector 内容预览 + 提取状态 UI
- Dropzone 自动入队 + 进度反馈

**不做什么**：
- ❌ 音频 ASR（P1，技术风险最高）
- ❌ EPUB 解析（P2，低频场景）
- ❌ Enhance 层 / ai_enrichments（P1，依赖在线 LLM）
- ❌ jieba-rs 中文分词（P1，FTS5 默认 tokenizer 足够 MVP）
- ❌ 三态编辑保护（P1，需要 Enhance 层配合）
- ❌ enrichment_log 审计（P1）
- ❌ 统一任务中心 UI（P1，MVP 用 Toast + Inspector 内嵌）
- ❌ 并发提取优化（P1，MVP 顺序执行足够）

### Debate 中未达成共识的争议

1. **Vision FFI 方案选择**（swift-bridge vs objc2 vs raw FFI）——Architect 需在 T05 开始前做 spike 验证
2. **quality_level 粒度**（整文档级 vs segment 级）——P0 用整文档级，Reviewer 认为混合文档需要 segment 级，P1 决定
3. **FTS 中文分词时机**（P0 vs P1）——Reviewer 认为应在 P0，最终决定 P1 优化，P0 用默认 tokenizer
