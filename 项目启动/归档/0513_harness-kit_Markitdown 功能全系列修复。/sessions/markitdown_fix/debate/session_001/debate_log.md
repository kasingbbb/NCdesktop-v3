# Debate Log — NCdesktop MarkItDown 全系列修复 & DMG 自包含

> 主持人：Host（本会话主 Agent）
> 提案方：Proposer（subagent）
> 批评方：Reviewer（subagent）
> 上下文：`sessions/markitdown_fix/session_context.md`
> 复杂度：L（4 层完整 Debate）

---

## Round 1 — Layer 1：问题定义

### Proposer 主张
- 三类失效根因正交：**分发链 / 路由 / 静默成功**
- 边界 In：pdf文本 / docx / pptx / xlsx / html / epub / image(EXIF + alt-text)
- 边界 Out：扫描型 pdf（错误码 `E_SCAN_PDF_UNSUPPORTED`）、audio/video（路由讯飞）、超大文件 >200MB
- KPI：真实样本通过率 ≥95%、首启 100%、DMG ≤300MB、Gatekeeper 通过、误路由 0%
- 选定方案：MarkItDown 覆盖 7 格式，扫描 pdf / audio 显式排除

### Reviewer 挑战
- **L3**：把 pdf/epub/录音并列为"全系列失效"是复合伪命题——三者根因正交，应分别归属
- **L3**：扫描 pdf 输出空是 markitdown 0.1.5 by-design，不是 bug，应被重述为"调度器路由策略缺失"
- **L2**：成功标准缺"可度量的样本与门槛"——样本从哪来、量、通过率门槛、DMG zero-setup 的成功矩阵均未定义
- **追加要求**："失效"必须有四元定义（异常退出 / 空 stdout / 乱码 / 缺结构），并落到代码

### Host 判断
- Reviewer 的 L3 直接命中 PM 原文与 session_context 间隐含矛盾，必须吸收
- Proposer 边界与 KPI 框架合理但缺失"失效定义"与"样本来源"
- 推进条件已满足：边界明确 + 四元定义已被吸纳 + KPI 矩阵补全

---

## Round 2 — Layer 2（理想态）+ Layer 3（差距分析）

### Proposer 主张
- **DMG 内部目录结构**：python/ + markitdown-venv/(symlink shim) + runtime-manifest.json + samples/
- **8 类错误码**：E_RUNTIME_MISSING / E_EXTRA_MISSING_EPUB / E_SCAN_PDF_UNSUPPORTED / E_AUDIO_WRONG_ROUTE / E_OUTPUT_EMPTY / E_OUTPUT_GIBBERISH / E_OUTPUT_NO_STRUCTURE / E_TIMEOUT_90S
- **运行时探测**：bundle/Resources/markitdown-venv → bundle/Resources/python → 失败（不降级系统 python3）
- **8 个 gap**（G1-G8）：静默失败 / epub extras / 扫描 pdf 路由 / 音频防呆 / runtime-manifest / 真实样本矩阵 / 签名+symlink / 升级回填
- **epub 根因复现命令**：`python3 -c "import markitdown, ebooklib; ..."`
- **音频路由防呆位置**：`markitdown.rs::extract()` 入口 assert mime 不以 audio/ 开头

### Reviewer 挑战
- **L3-①（必并入）**：python-build-standalone 在 macOS 13+ quarantine + hardened runtime 下要求 entitlements.plist 显式声明 `com.apple.security.cs.allow-dyld-environment-variables` 与 `allow-unsigned-executable-memory`，Tauri 默认 entitlements 不含
- **L3-②（必并入）**：DMG ≤300MB 与 universal2 fat 不可兼得——必须先决断单架构 or fat
- **L2-③（必并入）**：必须有"保留 vs 修改"二维矩阵，避免误伤已实现的 image 空输出回退、90s 超时、版本缓存
- **L2-④（必并入）**：升级回填是隐藏 P0——旧 `conversion_meta` 中 `status=success, content=''` 的记录需标 `legacy_unverified`，不能直接重判为 failed
- **L2-⑤（必并入）**：`codesign --deep` 对 standalone Python 有已知缺陷（Apple TN3127），必须改为"逆序逐个签名"
- **L2-⑥（必并入）**：真实样本含版权，必须用私有 git-lfs + AES-256 加密 zip + CI secret，且脱敏 SOP 必须落到打包脚本同级目录

### Host 判断
- Reviewer 6 条全部为有效约束，全部并入 L4 策略
- Proposer 8 gaps 框架保留，但 G7 需展开为"逆序签名 + entitlements"两个子项

---

## Round 3 — Layer 4：策略 / MVP / 分期

### Proposer 主张
- **架构决断**：MVP arm64-only 单架构；universal2 推 P2
- **MVP（P0）**：11 个 task（T1-T11），总 ~15 人日
- **P1**：扫描 pdf OCR 兜底、样本矩阵扩到 7×5、CI 发布流水线
- **P2**：universal2 / 双 DMG、manifest 热更新、大文件切片
- **Scope 裁剪三原则**：生产复现优先、真实样本验收唯一 KPI、签名/公证不可妥协
- **Top 5 风险登记表**：codesign 漏签 / epub 缺 ebooklib / 扫描 pdf 误判 / 升级误重跑 / 样本版权

### Reviewer 挑战
- **L3 主**：T2 把 "pbs 解压 + 直装 site-packages + symlink venv-shim" 合并是"假原子"——必须拆为 T-A（rpath）+ T-B（extras pin）+ T-C（symlink + 冷启验证）+ T-D（entitlements + 逆序签名）+ T-E（notarize + staple）+ T-F（DMG 整合 + 体积）共 6 个原子 task；依赖图必须显式
- **L2 (a)**：回溯校验必须明文加"任何引入新分类器/启发式的 task 一律踢出 P0 进 P1"——否则扫描 pdf 探测会被偷塞 P0
- **L2 (b)**：必须新增 `task_rollback_sop`（P0）：hotfix 周期、N-1 DMG 镜像、manifest schema_version
- **L2 (c)**：Out 必须显式列入：知识进化系统回填 / CI macOS runner 兜底 / x86_64 用户拦截文案
- **L2 (d)**：35 样本脱敏 SOP 前移到 **sprint 0 prerequisite**，且脱敏负责人 ≠ 打包负责人

### Host 判断
- 全部接受。Proposer T2 被强制拆为 T-A/B/C/D/E/F；新增 T12（回滚 SOP）；新增 sprint 0 task（样本脱敏 SOP）；新增 Out 三条
- 推进条件满足：MVP 任务清单原子化、依赖图清晰、回滚路径补全、scope 裁剪原则可执行

---

## 论证追踪表（最终版）

| 论点 | 提出方 | 层级 | 状态 | 备注 |
|------|--------|------|------|------|
| 三类失效根因正交 | Reviewer | L1 | ✅ 已验证 | Proposer 接受并写入边界声明 |
| "失效"四元定义 | Reviewer | L1 | ✅ 已验证 | 8 错误码已映射到四元 |
| 扫描 pdf 显式 Out | Proposer | L1 | ✅ 已验证 | E_SCAN_PDF_UNSUPPORTED 错误码立项 |
| 音频完全剥离 ASR | PM/session_context | L1 | ✅ 已验证 | G4 + T7 双重防呆 |
| 真实样本 ≥95% 通过率 / 35 样本 | Proposer | L1 | ✅ 已验证 | T8 + sprint 0 脱敏 SOP |
| DMG ≤300MB | Proposer | L1 | ✅ 已验证（条件性）| 仅在 arm64-only 决断下成立 |
| arm64-only MVP / universal2 推 P2 | Proposer | L4 | ✅ 已验证 | Reviewer 未反对，加 x86_64 拦截文案 Out 项 |
| 8 错误码体系 | Proposer | L2 | ✅ 已验证 | 落入 T5 |
| entitlements.plist 显式 | Reviewer | L2 | ✅ 已验证 | 落入 T-D |
| codesign 逆序逐个签名 | Reviewer | L2 | ✅ 已验证 | 替换 --deep，落入 T-D |
| 升级 legacy_unverified | Reviewer | L3 | ✅ 已验证 | 落入 T11 |
| 保留 vs 修改二维矩阵 | Reviewer | L3 | ✅ 已验证 | 落入 T10 |
| 真实样本私有 git-lfs + AES-256 | Reviewer | L3 | ✅ 已验证 | 落入 T0（sprint 0） |
| 打包链 6 原子 task | Reviewer | L4 | ✅ 已验证 | T2 拆为 T-A/B/C/D/E/F |
| 回滚 SOP 必须 P0 | Reviewer | L4 | ✅ 已验证 | 新增 T12 |
| 扫描 pdf 探测启发式 | Proposer T6 | L4 | ⏸️ 搁置 | 改为"mime 白名单 + 文件头嗅探"，不引入 pdftotext 抽 100 字启发式（否则进 P1）|
| 知识进化系统回填 | Reviewer | L4 | ⏸️ 搁置 | 显式 Out |
| 大文件 >200MB 切片 | Proposer | L1 | ⏸️ 搁置 | P2 |
| universal2 / 双 DMG | Proposer | L4 | ⏸️ 搁置 | P2 |

无 ❓ 待定项。无 ❌ 已推翻项。

---

## 回溯校验表（L1 ↔ L4 MVP）

| MVP 功能项 | 对应 Layer 1 核心问题 / 底线 | 优先级 |
|-----------|---------------------------|--------|
| T0 sprint 0 真实样本脱敏 SOP | 真实样本 ≥95% KPI 不可空头 | P0-pre |
| T-A pbs 解压 + rpath 自检 | 零外部依赖安装底线 | P0 |
| T-B pip install + extras pin + manifest | 分发链失效 → epub 必装 ebooklib | P0 |
| T-C venv-shim symlink + 冷启验证 | 首启 100% 可用率 | P0 |
| T-D entitlements + 逆序签名 | Gatekeeper 通过 | P0 |
| T-E notarize + staple + 干净机首开 | Gatekeeper notarized 通过 | P0 |
| T-F DMG 整合 + 体积报告 | DMG ≤300MB | P0 |
| T1 runtime-manifest.json 写入 + 启动自检 | 静默失败 → 显式失败 | P0 |
| T5 8 错误码 + conversion_meta.failure_code | "失效"四元定义落地 | P0 |
| T6 扫描 pdf 路由（mime+文件头嗅探，不引启发式）| Out: 扫描 pdf 必须明确拒绝 | P0 |
| T7 音频路由防呆（markitdown.rs assert + scheduler 单测）| ASR 完全独立 | P0 |
| T8 35 样本矩阵 + CI secret 解密 | 真实样本 ≥95% 验收 | P0 |
| T9 干净 VM 冒烟（macOS 12/14 arm64）| 首启 100% + Gatekeeper | P0 |
| T10 保留 vs 修改二维矩阵 | 不误伤已实现的 image/超时/版本缓存 | P0 |
| T11 升级回填 legacy_unverified migration | 老用户感知不退步 | P0 |
| T12 回滚 SOP（hotfix 周期 + N-1 + manifest schema_version）| 故障日有可执行预案 | P0 |

**覆盖性自检**：L1 In 列表 7 个格式由 T-B + T1 + T5 + T8 共同覆盖；L1 Out 由 T6 + T7 显式拦截；4 条底线分别落 T-A/T-C/T-D/T-E（零依赖+Gatekeeper）、T5/T1/T10（真实样本验收）、T7/T6（ASR 独立）、T-D/T-E（签名公证）。无 L1 核心问题被裁剪；无新增功能偷塞。
