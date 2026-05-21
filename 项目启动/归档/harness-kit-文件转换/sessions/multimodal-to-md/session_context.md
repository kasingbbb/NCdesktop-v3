# Session Context — 多模态文件转换为 Markdown

## 项目信息

- **项目名称**: NCdesktop 多模态文件转换引擎
- **Session 名称**: multimodal-to-md
- **复杂度等级**: L（复杂）
- **创建日期**: 2026-04-11

---

## 1. 项目背景

NCdesktop（NoteCapt Desktop）是一个基于 Tauri v2 + React 19 的 macOS 桌面应用，定位为多模态知识资产的非线性剪辑台。当前应用已实现知识库/项目/素材管理、时间轴、TF 卡同步、LLM 桥接导出等核心功能。

### 当前文件转换能力（基线）

| 能力 | 实现状态 | 说明 |
|------|----------|------|
| 数据库 → Markdown 组装 | ✅ 已实现 | `export_project_markdown` 命令，从 DB 拼装 |
| LLM 润色 Markdown | ✅ 已实现 | `llm_enhance_export` 命令 |
| 转录段 → txt/SRT/MD | ✅ 已实现 | 前端 `transcription-export.ts` |
| 文本文件读取 | ✅ 已实现 | `get_file_content`（std::fs::read_to_string） |
| PDF/图片预览 | ✅ 已实现 | `convertFileSrc` 在 WebView 中展示 |
| PDF → Markdown | ❌ 未实现 | — |
| 图片 OCR → Markdown | ❌ 未实现 | — |
| EPUB → Markdown | ❌ 未实现 | — |
| 音频转录 → Markdown | ❌ 未实现 | 有 symphonia 音频解码但无 ASR |
| 内容主题打标签 | ❌ 未实现 | — |

---

## 2. 核心需求

将以下多模态数据源转换为高质量的 Markdown 格式，使 AI（如 NotebookLM、ChatGPT）更容易吸收：

1. **照片** → 通过 OCR/视觉理解提取内容 → Markdown
2. **PDF** → 解析文本/表格/图片 → Markdown
3. **TXT** → 清洗格式化 → Markdown
4. **EPUB** → 解析章节结构 → Markdown
5. **录音** → 语音转文字 → Markdown

### 进阶需求

- 转换过程中自动提取主题并**打标签**
- 标签用于外部索引，提升 AI 检索质量

---

## 3. 技术栈约束

| 维度 | 约束 |
|------|------|
| 桌面框架 | Tauri v2（Rust 后端 + React 前端） |
| 后端语言 | Rust（stable） |
| 前端语言 | TypeScript（严格模式） |
| 数据库 | SQLite（rusqlite, bundled） |
| 目标平台 | macOS（Apple Silicon 优先，兼容 Intel） |
| 打包体积 | < 15MB（需考虑引入的库大小） |
| 冷启动 | < 500ms |
| 已有音频依赖 | symphonia（解码）、无 ASR 引擎 |
| LLM 能力 | 已有 reqwest + OpenAI 兼容 API 客户端 |
| 已有标签系统 | tags 表 + asset_tags 关联表 + AI 标签能力 |

---

## 4. 用户画像

- **主要用户**: 学生、研究者、知识工作者
- **核心场景**: 课堂录音+拍照 → 导入 NCdesktop → 转换为 Markdown → 喂给 AI 学习工具
- **技术水平**: 非技术用户，期望一键转换、零配置

---

## 5. 硬约束

1. 必须在本地完成核心转换（离线可用），LLM 增强可依赖网络
2. 不引入需要额外安装系统依赖的方案（用户零配置）
3. 转换结果必须是结构良好的 Markdown（带标题层级、列表、代码块等）
4. 必须与现有 NCdesktop 的素材管理系统集成（Asset 模型）
5. 转换过程需要有进度反馈
6. Apple Silicon 原生支持

---

## 6. Reviewer 应重点关注的风险域

- **打包体积膨胀**：引入 OCR/ASR/PDF 解析库可能大幅增加体积
- **性能影响**：大文件转换是否阻塞 UI
- **离线 vs 在线的边界**：哪些必须本地、哪些可以用 LLM API
- **转换质量**：不同格式的 Markdown 输出质量如何保证
- **macOS 原生能力**：是否可利用 macOS Vision/Speech 框架减少第三方依赖
