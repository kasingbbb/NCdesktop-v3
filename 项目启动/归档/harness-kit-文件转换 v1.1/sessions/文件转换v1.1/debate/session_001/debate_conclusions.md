# Debate 结论 — 文件转换 v1.1
# Session 001 | 日期：2026-04-12

---

## 四层共识摘要

### Layer 1：问题定义

**核心问题**：NCdesktop P0 的文件提取结果（`structured_md`）仅存在于 SQLite 数据库字段中，未物化为磁盘文件，未以独立 Asset 出现在工作区，导致用户必须通过 Inspector 面板才能读到内容，无法将转换结果直接"用"起来。

**范围边界**：
- **做**：PDF、照片、Word(.docx)、PPT(.pptx)、录音(mp3/m4a/wav) → 物化 .md 文件 + 创建衍生 Asset
- **不做（本版本）**：音频转写质量优化、MD 手动编辑、历史 Asset 批量回填、非 macOS 平台录音降级

**成功标准**：拖入上述任意格式文件后，用户无需任何额外操作，工作区中自动出现对应的 `.md` Asset 条目，点击可读。

---

### Layer 2：理想态

**用户体验目标**：
```
拖入文件 → 工作区：
  原件：课程表.pdf    ← 原始文件 Asset（保留）
  MD版：课程表.md     ← 衍生 Asset，标注"转换自 课程表.pdf"
```

**技术模型核心变化**：
- 现有：提取成功 → 写 DB 字段 → 结束
- 目标：提取成功 → 写 DB 字段（保留）→ **写出 .md 文件** → **创建衍生 Asset** → **发送前端事件** → 工作区自动刷新

**关键设计原则**：
- 非破坏性：原文件始终保留
- 透明性：衍生 Asset 的 `source_asset_id` 指向原件
- 懒生成：quality_level >= 1 且 structured_md 非空才物化

---

### Layer 3：差距分析

**必须解决的 Gap（MVP 底线）**：

| Gap | 涉及文件 |
|-----|---------|
| 提取成功后无物化写出步骤 | `extraction/scheduler.rs` |
| Asset 表缺少 `source_asset_id` 字段 | `db/migration.rs`, `models/asset.rs`, `db/asset.rs` |
| Word (.docx) 无提取器 | `extraction/extractors/docx.rs`（新建） |
| PPT (.pptx) 无提取器 | `extraction/extractors/pptx.rs`（新建） |
| 录音无 ASR 提取器 | `asr_ffi.swift`, `macos/asr_ffi.rs`, `extractors/audio_asr.rs`（新建） |
| dropzone.rs 缺少 docx/pptx/audio MIME 映射 | `commands/dropzone.rs` |
| 前端无衍生 Asset 区分展示 | `AssetListView.tsx` |
| 前端缺少 `asset-converted` 事件处理 | `stores/assetStore.ts` |

**已搁置风险（P1）**：
- 历史 Asset 的 structured_md 回填为 .md 文件
- 非 macOS 构建时录音文件的降级展示策略

---

### Layer 4：策略定稿

**录音 ASR 路径决策**：选项 2 —— macOS 原生 SFSpeechRecognizer（离线，复用 `macos/ocr_ffi.rs` FFI 先例）

**Task 执行顺序**：

```
并行批次 1（无依赖）：
  T01: DB migration（source_asset_id 字段）
  T03: docx 提取器
  T04: pptx 提取器（依赖 T03 引入的 crate）
  T05: audio ASR FFI bridge

串行批次 2（依赖批次 1）：
  T02: models/asset.rs 适配（依赖 T01）
  T06: dropzone.rs MIME 映射（依赖 T03/T04/T05 完成注册）

串行批次 3（依赖批次 2）：
  T07: scheduler.rs 物化写出 + 创建衍生 Asset（依赖 T01/T02）

串行批次 4（依赖批次 3）：
  T08: 前端事件监听 + 来源标记展示（依赖 T07）
```

---

## 关键决策记录

| 时间 | 决策 | 决策方 | 原因 |
|------|------|--------|------|
| 2026-04-12 | MVP 覆盖全格式（含 Word/PPT/录音） | PM 修正 | 用户期望开箱即用，不接受分期 |
| 2026-04-12 | 录音选 macOS SFSpeechRecognizer | PM 选择选项2 | 离线可用，有 FFI 先例，无 API 依赖 |
| 2026-04-12 | quality_level=0 不物化 | Host 技术底线 | 防止空内容垃圾文件写入工作区 |
| 2026-04-12 | 历史 Asset 回填搁置 P1 | Host/PM 共识 | 不影响新增流程，复杂度不合比例 |

---

## 移交 Conductor 的检查清单

- [x] 核心功能清单存在且每项有优先级
- [x] MVP 边界的"不做什么"非空且有原因
- [x] 高风险项已标注处理状态
- [x] 所有 Task 有明确依赖关系
- [x] PM 已确认可以开始编码
