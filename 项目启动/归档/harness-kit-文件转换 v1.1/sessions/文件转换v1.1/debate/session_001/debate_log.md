# Debate Log — 文件转换 v1.1
# Session 001 | 日期：2026-04-12

---

## 背景

本次 Debate 由 PM（用户）启动，触发原因：NCdesktop P0 文件转换功能交付后，使用过程中发现**核心体验偏差**——

> 用户反馈原话："用户更希望的是『把不同格式的文件拖拽进工作区以后，就能够在工作区中看见已经被变成 md 格式的对应版本，这样我就不需要重新再转换一次』，所以用户需要的不是提取，而是直接转换成结果。"

参与方：Host（主持人）、Proposer（功能设计方）、Reviewer（质疑方）
> 本次 Debate 由 Host 同时扮演 Proposer 和 Reviewer 视角，对话结构化输出。

---

---

## [Layer 1 — 问题定义]

### Round 1

**Host 向 Proposer 提问：**
现有 P0 实现中，用户拖入文件后能在工作区看到什么？提取结果存在哪里？用户的实际路径是什么？

**Proposer 回答：**

现有流水线分三层：
1. `dropzone.rs`：复制原文件到工作区 → 创建 Asset 记录（DB）
2. `spawn_dropzone_ai_job`：LLM 分类 + 重命名 + 打标（写 DB）
3. `PipelineScheduler`：提取文件内容 → `structured_md` 写入 `extracted_contents` 表（DB ONLY）

用户实际体验路径：拖入 → 等待 → 打开右侧 Inspector 面板 → 在 `InspectorExtraction` 中查看提取的文字。

**提取结果的物理位置**：仅存在于 SQLite 数据库 `extracted_contents.structured_md` 字段，没有对应的磁盘文件，没有独立的 Asset 记录。

**Host 追问 Reviewer：**
上述描述是否准确？有无遗漏？

**Reviewer 质疑：**
准确。补充一点：现有 `AssetListView` 是双栏结构（左：导入原件，右：工作区副本），右栏目前展示的是"AI 重命名后的文件名 + 标签 + ExtractionBadge 状态"——提取完成只会更新 badge 状态，不会增加新的 Asset 条目。用户要读提取内容只能通过 Inspector，且只有"只读展示"，无法当文件用。

**Host 判断：** 问题定义清晰。推进。

---

### Layer 1 共识

| 共识项 | 内容 |
|--------|------|
| 核心问题 | 提取结果（`structured_md`）停留在数据库，未物化为磁盘文件，用户无法在工作区"用"它 |
| 在范围内 | PDF、图片、Word、PPT、录音 → .md 物化 |
| 不在范围内 | 音频转写质量优化、MD 手动编辑、历史 Asset 回填 |
| 成功标准 | 拖入文件后，工作区出现 `[原文件名].md` 的 Asset 条目，打开即是可读 Markdown |
| 系统边界 | 物化发生在提取管线成功后；原文件不受影响 |

---

---

## [Layer 2 — 理想态]

### Round 2

**Host 向 Proposer 提问：**
理想的用户体验应该是什么样？技术模型的核心变化是什么？

**Proposer 回答：**

**理想用户体验：**
```
[悬浮窗] 正在转换…
↓
[工作区]
  原件：课程表.pdf    →  2026/4/12 14:32
  MD版：课程表.md     →  刚刚  ✨ 已转换
  
  点击「课程表.md」→ 直接在查看器打开 Markdown 内容
```

**核心技术模型变化：**

现有（P0）：
```
提取成功 → 写 extracted_contents.structured_md（DB）→ 结束
```

理想态（v1.1）：
```
提取成功 → 写 extracted_contents.structured_md（DB，保留）
         → 物化写出：<工作区>/<asset_id>_<stem>.md（磁盘）
         → 创建衍生 Asset（asset_type="markdown", source_type="converted_from", source_asset_id=原件ID）
         → app.emit("notecapt/asset-converted")
         → 前端 assetStore 收到事件 → 刷新 → 工作区出现新条目
```

**关键属性：**
1. 透明性：MD 版与原件有明确来源关联（`source_asset_id`）
2. 可用性：磁盘上的真实 `.md` 文件，可读可发送
3. 非破坏性：原文件始终保留
4. 懒生成：`quality_level >= 1` 且 `structured_md` 非空才物化

**Reviewer 质疑：**
工作区将出现"原件 + MD 版"两条记录，用户是否会困惑？

**Proposer 回应：**
UI 层需在右栏给衍生 MD Asset 增加"转换自 [原件名]"的来源标记，利用已有 `source_asset_id` 字段在前端做区分渲染。这是 MVP 必须解决的，不是可选项。

**Host 判断：** 充分，推进。

---

### Layer 2 共识

| 共识项 | 内容 |
|--------|------|
| 理想态三要素 | ①工作区 MD 版以独立 Asset 出现 ②有原件↔MD版关联可追溯 ③拖入即触发，零干预 |
| 技术模型核心变化 | scheduler.rs 提取成功分支增加"物化写出 + 创建衍生 Asset"步骤 |
| UI 必须解决 | 两条 Asset 记录的区分展示（"转换自"来源标记） |

---

---

## [Layer 3 — 差距分析]

### Round 3

**Host 向 Proposer 提问：**
从现有 P0 实现到理想态，主要 gap 是什么？风险点在哪里？

**Proposer 回答：**

**Gap 清单：**

| Gap | 性质 | MVP 必须 |
|-----|------|---------|
| extraction 成功后无物化写出步骤 | 核心缺失 | ✅ |
| Asset 表无 `source_asset_id` 字段 | 数据模型缺失 | ✅ |
| `dropzone.rs` MIME 映射不覆盖 docx/pptx/audio | 入口缺失 | ✅ |
| Word (.docx) 无提取器 | 新功能 | ✅ |
| PPT (.pptx) 无提取器 | 新功能 | ✅ |
| 录音无提取器（需要 ASR） | 新功能，复杂度高 | ✅（方案待定） |
| 前端无"转换版"区分展示 | UI 缺失 | ✅ |
| `extraction:completed` 事件后前端无刷新逻辑 | 前端缺失 | ✅ |

**主要风险：**

| 风险 | 严重性 | 缓解 |
|------|--------|------|
| 工作区两倍条目造成困惑 | 高 | UI 来源标记 |
| 录音 ASR 路径未定（macOS原生 vs API vs 本地模型） | 高 | Layer 4 决策 |
| 提取质量低生成垃圾 MD | 中 | quality_level 阈值过滤 |
| migration 向后兼容 | 低 | source_asset_id 可为 NULL |

**Host 追问 Reviewer：**
录音 ASR 的路径选择是 Layer 3 的核心风险，三个选项各有什么致命缺陷？

**Reviewer 分析：**
- 路径 A（macOS SFSpeechRecognizer）：离线可用，质量中等，需要 Swift FFI bridge（约 2-3 天工作量），但项目已有 `macos/ocr_ffi.rs` 的完整先例，风险可控
- 路径 B（LLM API 音频端点）：依赖用户配置 AI Key + API 支持音频（不确定性高），Ark API 是否有 Whisper 端点未验证
- 路径 C（本地 Whisper 库）：打包体积增加几百 MB，不可接受

**Host 判断：** 三个选项已足够清晰，路径决策推入 Layer 4。

---

### Layer 3 共识

| 共识项 | 内容 |
|--------|------|
| 必须解决的核心 Gap | 物化写出、source_asset_id 字段、3个新提取器、MIME 映射、前端展示 |
| 录音路径 | 三选一，需 PM 决策 |
| 历史回填 | 搁置 P1，不影响新增流程 |

---

---

## [Layer 4 — 策略]

### Round 4-A（初始策略草案）

**Proposer 初稿 MVP scope：**

初稿仅覆盖照片 + PDF（已有提取器），Word/PPT/录音搁置 P2。

**PM 修正（关键 scope 扩展）：**

> "有修正，我需要在 MVP 就实现支持照片、pdf、word、ppt、录音等多模态文件进入工作区之后，就完成了 md 格式转换的功能。"

**Host 响应：**
修正接受。MVP 范围扩大为全格式覆盖，但录音的 ASR 路径仍需 PM 选择（提出三个选项）。

---

### Round 4-B（录音路径决策）

**PM 决策：** 选项 2——macOS SFSpeechRecognizer（原生离线，复用 OCR FFI 模式）

**Host 判断：**
- 技术可行性已确认（`macos/ocr_ffi.rs` + `build.rs` 编译 Swift 的先例完整）
- 仅 macOS 发布，平台约束与选项匹配
- 推进定稿

---

### Round 4-C（最终策略定稿）

**格式覆盖矩阵（最终）：**

| 格式 | MIME | 实现路径 | 新建/复用 |
|------|------|---------|---------|
| 照片 jpg/png/webp/heic | image/* | 复用 `image_ocr.rs` | 复用 |
| PDF（文字+扫描） | application/pdf | 复用 `pdf_text.rs` + OCR fallback | 复用 |
| Word .docx | application/vnd.openxmlformats-officedocument.wordprocessingml.document | 新建 `docx.rs`：zip 解压 + `word/document.xml` 解析 | 新建 |
| PPT .pptx | application/vnd.openxmlformats-officedocument.presentationml.presentation | 新建 `pptx.rs`：zip 解压 + 逐 slide XML 解析 | 新建 |
| 录音 mp3/m4a/wav | audio/* | 新建 `audio_asr.rs`：macOS SFSpeechRecognizer FFI | 新建 |

**Task 分解（8 个，有向无环依赖图）：**

```
T01（DB migration: source_asset_id）
  └─ T02（models/asset.rs 同步适配）
       └─ T07（scheduler.rs 物化写出 + 创建衍生 Asset）
            └─ T08（前端：事件监听 + 来源标记展示）

T03（docx 提取器）──┐
T04（pptx 提取器）──┤── 共享 zip/quick-xml crate
                    └─ T06（dropzone.rs MIME 映射扩展）

T05（audio_asr FFI bridge）──┘（独立，不依赖 T03/T04）
                              └─ T06（MIME 映射扩展）
```

**不可妥协底线（最终确认）：**
1. 原始文件绝对不删除
2. quality_level = 0 或 structured_md 为空 → 不写出 .md
3. audio extractor 的 can_handle 在非 macOS 下永返 false
4. Word/PPT 本版本仅提取文字，跳过嵌入图片

---

### 论证追踪表（最终）

| 论点 | 提出方 | 层级 | 状态 | 备注 |
|------|--------|------|------|------|
| 用户需要的是物化 .md 文件，而非 DB 字段展示 | PM | L1 | ✅ 已验证 | 写入磁盘 + 创建 Asset |
| 工作区两条记录需要 UI 区分 | Reviewer | L2 | ✅ 已验证 | 衍生 Asset 加"转换自"标记 |
| 录音选择 macOS SFSpeechRecognizer | PM | L4 | ✅ 已验证 | 用户明确选择选项 2 |
| Word/PPT 通过 zip+XML 解析可行 | Proposer | L3 | ✅ 已验证 | OOXML 规范，先例成熟 |
| 历史 Asset 回填 | Proposer | L4 | ⏸️ 搁置 | P1，不影响新增流程 |
| 非 macOS 平台录音降级处理 | Reviewer | L4 | ⏸️ 搁置 | 当前仅 macOS 发布，P1 再议 |
| quality_level 阈值过滤防垃圾文件 | Reviewer | L3 | ✅ 已验证 | quality >= 1 才物化 |
| 音频 API 路径（路径B）不稳定 | Reviewer | L3 | ❌ 已推翻 | Ark API 音频支持不确定，选项2替代 |
| 本地 Whisper 打包体积不可接受（路径C） | Reviewer | L3 | ❌ 已推翻 | 几百 MB 不可接受 |

---

### 层间过渡验证（Layer 4 → PRD）

- [x] Layer 4 无"❓ 待定"核心定义
- [x] 所有"⏸️ 搁置"项已标注为 P1/out-of-scope
- [x] 本层共识可直接作为 PRD 功能需求来源
- [x] 论证追踪表已更新

---

**Debate 结论：正式进入 PRD 产出阶段。**
