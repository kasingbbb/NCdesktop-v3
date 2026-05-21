# Session Context — 文件转换 v1.1

> 填写时间：2026-04-12

---

## 1. 项目信息

- **项目名称**：NCdesktop 文件转换功能 v1.1
- **一句话描述**：将拖入工作区的多模态文件（PDF、图片、Word、PPT、录音）自动转换为 Markdown 格式，物化为磁盘文件并以独立 Asset 出现在工作区
- **项目类型**：Desktop App（Tauri + React）
- **复杂度等级**：M（现有提取管线可复用，新增 3 个提取器 + 物化写出逻辑 + 前端展示适配）

---

## 2. 技术上下文

- **主语言**：Rust（后端）+ TypeScript/React（前端）
- **框架/运行时**：Tauri v2、React 18、Zustand、Vite
- **数据库**：SQLite（通过 rusqlite，Mutex 保护单连接）
- **关键外部依赖**：
  - macOS Vision Framework（OCR，已接入）
  - macOS SFSpeechRecognizer（ASR，本版本新接入）
  - zip crate + quick-xml crate（Word/PPT 解析，本版本新引入）
  - 现有 LLM Client（Ark API / OpenAI 兼容，可选，不用于本版本转换核心路径）
- **现有代码库**：改造现有代码
  - 提取管线：`NCdesktop/src-tauri/src/extraction/`
  - 拖放入库：`NCdesktop/src-tauri/src/commands/dropzone.rs`
  - macOS FFI 模式先例：`NCdesktop/src-tauri/src/macos/ocr_ffi.rs`
  - 工作区前端：`NCdesktop/src/components/features/AssetListView.tsx`
  - Asset 数据模型：`NCdesktop/src-tauri/src/models/asset.rs`
- **目标部署环境**：本地 macOS（Tauri 桌面应用）

---

## 3. 关键约束

- **安全性要求**：低 — 本地文件操作，无网络传输，无用户数据上传
- **性能要求**：中 — 提取为后台异步，不阻塞 UI；大文件（>50MB PDF、长录音）需有超时保护
- **用户体验要求**：高 — 拖入后工作区自动出现 MD 版，零干预；转换进度可感知（已有 ExtractionBadge）
- **可维护性要求**：中 — 新提取器遵循现有 `Extractor` trait 模式，不引入全局架构变更
- **不可妥协的底线**：
  1. 原始文件绝对不删除，MD 版是衍生物
  2. `quality_level = 0` 或 `structured_md` 为空字符串时不写出 .md 文件（防止垃圾文件）
  3. SFSpeechRecognizer 仅在 macOS 可用；非 macOS 构建时 audio extractor 的 `can_handle` 永返 false
  4. Word/PPT 提取本版本仅提取文字内容，跳过嵌入图片/图表

---

## 4. 质量偏好

| 维度 | 权重 | 说明 |
|------|------|------|
| 功能正确性 | 35% | 转换结果必须可读，格式基本正确 |
| 安全性 | 10% | 本地操作，无高风险 |
| 代码质量 | 20% | 新提取器须遵循 Extractor trait 模式 |
| 测试覆盖 | 15% | 新提取器须有 unit test（含空文件边界） |
| 架构一致性 | 15% | 不破坏现有 extraction pipeline 接口 |
| 可维护性 | 5% | 新格式未来可按同模式扩展 |

---

## 5. 代码规范

```
- Rust：错误处理用 ExtractionError 枚举，不用 unwrap/expect（除非 infallible）
- 新提取器文件名与格式对应：docx.rs / pptx.rs / audio_asr.rs
- structured_md 输出：文档类（Word/PPT/PDF）用标准 CommonMark；OCR/ASR 结果用纯段落
- 新增 Asset 时 source_type = "converted_from"，source_asset_id 填写原件 Asset ID
- 前端事件名：保持 "notecapt/" 前缀命名空间，新增 "notecapt/asset-converted"
- TypeScript：不新增 any 类型，衍生 Asset 标识字段用可选字段扩展 Asset 类型
```

---

## 6. 审查重点

```
- scheduler.rs 改动：写出 .md 失败不应中断整个 extraction 成功流程（降级：仅记录 warn）
- 新 Asset 插入：source_asset_id 外键不存在时需优雅降级（不崩溃）
- audio_asr.rs：长录音（>5min）需要分段处理或超时截断
- migration.rs：source_asset_id 字段须向后兼容（已有记录的 NULL 值合法）
- dropzone.rs：新增 MIME 映射须包含 .docx / .pptx 的完整 MIME 字符串
```

---

## 7. 角色专业背景补充

- **Proposer 应具备的专业知识**：Rust 异步运行时（tokio）、OOXML 文档结构（zip+XML）、macOS Speech Framework FFI、SQLite migration 模式
- **Reviewer 应重点关注的风险域**：文件写出失败的错误传播路径、DB migration 向后兼容性、跨线程 Asset 插入的锁竞争

---

## 8. 文件路径约定

- **PRD 路径**：`product/prd/`
- **源码路径**：`product/src/`（本项目实际源码在 `NCdesktop/`，此处存放架构设计文档）
- **Session 记录路径**：`sessions/`
- **进度文件**：`sessions/conductor/progress.md`
- **架构方案存放**：`sessions/conductor/tasks/task_001_architect/output.md`

---

## 9. 辩题概述

- **核心辩题**：如何将 NCdesktop 现有的"内容提取存 DB"管线改造为"提取后物化 .md 文件 + 创建衍生 Asset"，同时扩展多模态格式支持（Word/PPT/录音）
- **辩论偏好**：
  - 重点辩论层：差距分析 + 策略
  - 最关心的维度：体验（零干预物化）+ 架构一致性（不破坏现有管线）
