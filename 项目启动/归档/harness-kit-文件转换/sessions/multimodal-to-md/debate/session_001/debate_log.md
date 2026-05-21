# Debate Log — 多模态文件转换引擎

> Session: multimodal-to-md
> 日期: 2026-04-11
> Host: 主持人 Agent
> 主题: 为 NCdesktop 构建多模态文件→Markdown 转换引擎

---

## Layer 1 共识（问题定义）

### 1.1 核心问题定义

**经过 3 轮辩论确认的核心定义：**

这个引擎解决的是 **「知识管道的本地可执行底座」** 问题——不是简单的格式转换工具，也不是完整的知识图谱系统，而是：

1. **离线可用的内容索引层**：将多模态素材（照片、PDF、录音、EPUB、TXT）的内容提取为结构化中间表示，使其可被搜索、引用、关联
2. **LLM 增强管道的输入端**：提取的结构化内容为 LLM 提供高质量输入，支持摘要、分类、标签推断等增强操作
3. **面向用户的 Markdown 渲染**：结构化中间层的一种呈现格式，支持复制粘贴到外部 AI 工具

**核心价值不在于"格式转换"本身**（GPT-4o/Gemini 已原生多模态），而在于**本地化 + 结构化 + 可索引 + 离线可用**的组合价值。

### 1.2 系统边界

**范围内（In Scope）：**

| 输入格式 | 细分 | 优先级 | 转换方式 |
|---|---|---|---|
| 图片 | JPEG, PNG, HEIC, WebP | P0 | macOS Vision OCR |
| PDF（文本型） | application/pdf | P0 | pdf-extract（纯 Rust） |
| PDF（扫描型） | application/pdf | P0 | Vision OCR 逐页 |
| TXT/MD | text/* | P0 | 直接读取（已实现） |
| 音频 | MP3, M4A, WAV, AAC | P1 | Speech Framework 保底 + Whisper 按需下载 |
| EPUB | application/epub+zip | P2 | epub-rs（纯 Rust） |

**明确不在范围内（Out of Scope）：**
- 视频转录、实时翻译、手写体精确 OCR、Office 格式（DOCX/PPTX/XLSX）、双向转换

**输出目标：**
- 主输出：`extracted_content` 结构化中间表（raw_text + structured_md + segments_json）
- 面向用户：Markdown 渲染
- 面向搜索：FTS5 全文索引
- 面向 LLM：JSON segments

### 1.3 关键约束与取舍

**不可能三角解法（三轮共识）：**

| 约束 | 优先级 | 让步程度 |
|---|---|---|
| 打包体积 ≤ 15MB | **硬约束，不让步** | — |
| 离线可用 | 硬约束，部分让步 | PDF/OCR/EPUB 完全离线；ASR 离线需首次下载 Whisper 模型 |
| 零配置 | 硬约束，部分让步 | PDF/OCR/EPUB 零配置；ASR 首次使用需确认下载 |

**体积预算验证：** ~9.2MB（Tauri 基础包 + pdf-extract + epub-rs + FFI 桥接），Whisper 模型 75-142MB 不打包、按需下载。

### 1.4 三层管道架构

```
Extract（离线，确定性）→ Enhance（在线，LLM，用户主动触发）→ Index（离线，自动触发）
```

- Extract 产物：`extracted_content` 表
- Enhance 产物：`ai_enrichments` 表（从旧 `ai_analyses` 拆分而来）
- Index 产物：`fts_content` FTS5 虚拟表 + `asset_tags`

### 1.5 数据模型核心决策

- **`ai_analyses` 处理方式**：Split & Replace，V4 迁移拆为 `extracted_content` + `ai_enrichments`
- **`segments_json`**：Envelope Schema v1 + Rust typed enum（5 种 segment 类型）
- **`extracted_content.status`**：5 值 CHECK（pending/extracting/extracted/failed/unsupported）
- **增强历史**：`ai_enrichments` UPSERT 保最新 + `enrichment_log` 追加历史
- **管道恢复**：`pipeline_tasks` 持久化队列 + 启动恢复 + 自动重试

### 1.6 质量分级（Markdown 输出层级）

| 输入格式 | 离线保底 | LLM 增强可达 |
|---|---|---|
| PDF（文本型） | Level 2（标题+列表+表格） | Level 3（语义理解） |
| PDF（扫描型） | Level 1（段落分割） | Level 3 |
| 图片 OCR | Level 1（段落分割） | Level 3 |
| 音频转录 | Level 0（纯文本+时间戳） | Level 3 |
| EPUB | Level 2（章节结构） | Level 3 |

### 1.7 用户与核心场景（优先级排列）

1. **P0** - 课堂照片批量 OCR → Markdown
2. **P0** - PDF 课件文字提取 → Markdown
3. **P1** - 课堂录音转文字 → Markdown
4. **P1** - 混合素材一键导出（升级现有 export_project_markdown）
5. **P2** - EPUB 教材提取 → Markdown

### 1.8 成功标准

| 指标 | MVP 目标 |
|---|---|
| PDF 文字提取准确率 | ≥ 95%（文字型） |
| 图片 OCR 准确率 | ≥ 85%（印刷体中英文） |
| 音频 ASR 准确率 | ≥ 80%（安静环境普通话） |
| 转换速度 - OCR | ≤ 2s/张 |
| 转换速度 - PDF | ≤ 1s/10 页（文字型） |
| 体积增量 | ≤ 3MB（不含 ASR 模型） |
| 零崩溃率 | 失败不崩溃，返回明确错误信息 |

### 1.9 标签系统定位

独立处理阶段（非管道副作用），三来源分层：
1. 文件元数据标签（离线，自动，100%）
2. 提取内容关键词标签（离线，TF 规则，精确率>70%）
3. LLM 智能分类标签（在线，复用 llm_classify）

---

## 论证追踪表（Layer 1 最终版）

| 论点 | 提出方 | 层级 | 状态 | 备注 |
|---|---|---|---|---|
| 核心痛点是"知识采集最后一公里断裂" | Proposer | L1 | ✅ 已验证 | 修正为"本地知识底座"定位 |
| 纯格式转换在 LLM 多模态时代缺乏壁垒 | Reviewer | L1 | ✅ 已验证 | 推动 Proposer 修正定位，核心价值 = 本地化+结构化+可索引 |
| 体积/离线/零配置不可能三角 | Reviewer | L1 | ✅ 已验证 | 解法：macOS Vision 零体积 OCR + Whisper 按需下载 |
| macOS Vision OCR 作为 P0 核心提取器 | Proposer | L1 | ✅ 已验证 | macOS 14+ 中文 OCR 可用，置信度回传 |
| Whisper 按需下载策略 | Proposer | L1 | ✅ 已验证 | 符合 macOS 生态惯例 |
| 质量分级 Level 0-3 | Reviewer→Proposer | L1 | ✅ 已验证 | 每种格式明确离线保底和 LLM 增强层级 |
| 三层管道：Extract→Enhance→Index | Proposer | L1 | ✅ 已验证 | 增强为用户主动触发 |
| ai_analyses Split & Replace | Proposer | L1 | ✅ 已验证 | V4 迁移拆为 extracted_content + ai_enrichments |
| segments_json Envelope Schema v1 | Proposer | L1 | ✅ 已验证 | Rust typed enum + 前端 discriminated union |
| pipeline_tasks 持久化队列 | Proposer | L1 | ✅ 已验证 | 启动恢复 + 自动重试 |
| macOS 原生 API 跨版本质量风险 | Reviewer | L1 | ⏸️ 搁置 | P2/运维：版本发布前人工基准测试 |
| quality_level 对混合文档不充分 | Reviewer | L1 | ⏸️ 搁置 | P2：可下沉到 segment 级别 |
| 并发度按操作类型区分 | Reviewer | L1 | ⏸️ 搁置 | P1 实现优化 |
| 管道幂等与并发安全 | Reviewer | L1 | ⏸️ 搁置 | 假设单实例，Layer 2 确认 |
| FTS 中文分词策略 | Reviewer | L1 | ⏸️ 搁置 | Layer 2 讨论 |
| enrichment_log 保留/裁剪策略 | Reviewer | L1 | ⏸️ 搁置 | P2 运维 |
| "本地知识底座"的护城河具化 | Reviewer | L1 | ⏸️ 搁置 | Watch Item，不阻塞 Layer 1 |

---

## 层间过渡验证（Layer 1 → Layer 2）

- [x] 当前层无"❓ 待定"状态的核心定义
- [x] 所有"⏸️ 搁置"项已明确标注为 P2/out-of-scope 并获得共识
- [x] 本层共识可以被直接引用为下一层的讨论基础
- [x] 论证追踪表已更新

---

**进入 Layer 2: 理想态...**

---

## Layer 2 共识（理想态）

### 2.1 渐进增强架构（Progressive Enhancement Architecture）

理想态的核心不在于任何单一子系统的极致性能，而在于 **L0→L1→L2→L3 每一层都独立可用、独立有价值**：

- **L0 元数据提取**（离线，100%）：文件类型/大小/日期/EXIF
- **L1 内容提取**（离线，85-95%）：OCR/PDF/ASR 本地提取
- **L2 AI 增强**（在线优先，离线降级 60-70%）：概念提取、语义标签
- **L3 知识图谱**（仅在线）：观点聚合、知识拓展

### 2.2 关键架构决策

1. **现有 V3 知识关联系统就是 Enhance 层**——不建第二套管道
2. **物理拆分在 Extract 层（确定性），语义拆分在 Enhance 层（AI）**
3. **三态编辑保护**：ai_draft → accepted → user_modified
4. **四级优雅降级**：完整提取 > 部分提取 > 元数据仅提取 > 标记失败
5. **FTS 中文分词**：jieba-rs 预分词 + FTS5 simple，80ms lazy 加载
6. **离线约束诚实声明**：L0-L1 离线可用，L2 离线降级标注，L3 仅在线

---

## Layer 3 共识（差距分析）

### 3.1 核心差距

1. **管道基础设施从 0 开始**：extracted_content / ai_enrichments / pipeline_tasks 三张表不存在
2. **内容提取能力为零**：PDF、OCR、ASR 三大提取器均未实现
3. **中文搜索不可用**：无 jieba-rs 分词

### 3.2 关键路径

~18 工作日单人串行，关键路径 T05(Vision FFI 4d) → T06(1d) → T07(1.5d)

### 3.3 最大风险

- 技术：macOS Vision FFI 桥接、SQLite 并发
- 产品：ASR 质量、用户对渐进可用的理解
- 安全：extracted_content 含敏感信息

---

## Layer 4 共识（策略）

### 4.1 MVP 定义

**一句话**：用户将照片和 PDF 拖入 NCdesktop，即可离线获得可搜索、可复制的结构化 Markdown。

### 4.2 分期

- **P0 (MVP)**：11 个 Task，18 工作日 — PDF + OCR + 管道 + 搜索 + UI
- **P1 (V1.1)**：12 工作日 — ASR + 中文分词 + Enhance 层 + 标签
- **P2 (V2)**：9 工作日 — Whisper + EPUB + LLM 标签 + 旧表弃用

### 4.3 裁剪红线

绝对不裁剪：extracted_content 表、PDF 提取、OCR 能力、管道调度器

---

## 最终论证追踪表

| 论点 | 提出方 | 层级 | 状态 | 备注 |
|------|--------|------|------|------|
| 核心痛点：知识采集最后一公里断裂 | Proposer | L1 | ✅ 已验证 | 修正为"本地知识底座" |
| 纯格式转换在 LLM 多模态时代缺乏壁垒 | Reviewer | L1 | ✅ 已验证 | 差异化 = 本地+结构化+可索引 |
| 体积/离线/零配置不可能三角 | Reviewer | L1 | ✅ 已验证 | Vision 零体积 + Whisper 按需下载 |
| macOS Vision OCR 作为 P0 核心提取器 | Proposer | L1 | ✅ 已验证 | 14+ 中文可用 |
| 三层管道 Extract→Enhance→Index | Proposer | L1 | ✅ 已验证 | 增强为用户主动 |
| ai_analyses Split & Replace | Proposer | L1 | ✅ 已验证 | V4 迁移，VIEW 兼容 |
| pipeline_tasks 持久化队列 | Proposer | L1 | ✅ 已验证 | 启动恢复 + 重试 |
| 渐进增强架构 L0-L3 | Proposer | L2 | ✅ 已验证 | 每层独立可用 |
| 现有 V3 知识系统 = Enhance 层 | Proposer | L2 | ✅ 已验证 | 不建第二套管道 |
| 三态编辑保护 | Proposer | L2 | ✅ 已验证 | P1 实现 |
| 四级优雅降级 | Proposer | L2 | ✅ 已验证 | 逐场景定义 |
| jieba-rs 预分词方案 | Proposer | L2 | ✅ 已验证 | 80ms 加载，P1 集成 |
| Vision FFI 估时调整为 4d | Reviewer | L4 | ✅ 已采纳 | + Day 3 checkpoint |
| segments_json P0 预留列 | Reviewer | L4 | ✅ 已采纳 | 最简 segment，+0.5d |
| ai_analyses fallback 用 VIEW | Reviewer | L4 | ✅ 已采纳 | v_asset_content |
| MVP 工期 18d 单人串行 | 共识 | L4 | ✅ 已验证 | 含 2d 集成测试 |
| quality_level 混合文档粒度 | Reviewer | L1 | ⏸️ P1 | segment 级 P1 决定 |
| 管道幂等与并发安全 | Reviewer | L1 | ⏸️ P1 | 单实例假设 |
| FTS 中文分词时机 | Reviewer | L2 | ⏸️ P1 | Reviewer 建议 P0，最终定 P1 |
| enrichment_log 保留策略 | Reviewer | L1 | ⏸️ P2 | 运维级 |
| 本地知识底座护城河具化 | Reviewer | L1 | ⏸️ Watch | 长期 |

---

## PRD 产出

最终 PRD 已写入：`sessions/multimodal-to-md/prd/multimodal-extraction-prd-v1.md`

包含完整的：
1. 项目概述与核心定位
2. 用户定义与 5 个核心场景
3. 功能需求（20 项，带优先级）
4. 数据模型（DDL + Schema + 生命周期）
5. 非功能需求（性能/安全/可靠性）
6. 技术约束
7. 分期计划（P0/P1/P2 + Task 清单）
8. 风险登记册
9. **Conductor 桥接摘要**（含核心功能清单、技术底线、高风险项、MVP 边界、未达共识争议）
