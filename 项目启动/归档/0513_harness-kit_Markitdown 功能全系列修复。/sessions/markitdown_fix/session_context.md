# Session Context — NCdesktop / MarkItDown 转录全系列修复 & DMG 自包含分发

> 本文件由 Host 在 Onboarding 阶段写入，作为 Debate / Conductor 全流程的唯一领域上下文来源。

---

## 1. 项目信息 [必填]

- **项目名称**：NCdesktop / NoteCapt（桌面端）— MarkItDown 转录链全系列修复 & DMG 自包含分发
- **一句话描述**：让 NoteCapt 对外分发的 DMG 内置 MarkItDown + Python + 所有依赖，确保 pdf / docx / pptx / xlsx / html / epub / image 全格式转录在生产环境真实可用，ASR 走讯飞在线方案。
- **项目类型**：Desktop App（Tauri + React + Rust + 嵌入 Python 运行时）
- **复杂度等级**：**L**（多格式真实样本验收 + 跨技术栈打包 + 签名/公证 + 生产事故修复，技术与安全敏感度均高）

---

## 2. 技术上下文 [必填]

- **主语言**：Rust（src-tauri）+ TypeScript（前端）+ Python（MarkItDown 子进程）+ Bash（打包脚本）
- **框架/运行时**：Tauri 2.x · React 18 · Vite · python-build-standalone 3.12.7 · MarkItDown 0.1.5（pdf/docx/pptx/xlsx + beautifulsoup4 + ebooklib）
- **数据库**：SQLite（asset / conversion_meta / tag / migration）
- **关键外部依赖**：
  - `markitdown[pdf,docx,pptx,xlsx]==0.1.5`
  - `beautifulsoup4`（html）· `ebooklib`（epub）
  - python-build-standalone（cpython 3.12.7，aarch64-apple-darwin / x86_64-apple-darwin）
  - 讯飞在线 ASR（audio）— **不走 MarkItDown**
  - codesign / xcrun notarytool（macOS 签名公证）
- **现有代码库**：改造现有代码
  - 提取器：`src-tauri/src/extraction/extractors/markitdown.rs`（含 90s 超时、版本缓存、image 空输出回退）
  - 运行时探测：`src-tauri/src/extraction/scheduler.rs:531-532`（`markitdown-venv/bin/python[3]`）
  - 打包脚本：`scripts/prepare-embedded-python.sh`、`scripts/prepare-embedded-markitdown-runtime.sh`、`scripts/build-macos-dmg.sh`
  - 配置：`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml`
- **目标部署环境**：macOS DMG 离线分发（arm64 优先，可选 x86_64）；用户机器**不需要预装 Python / pip / 任何依赖**。

---

## 3. 关键约束 [必填]

- **安全性要求**：中 — 本地文件转录，无网络上传（ASR 除外，讯飞经用户授权后调用）；签名/公证必须通过 Gatekeeper。
- **性能要求**：中 — 单文件 < 90s 子进程超时已存在；DMG 体积可接受 < 300 MB（嵌入 Python+deps 后）。
- **用户体验要求**：高 — **"双击安装即用"是绝对底线**；不得弹任何"请安装 Python"提示。
- **可维护性要求**：中 — 打包流程必须可在 CI 复现，依赖版本必须 pin。
- **不可妥协的底线**：
  1. **零外部依赖安装**：用户安装 DMG 后无需安装 Python / pip / brew / 任何系统包。
  2. **真实样本全格式通过**：pdf（文本 + 扫描）/ docx / pptx / xlsx / html / epub / image 必须用**真实生产样本**验收（不是单测 mock）。
  3. **生产失效场景必须复现并修复**：当前生产环境 pdf / epub / 录音转录失效问题必须先复现根因，再设计修复。
  4. **ASR 完全独立**：音频文件不走 MarkItDown，走 `audio_asr_iflytek.rs`，本次 PRD 不修改 ASR 业务逻辑。
  5. **签名与公证**：DMG 必须可在干净 macOS（无开发者权限）上首次打开通过 Gatekeeper（codesign --deep + notarytool staple）。

---

## 4. 质量偏好（影响 Reviewer 评分权重）

| 维度 | 权重 | 说明 |
|------|------|------|
| 功能正确性 | 35% | 真实样本端到端转录成功率是核心 KPI |
| 安全性 | 10% | 主要是签名/公证；无认证授权场景 |
| 代码质量 | 10% | 提取器代码已较成熟，重点在打包 |
| 测试覆盖 | 25% | 必须有"真实样本验收矩阵"+ DMG 冒烟测试 |
| 架构一致性 | 10% | 必须遵守现有 Extractor trait + scheduler 探测路径 |
| 可维护性 | 10% | 打包脚本必须幂等可复现 |

> 总和 100%。本次提高"功能正确性"和"测试覆盖"权重，因为这是一次生产事故修复 + 分发链路加固。

---

## 5. 领域特定代码规范

```
- Rust：所有外部进程调用必须有超时；stderr 必须采集并 log::warn!；mime 与扩展名同时检测。
- Bash：打包脚本必须 set -euo pipefail；所有路径变量必须双引号；幂等（可重复执行得到相同结果）。
- Python 嵌入：禁止使用 venv --copies（会破坏 python-build-standalone 的 @executable_path rpath）；直接 pip install 到 standalone Python 的 site-packages，然后用 symlink 做 venv-shim。
- 版本锁定：markitdown 版本、python-build-standalone 版本、extras 列表必须 pin 在打包脚本里并写入 runtime-manifest.json。
- 路径解析：运行时探测顺序 = bundle Resources/markitdown-venv → bundle Resources/python → PATH 中的 python3 → 失败。
```

---

## 6. 领域特定审查重点

```
- 嵌入 Python 的 rpath / dylib 路径在 .app/Contents/Resources 下是否能解析（不依赖任何 /usr/local 或开发机绝对路径）。
- symlink 在 codesign --deep 后是否仍有效；签名是否覆盖所有 .so / .dylib / 可执行 bin。
- runtime-manifest.json 是否与实际安装的 markitdown 版本、extras 一致（避免"清单说有 epub 实际没装 ebooklib"）。
- pdf 扫描件路径：markitdown 0.1.5 的 pdf extras 是否真的覆盖图片扫描 pdf（如不覆盖，本次 PRD 必须明确"扫描 pdf 走 pdf_scan / image_ocr 而非 markitdown"）。
- epub 转换是否真的依赖 ebooklib 且被打入 site-packages（生产失效高度怀疑是 extras 未声明 epub）。
- 音频文件路由：MarkItDown extractor 的 SUPPORTED_MIME_TYPES 不得包含 audio/*；调度器应把音频路由到 iflytek。
- DMG 体积是否在可接受区间；首次启动冷启动是否在可接受时间内。
- Gatekeeper：未签名 / ad-hoc 签名 / Developer ID 签名三种状态下的用户提示差异，是否在 README/UI 中明示。
```

---

## 7. 角色专业背景补充

- **Proposer 应具备的专业知识**：
  - python-build-standalone 的 rpath / 重定位机制
  - macOS codesign / notarization / Gatekeeper 流程
  - MarkItDown 0.1.5 各 extras 的实际依赖（pdfminer.six / python-docx / python-pptx / openpyxl / mammoth / beautifulsoup4 / ebooklib / Pillow）
  - Tauri 2.x bundle resources 机制
  - 真实样本回归测试矩阵设计
- **Reviewer 应重点关注的风险域**：
  - 扫描型 PDF（image-only pdf）被错误路由到 markitdown 后输出空 / 乱码
  - epub 因缺 ebooklib 静默失败
  - 嵌入 Python 在干净 macOS 上首次启动 dyld 错误
  - 签名后 symlink 失效导致 venv-shim 找不到 python
  - 老用户已安装版本的升级路径（旧 conversion_meta 是否要回填）
  - DMG 跨架构（arm64 / x86_64）的 fat / 单架构策略
  - 离线机器无 codesign 信任链时的兜底文案

---

## 8. 文件路径约定

- **PRD 路径**：`sessions/markitdown_fix/prd/`
- **源码路径**：仓库根 `NCdesktop/`（参考用，PRD 不直接落代码）
- **Session 记录路径**：`sessions/markitdown_fix/`
- **进度文件**：`sessions/markitdown_fix/conductor/progress.md`
- **架构方案存放**：`sessions/markitdown_fix/conductor/tasks/task_001_architect/output.md`
- **Debate 记录**：`sessions/markitdown_fix/debate/session_001/{debate_log.md, debate_conclusions.md}`

---

## 9. 辩题概述

- **核心辩题**：在不要求用户安装任何环境的前提下，如何让 NoteCapt DMG 的 MarkItDown 转录链对 pdf / docx / pptx / xlsx / html / epub / image 全格式在真实生产样本上稳定可用？ASR 已剥离至讯飞在线，本辩题不覆盖。
- **辩论偏好**：
  - 重点辩论层：**问题定义** + **差距分析** + **策略**（理想态可简化）
  - 最关心的维度：**功能正确性**（真实样本覆盖）+ **打包可分发性**（zero-setup DMG）

---

## 10. PM 注入的原始需求（原文）

> 现在内置了 markitdown 功能，但是生产环境上面很多真实转录进去的文档 pdf、epub、录音都失效了。所以我希望严格的确认 markitdown 的所有转录功能是完整能实现的（除了 ASR，ASR 我准备单独用讯飞的在线方案）。除了 markitdown 功能完善之外，我需要严谨的实现 markitdown 所有的功能和依赖可以被打包进 dmg，使得我对外发 dmg 就能使用 转录功能，不需要他自己安装 python、各种依赖。
