# Debate 共识结论 — NCdesktop MarkItDown 全系列修复

> 本文件汇总 4 层 Debate 的最终共识，供 PRD 与 Conductor 直接引用。

## Layer 1 共识：问题定义
- **三类失效根因正交**：分发链问题 / 路由问题 / 静默成功问题。
- **"失效"四元定义**：① 异常退出；② 退出 0 但 stdout 空（≤ 0 非空白字符）；③ 输出乱码（非 UTF-8 或可打印字符 < 50%）；④ 输出存在但缺结构（标题+段落 = 0）。
- **In**：pdf 文本型 / docx / pptx / xlsx / html / epub / image(EXIF+alt-text)。
- **Out**：扫描型 pdf / audio·video / 超大文件 >200MB / 非 macOS / **知识进化系统历史回填** / x86_64 fat 包。
- **KPI**：真实样本通过率 ≥95%（每格式 ≥5 个）、干净 macOS 12&14 arm64 VM 首启 100%、DMG ≤300MB、Gatekeeper notarytool staple 通过、扫描 pdf / audio 误路由率 = 0%。

## Layer 2 共识：理想态
- **DMG 目录**：`NoteCapt.app/Contents/Resources/{python/, markitdown-venv/(symlink shim), runtime-manifest.json, samples/}`。
- **8 错误码**：E_RUNTIME_MISSING / E_EXTRA_MISSING_EPUB / E_SCAN_PDF_UNSUPPORTED / E_AUDIO_WRONG_ROUTE / E_OUTPUT_EMPTY / E_OUTPUT_GIBBERISH / E_OUTPUT_NO_STRUCTURE / E_TIMEOUT_90S。
- **运行时探测**：bundle/Resources/markitdown-venv → bundle/Resources/python → 失败（不降级系统 python3）。
- **签名链**：逆序逐个签名（先内层 .so/.dylib → 中层 bin/python3.12 → 外层 .app），**禁用 `codesign --deep`**。
- **entitlements.plist 必含**：`com.apple.security.cs.allow-dyld-environment-variables`、`com.apple.security.cs.allow-unsigned-executable-memory`。

## Layer 3 共识：差距清单（8 gaps + 6 追加隐患）
| Gap | 说明 | MVP |
|-----|------|-----|
| G1 静默失败误判成功 | scheduler.rs:531-532 exit=0 + stdout='' 当成 OK | P0 |
| G2 epub extras 缺失 | `prepare-embedded-markitdown-runtime.sh` 未显式装 ebooklib | P0 |
| G3 扫描 pdf 错误路由 | markitdown.rs SUPPORTED_MIME_TYPES 不区分扫描/文本 | P0 |
| G4 音频路由防呆缺失 | 入口缺 assert | P0 |
| G5 runtime-manifest 不存在 | 无法运行时校验 extras | P0 |
| G6 真实样本矩阵未建 | 仓库无 fixtures | P0 |
| G7 签名+symlink 兼容性 | codesign --deep 不可靠 | P0 |
| G8 老用户升级回填 | 旧 conversion_meta 待标记 | P0 |
| R-① entitlements.plist 缺失 | Tauri 默认不含两项关键 entitlement | P0 |
| R-② 架构决断 | arm64-only（universal2 推 P2） | P0 |
| R-③ 保留 vs 修改矩阵 | 避免误伤 image 空回退/超时/版本缓存 | P0 |
| R-④ 升级感知 | legacy_unverified ≠ failed | P0 |
| R-⑤ 逆序签名脚本 | 替换 codesign --deep | P0 |
| R-⑥ 样本私有 + 加密 | git-lfs + AES-256 + CI secret | P0-pre |

## Layer 4 共识：MVP 策略
- **架构**：arm64-only DMG；universal2 / 双 DMG 推 P2（仅在 Intel 用户反馈出现时启动）。
- **任务粒度规则**：打包链至少 6 原子 task（T-A/B/C/D/E/F），扁平合并视为违规。
- **Scope 裁剪 4 原则**：① 生产复现优先；② 真实样本验收为唯一 KPI；③ 签名/公证不可妥协；④ **任何引入新分类器/启发式的 task 一律踢出 P0 进 P1**。
- **P0 task 总数**：1 sprint-0 + 16 P0 task（详见 PRD 分期计划）。
- **回滚 SOP 是 P0**：包含 hotfix 周期、N-1 DMG 镜像、manifest schema_version。
- **样本脱敏前置**：sprint 0 prerequisite，脱敏负责人 ≠ 打包负责人。

## 未达成共识的争议
**无**。Reviewer 提出的所有 L3/L2 挑战均被吸收并落地为具体 task；Proposer 主张未被任何挑战推翻。
