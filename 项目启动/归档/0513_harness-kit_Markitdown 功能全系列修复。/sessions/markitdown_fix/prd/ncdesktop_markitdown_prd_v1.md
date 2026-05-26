# PRD — NCdesktop / NoteCapt MarkItDown 全系列修复 & DMG 自包含分发

| 字段 | 值 |
|------|----|
| PRD 版本 | v1.0 |
| 创建日期 | 2026-05-13 |
| 复杂度等级 | L |
| 产出流程 | Onboarding → Debate（4 层完整）→ PRD |
| 目标仓库 | `NCdesktop/项目启动/NCdesktop` |
| Session 路径 | `sessions/markitdown_fix/` |
| 上游共识 | `debate/session_001/debate_conclusions.md` |

---

## 1. 项目概述

NoteCapt 桌面端（Tauri + React + Rust）当前已嵌入 MarkItDown 转录链，但**生产环境上 pdf / epub / 录音文件转录大量静默失效**。根因被 Debate 拆解为三类正交问题：

1. **分发链失效** — DMG 未自包含 Python + 全 extras（特别是 `ebooklib`），用户机器一旦没装相应依赖即静默失败。
2. **路由错误** — 扫描型 pdf 被错误路由进 markitdown，markitdown 0.1.5 仅覆盖文本型 pdf，扫描件输出空字符串。
3. **静默成功** — 子进程退出码 0 + stdout 空被 scheduler 误判为成功，conversion_meta 写入"成功但空"记录。

本次 PRD 目标：**让 NoteCapt 对外分发的 DMG 内置 MarkItDown + Python + 全部依赖，确保 pdf(文本) / docx / pptx / xlsx / html / epub / image 全格式转录在真实生产样本上稳定可用；ASR 已剥离至讯飞在线方案，本 PRD 仅做"音频路由防呆"，不修改 ASR 业务逻辑。**

---

## 2. 用户定义与核心场景

### 用户画像
macOS 12+ 用户（arm64 优先），**无 Python / brew / Xcode CLT**，从官网下载 DMG。

### Happy Path（≤ 3 分钟内完成）
1. 下载 DMG（HTTPS + SHA256 公示） →
2. 双击挂载 → 拖入 Applications →
3. 首次启动通过 Gatekeeper（Developer ID notarized + stapled） →
4. 拖入任一支持格式（pdf文本 / docx / pptx / xlsx / html / epub / image）→
5. ≤ 90s 内呈现结构化 markdown。

### 拒绝路径（前置拦截，不进 markitdown）
- 扫描型 pdf → 显示 `E_SCAN_PDF_UNSUPPORTED` + 文案"扫描型 pdf 暂未支持，请等待后续 OCR 版本"
- 音频/视频 → 路由讯飞 ASR（本 PRD 不修改其业务逻辑）
- 超大文件 >200MB → 前置 reject
- x86_64 Mac 启动 → 明确拦截 UI（非 silent crash）

---

## 3. 功能需求

### 3.1 P0 必须实现（带优先级）

| ID | 功能 | 优先级 | 核心场景 | Debate 关键约束 |
|----|------|--------|---------|-----------------|
| F1 | DMG 自包含 Python 3.12.7 + MarkItDown 0.1.5 全 extras | P0 | 零外部依赖安装 | Layer 1 底线 #1 |
| F2 | runtime-manifest.json 写入并启动自检 | P0 | 显式失败替代静默失败 | Layer 2 共识 |
| F3 | 8 类错误码体系 + conversion_meta.failure_code 字段 | P0 | "失效"四元定义落地 | Layer 1/2 共识 |
| F4 | 扫描型 pdf 路由防呆（mime + 文件头嗅探，**不引入文本字数启发式**） | P0 | Out 边界明确拒绝 | Layer 3 G3 + 裁剪原则 #4 |
| F5 | 音频/视频路由防呆（markitdown.rs::extract 入口 assert + scheduler 单测） | P0 | ASR 完全独立 | Layer 3 G4 |
| F6 | entitlements.plist 显式声明 `cs.allow-dyld-environment-variables` + `cs.allow-unsigned-executable-memory` | P0 | Gatekeeper 通过前提 | Layer 2 R-① |
| F7 | 逆序逐个签名脚本（替换 `codesign --deep`） | P0 | 干净机首启 100% | Layer 2 R-⑤ |
| F8 | 7 格式 × ≥5 真实样本矩阵（加密 + CI secret 解密） | P0 | 真实样本 ≥95% KPI | Layer 1/3 R-⑥ |
| F9 | 干净 macOS 12 & 14 arm64 VM 冒烟（端到端 7 格式 + Gatekeeper） | P0 | 首启 100% KPI | Layer 1 KPI |
| F10 | 保留 vs 修改二维矩阵（保留 image 空回退 / 90s 超时 / 版本缓存；修改静默成功判定） | P0 | 不被重构误伤 | Layer 2 R-③ |
| F11 | 升级回填 migration：旧 `conversion_meta` 中 `status=success & content=''` 标 `legacy_unverified` | P0 | 老用户感知不退步 | Layer 3 R-④ |
| F12 | 回滚 SOP：hotfix 周期 + N-1 DMG 镜像 + manifest schema_version | P0 | 故障日有可执行预案 | Layer 4 |

### 3.2 P1（MVP 上线后 2 周内）
- 扫描型 pdf 通过 `pdf_scan / image_ocr` 兜底链实现可用（独立 extractor，非降级到 markitdown）
- 真实样本矩阵从 ≥5/格式扩到 ≥10/格式
- CI 公证 staple 自动化与发布流水线

### 3.3 P2（视用户反馈）
- universal2 fat 或双 DMG 支持 Intel Mac
- runtime-manifest 在线热更新机制
- 大文件（>90s 超时）切片处理
- 知识进化系统历史 conversion_meta 全量回填

---

## 4. 非功能需求

### 4.1 性能
- 单文件转录子进程 90s 超时（已实现，保留）
- DMG ≤ 300 MB（arm64-only 决断下成立）
- 首次启动冷启动 P95 < 2s（嵌入 Python lazy 加载）

### 4.2 安全
- 本地文件转录，无网络上传（ASR 除外）
- DMG 必须 Developer ID notarized + stapled，干净 macOS 离线开机可通过 Gatekeeper
- entitlements.plist 仅声明必要项，不开放任意权限

### 4.3 可用性
- "双击安装即用"是绝对底线，禁止任何"请安装 Python"提示
- 8 类错误码必须显示在前端，并提供"一键复制诊断日志"按钮

### 4.4 可维护性
- 打包脚本必须 `set -euo pipefail` 且幂等
- 所有版本号 pin 在 `prepare-embedded-*.sh` + `runtime-manifest.json` 双重源
- 真实样本不进公共仓库（私有 git-lfs + AES-256 加密 + CI secret 解密）

---

## 5. 技术约束（源自 session_context.md）

- **主语言**：Rust（src-tauri）+ TypeScript（前端）+ Python（MarkItDown）+ Bash（打包）
- **关键版本**：cpython 3.12.7（python-build-standalone 20241016）· markitdown 0.1.5 with `[pdf,docx,pptx,xlsx]` extras + `beautifulsoup4` + `ebooklib`
- **运行时探测顺序**：bundle Resources/markitdown-venv → bundle Resources/python → 失败（**不降级系统 python3**）
- **嵌入禁用项**：`venv --copies`（破坏 standalone Python rpath）；`codesign --deep`（Apple TN3127 已知缺陷）
- **代码规范**：所有子进程必须超时 + 采集 stderr + log::warn!；mime 与扩展名同时检测
- **数据库**：SQLite 现有 schema，新增 `conversion_meta.failure_code` 字段（migration）

---

## 6. 分期计划

### Sprint 0（前置，必须先完成）
- **T0**：真实样本脱敏 SOP + 加密 zip + git-lfs 私有仓 + CI secret。脱敏负责人 ≠ 打包负责人。

### Sprint 1（P0 打包链 — 6 原子 task，强制依赖图）
- **T-A**：`prepare-embedded-python.sh` 改造 + rpath 自检脚本（`verify-rpath.sh` 可独立运行）
- **T-B**：`prepare-embedded-markitdown-runtime.sh` extras pin（含 `ebooklib`、`beautifulsoup4`）+ 生成 `runtime-manifest.json`
- **T-C**：venv-shim symlink 改造 + 干净 macOS VM 冷启 import 全部 extras 验证
- **T-D**：`entitlements.plist` 编写 + 逆序逐个签名脚本（替换 `codesign --deep`）
- **T-E**：notarytool 提交 + stapler staple + Gatekeeper 干净机首开验证
- **T-F**：`build-macos-dmg.sh` 整合 + DMG 体积报告（CI 门禁 ≤300MB）

依赖图（必须显式）：**T0 → T-A → T-B → T-C → T-D → T-E → T-F**

### Sprint 2（P0 提取器与路由）
- **T1**：`runtime-manifest.json` 启动自检 + 失败码 `E_RUNTIME_MISSING` / `E_EXTRA_MISSING_EPUB`
- **T5**：8 错误码落到 `markitdown.rs` 与 `conversion_meta`（含 schema migration）
- **T6**：扫描 pdf 路由防呆（mime + 文件头嗅探，**禁止文本字数启发式**）
- **T7**：音频/视频路由防呆 + scheduler 单测
- **T10**：保留 vs 修改二维矩阵实现

### Sprint 3（P0 验收与升级）
- **T8**：7 格式 × ≥5 真实样本矩阵 + CI secret 解密 + 端到端断言
- **T9**：干净 macOS 12/14 arm64 VM 冒烟（AppleScript 自动化）
- **T11**：升级回填 migration（`legacy_unverified`）
- **T12**：回滚 SOP 文档 + N-1 DMG 镜像归档机制 + manifest schema_version

### MVP 验收门禁（所有 P0 完成后）
- [ ] 35 样本（含历史 epub 失效样本）端到端通过率 ≥ 95%
- [ ] 干净 macOS 12 & 14 arm64 VM 3 次冷启 100% 成功
- [ ] DMG ≤ 300 MB（CI 自动 du 校验）
- [ ] `spctl -a -vv` 输出 `accepted: Notarized Developer ID`
- [ ] 扫描 pdf / audio 误路由率 = 0%（单测 + 集成）
- [ ] runtime-manifest 启动自检覆盖 7 个关键 import（`ebooklib, bs4, pdfminer, pptx, docx, openpyxl, PIL`）

---

## 7. 不可妥协的底线（再次强调）

1. **零外部依赖安装**：用户安装 DMG 后无需安装 Python / pip / brew / 任何系统包。
2. **真实样本全格式通过**：必须用真实生产样本（脱敏后）验收，禁止仅 mock 单测。
3. **生产失效根因必须先复现再修复**：epub 失效根因必须执行 `python3 -c "import ebooklib"` 在嵌入 venv 中确认；扫描 pdf 必须用真实扫描件验证当前行为。
4. **ASR 完全独立**：音频不进 markitdown；本 PRD 不修改 `audio_asr_iflytek.rs` 业务逻辑。
5. **签名与公证**：DMG 必须可在干净 macOS（无开发者权限）首启通过 Gatekeeper。

---

## Conductor 桥接摘要

### 核心功能清单（带优先级）

| 功能 | 优先级 | 核心用户场景 | 来自 Debate 的关键约束 |
|------|--------|-------------|----------------------|
| DMG 自包含 Python+MarkItDown 全 extras | P0 | 用户零依赖安装即用 | L1 底线 #1；arm64-only 决断 |
| runtime-manifest.json + 启动自检 | P0 | 显式失败替代静默失败 | L2 共识 |
| 8 错误码 + conversion_meta.failure_code | P0 | "失效"四元定义落地 | L1 四元失效定义 |
| 扫描 pdf 路由防呆（mime+文件头，无启发式） | P0 | Out 边界明确拒绝 | L4 裁剪原则 #4 |
| 音频路由防呆 | P0 | ASR 完全独立 | L1 底线 #4 |
| entitlements.plist + 逆序签名脚本 | P0 | 干净机首启 100% + Gatekeeper | L2 R-①/R-⑤；Apple TN3127 |
| 7 格式 × ≥5 真实样本矩阵（加密私有） | P0 | 真实样本 ≥95% KPI | L3 R-⑥ |
| 干净 macOS VM 冒烟 | P0 | 首启 100% KPI | L1 KPI |
| 保留 vs 修改二维矩阵 | P0 | 不误伤 image 空回退/超时/版本缓存 | L2 R-③ |
| 升级回填 legacy_unverified | P0 | 老用户感知不退步 | L3 R-④ |
| 回滚 SOP + N-1 镜像 + manifest schema_version | P0 | 故障日有可执行预案 | L4 |
| 扫描 pdf OCR 兜底 | P1 | 扫描件最终可用 | L4 推后 |
| universal2 / 双 DMG | P2 | Intel Mac 支持 | L4 推后 |

### 不可妥协的技术底线

1. **运行时探测严格三级**：bundle/Resources/markitdown-venv → bundle/Resources/python → 失败（**不降级系统 python3**，违反此项即破坏"零依赖"底线）。
2. **打包链至少 6 个原子 task**（T-A/B/C/D/E/F）；任何把它们合并为单一巨型 task 的方案视为违规。Architect 必须按此粒度产出 task 清单。
3. **禁用 `codesign --deep`**，必须实现逆序逐个签名脚本（Apple TN3127）。
4. **禁用 `venv --copies`**，必须用 symlink venv-shim（python-build-standalone @executable_path rpath 兼容）。
5. **真实样本不得进入公共仓库**，必须用私有 git-lfs + AES-256 加密 + CI secret 解密。
6. **任何引入新分类器/启发式的 task 一律踢出 P0 进 P1**（防止扫描 pdf 探测偷塞 P0 导致 MVP 滑期）。
7. **MarkItDown SUPPORTED_MIME_TYPES 不得包含 audio/* video/***，且 `extract()` 入口必须 assert。

### 已识别的高风险项

| 风险 | 来源（Debate 哪一轮） | 当前状态 | 缓解策略 |
|------|---------------------|----------|----------|
| `codesign --deep` 漏签 .so 导致 dyld 失败 | R2 (L2) | 已解决（规范化） | T-D 逆序签名脚本 + `codesign -vvv --strict` 验证 |
| epub 因缺 ebooklib 静默失败（生产已发生）| R1 (L1) | 已解决（规范化） | T-B extras pin + T1 启动自检 `import ebooklib` |
| 扫描 pdf 走 markitdown 输出空被判成功 | R1 (L1) | 已解决（规范化） | T5 四元失效校验 + T6 路由前拦截 |
| 老用户升级后旧记录被误判 failed 触发批量重跑 | R3 (L3) | 已解决（规范化） | T11 `legacy_unverified` migration + UI 区分文案 |
| 真实样本含版权 commit 即法律风险 | R2 (L2) | 已解决（规范化） | T0 私有 git-lfs + AES-256 + CI secret |
| pbs 在 macOS 13.x 子版本 dyld 崩溃 | L4 脆弱性 | 已搁置（监控）| T12 回滚 SOP + N-1 DMG 镜像 |
| Intel Mac 用户实际占比 > 10% | L4 脆弱性 | 已搁置（监控）| P2 提前启动双 DMG |
| markitdown 0.1.5 自身 epub 转换 bug（非 extras 问题） | L3 脆弱性 | 已搁置（待 T8 真实样本验证）| 若验证发现：升级到 markitdown 新版本 / 自实现 ebooklib→md |

### MVP 边界声明

**做什么**：
- 嵌入 Python 3.12.7 + markitdown 0.1.5 + extras（pdf, docx, pptx, xlsx, html, epub, image-metadata）到 DMG，arm64-only
- 8 错误码体系 + conversion_meta.failure_code
- 扫描 pdf / audio / 超大文件路由防呆（不进 markitdown）
- entitlements + 逆序签名 + notarize + staple
- 7 格式 × ≥5 真实样本矩阵 + 干净 macOS VM 冒烟
- 保留 vs 修改矩阵；升级 legacy_unverified migration；回滚 SOP

**不做什么（显式 Out，含原因）**：
- 扫描 pdf 真正可用 OCR：依赖独立 extractor，需 Tesseract/poppler 嵌入，体积+时间不可控，**推 P1**
- universal2 / Intel Mac 支持：体积 380-420MB 突破 KPI；**推 P2**
- 知识进化系统历史 conversion_meta 全量回填：属于跨系统迁移，**显式 Out**
- 大文件 >200MB 切片：超出 90s 超时上限的工程问题，**推 P2**
- 修改 `audio_asr_iflytek.rs` 业务逻辑：本 PRD 仅做路由防呆，**显式 Out**
- 公共仓库样本管理：版权风险，**显式 Out（用私有 git-lfs）**
- CI macOS runner 缺位下的兜底：若 CI 无 macOS runner，35 样本矩阵只能本地跑，CI 仅 lint/unit，**记录但不在本 PRD 范围内修复**
- x86_64 用户体验文案：MVP 仅 arm64，必须有明确拦截 UI（非 silent crash），但拦截文案设计**显式 Out（P2 随双 DMG 一起做）**——MVP 期采用最小可用文案

### Debate 中未达成共识的争议
**无**。所有 Reviewer 挑战均被 Proposer 吸收或被 Host 裁决并入策略。Architect 在设计时无需额外消除争议。
