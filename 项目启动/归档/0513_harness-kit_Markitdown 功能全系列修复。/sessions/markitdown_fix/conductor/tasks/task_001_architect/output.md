# 技术方案 — NCdesktop MarkItDown 全系列修复 & DMG 自包含分发

> Architect 产出，承接 PRD v1.0（`sessions/markitdown_fix/prd/ncdesktop_markitdown_prd_v1.md`）与 Debate 共识。
> 状态机：PRD_READY → ARCHITECTURE（本文件）→ DEVELOPING（按 task 清单分发）。

---

## 0. 需求复述 / 约束识别 / 风险扫描（思考协议）

### 0.1 需求复述（自有语言）
对外分发的 NoteCapt DMG 必须在 macOS 12+ arm64 干净机器上做到："双击装入 Applications → 通过 Gatekeeper → 拖入 pdf(文本) / docx / pptx / xlsx / html / epub / image 即可在 90s 内输出 markdown"。当前生产环境 epub / 部分 pdf / 录音转录失效的三类正交根因（**分发链失效 / 路由错误 / 静默成功**）必须同时被消除。ASR 已剥离讯飞在线，本期仅做"音频不进 markitdown"路由防呆。

### 0.2 硬约束清单（不得在设计中违反）
H1. 运行时探测严格三级：`Resources/markitdown-venv → Resources/python → 失败`，**禁止降级 `PATH` 的 `python3`**。
H2. 打包链至少 6 原子 task（T-A..T-F），不得合并。
H3. 禁用 `codesign --deep`（Apple TN3127）。
H4. 禁用 `venv --copies`（破坏 standalone Python `@executable_path` rpath）；用 symlink venv-shim。
H5. MarkItDown `SUPPORTED_MIME_TYPES` 不得含 `audio/*` / `video/*`；`extract()` 入口必须 `assert`。
H6. 任何新分类器/启发式（含"扫描 pdf 文本字数阈值"）一律 P1 起步。
H7. 真实样本：私有 git-lfs + AES-256 + CI secret 解密；脱敏负责人 ≠ 打包负责人。
H8. 失败必须显式：8 错误码 + `conversion_meta.failure_code`，禁止"exit 0 + stdout=''"判成功。

### 0.3 高风险项与本设计的应对
| 风险 (来自 PRD 桥接) | 设计应对 |
|---|---|
| `codesign --deep` 漏签 .so 导致 dyld 失败 | T-D 实现 `sign-bundle.sh`：递归收集 → 按"深度倒序"排序 → 逐个 `codesign --options runtime --timestamp` → 最后签 `.app` 外壳；末尾 `codesign --verify --deep --strict --verbose=4` 校验 |
| epub 缺 ebooklib 静默失败 | T-B `requirements.lock` 显式 pin `ebooklib`、`beautifulsoup4`，并在 `runtime-manifest.json.imports` 列出 7 个关键模块；T1 启动自检逐个 `python -c "import X"` |
| 扫描 pdf 走 markitdown 输出空被判成功 | T-D（运行期）：`scheduler` 在 markitdown 调度前对 `application/pdf` 做 **mime + 文件头**（`%PDF-` 后扫描 first-page 是否仅含 image XObject）路由判定；T-T8 单测覆盖；**不引入文本字数阈值** |
| 老用户旧 `success & content=''` 被误判 failed | T11 migration：`legacy_unverified`（与 `failed` 区分），UI 文案分两类 |
| 真实样本含版权 | T0 SOP + AES-256 + 私有 git-lfs；CI 用 `MARKITDOWN_SAMPLES_KEY` secret |
| `pbs` 在 macOS 13.x 子版本 dyld 崩溃 | T-E notarize 前在两台 VM (12 / 14) 冷启验证；T15 回滚 SOP + N-1 镜像 |
| markitdown 0.1.5 自身 epub bug | T12 真实样本矩阵触发；若验证失败，决策"升 markitdown 新版本 / 自实现 ebooklib→md"（不在本期 scope 内自动执行） |

### 0.4 技术决策点（汇总，详见 ADR）
- DK1：嵌入 Python 来源（python-build-standalone vs 系统 vs Homebrew 重打包）
- DK2：venv 实现（`venv --copies` vs symlink shim vs `pip install --target`）
- DK3：签名策略（`codesign --deep` vs 逆序逐个）
- DK4：扫描 pdf 路由（文本字数启发式 vs mime+文件头嗅探）
- DK5：错误码落地（独立表 vs `conversion_meta.failure_code` 字段）
- DK6：架构支持（universal2 vs arm64-only）
- DK7：runtime-manifest 校验时机（启动一次 vs 每次调用）
- DK8：样本仓位置（公开 vs 私有 git-lfs vs S3）

---

## 1. 技术选型

| 维度 | 选型 | 见 ADR |
|---|---|---|
| 嵌入 Python | `python-build-standalone` cpython 3.12.7 aarch64-apple-darwin（20241016 build） | ADR-001 |
| MarkItDown | `markitdown[pdf,docx,pptx,xlsx]==0.1.5` + `beautifulsoup4` + `ebooklib` | ADR-002 |
| venv 形态 | symlink venv-shim：`markitdown-venv/bin/python -> ../../python/bin/python3.12` | ADR-003 |
| 签名链 | 逆序逐个 `codesign --options runtime --timestamp`（禁用 `--deep`） | ADR-004 |
| 公证 | `xcrun notarytool submit --wait` + `xcrun stapler staple` | ADR-005 |
| 扫描 pdf 路由 | mime + 文件头嗅探（无文本字数阈值） | ADR-006 |
| 错误码落地 | `conversion_meta.failure_code TEXT NULL` + 应用层枚举 | ADR-007 |
| 架构 | arm64-only MVP；universal2 推 P2 | ADR-008 |
| 样本仓 | 私有 git-lfs + AES-256 zip + GitHub Actions secret | ADR-009 |
| Runtime 自检 | 启动时一次性自检；失败抛 `E_RUNTIME_MISSING` / `E_EXTRA_MISSING_<X>`，前端阻塞所有转录入口 | ADR-010 |

---

## 2. Architecture Decision Records (ADR)

### ADR-001：嵌入 python-build-standalone 3.12.7（arm64）
- **状态**：已接受
- **上下文**：用户机器无 Python；需要可分发的二进制运行时。
- **决策**：使用 `astral-sh/python-build-standalone` 2024-10-16 release 的 `cpython-3.12.7+20241016-aarch64-apple-darwin-install_only.tar.gz`，解压到 `Resources/python/`。
- **被排除项**：系统 `python3`（用户可能无）；Homebrew Python（依赖 brew）；自编译 cpython（脆弱）。
- **后果**：DMG 体积 +~70MB；`@executable_path` rpath 必须保持不变，**因此禁止 `venv --copies`**。
- **补强（2026-05-13，task_001 Reviewer 共识）**：rpath verify 的合法路径前缀字面集合扩展为 5 项 —— `@executable_path/`、`@loader_path/`、`@rpath/`、`/usr/lib/`、`/System/Library/`。`@loader_path/` 与 `@rpath/` 是 PBS 产物 dylib 间相互引用的实测必需 Mach-O 相对引用形式，与"无开发机绝对路径泄露"原则一致。开发机绝对路径前缀（`/Users/`、`/opt/`、`/private/`、`/tmp/`、`/usr/local/`）仍判 VIOLATION。

### ADR-002：MarkItDown 0.1.5 + 显式 extras pin
- **状态**：已接受
- **决策**：`pip install "markitdown[pdf,docx,pptx,xlsx]==0.1.5" "beautifulsoup4==4.12.3" "ebooklib==0.18"` 到 standalone Python 的 `site-packages`。
- **被排除项**：自动 `pip install markitdown`（生产已证明 extras 默认不装齐 epub 依赖）。
- **后果**：版本必须同时 pin 在 `prepare-embedded-markitdown-runtime.sh` 与 `runtime-manifest.json` 两处，CI 校验一致。

### ADR-003：symlink venv-shim
- **状态**：已接受
- **决策**：`mkdir Resources/markitdown-venv/bin` 后建符号链：`python -> ../../python/bin/python3.12`、`python3 -> python`；脚本入口直接调用 `Resources/markitdown-venv/bin/python -m markitdown`。
- **被排除项**：`python -m venv --copies`（破坏 rpath，dyld 起不来）；`pip install --target`（PYTHONPATH 注入风险）。
- **后果**：签名脚本必须能签穿 symlink；`codesign --deep` 不行，因此与 ADR-004 强耦合。

### ADR-004：逆序逐个签名
- **状态**：已接受
- **决策**：脚本 `scripts/sign-bundle.sh`：
  ```
  find "$APP/Contents" \( -name "*.so" -o -name "*.dylib" -o -perm -u+x \) -type f
    | awk '{print length($0), $0}' | sort -rn | cut -d' ' -f2-
    | xargs -I{} codesign --force --options runtime --timestamp -s "$IDENTITY" -- "{}"
  codesign --force --options runtime --timestamp --entitlements entitlements.plist -s "$IDENTITY" "$APP"
  codesign --verify --deep --strict --verbose=4 "$APP"
  ```
- **被排除项**：`codesign --deep`（Apple TN3127 已知缺陷，对 standalone Python 的复杂 dylib 拓扑漏签）。
- **后果**：脚本必须幂等；CI 与本地行为一致；签名验证作为 T-D 验收 AC。

### ADR-005：notarytool + stapler（API-key 模式）
- **状态**：已接受
- **决策**：用 `xcrun notarytool submit "$DMG" --key-id $K --key $P8 --issuer $I --wait`，成功后 `xcrun stapler staple "$DMG"`；干净机 `spctl -a -vv -t open --context context:primary-signature "$DMG"` 必须输出 `accepted: Notarized Developer ID`。
- **被排除项**：`altool`（已退役）；纯本地 Developer ID 签名不公证（Gatekeeper 提示风险）。
- **后果**：T-E 需要 App Store Connect API key 三元组（CI secret）。

### ADR-006：扫描 pdf 路由用"mime + 文件头嗅探"，禁文本字数启发式
- **状态**：已接受
- **上下文**：H6 硬约束。
- **决策**：在 `scheduler.rs` markitdown 调度分支前，对 `mime == application/pdf` 调用 `is_scan_pdf(path) -> bool`：用 `lopdf` / `pdf` crate 解析 page 1，若 `Resources.XObject` 中**仅含 Image XObject 且无 Font 引用** → 视为扫描型，路由到 `E_SCAN_PDF_UNSUPPORTED`（MVP 阶段拒绝；P1 由 `pdf_scan` extractor 接管）。
- **被排除项**：文本字数 < 阈值（启发式，H6 禁用）；运行 markitdown 后看 stdout 长度（"已经污染了 conversion_meta"）。
- **后果**：依赖 `lopdf` 或类似 crate；T9 必须用真实扫描件 + 真实文本 pdf 各 ≥3 样本做交叉验证。

### ADR-007：`conversion_meta.failure_code` 单字段 + 应用层枚举
- **状态**：已接受
- **决策**：新增 `failure_code TEXT NULL`；应用层 `enum FailureCode { ERuntimeMissing, EExtraMissingEpub, EScanPdfUnsupported, EAudioWrongRoute, EOutputEmpty, EOutputGibberish, EOutputNoStructure, ETimeout90s }`；migration 老记录 `status=success & content='' → failure_code='legacy_unverified'`（与 8 错误码并列的"已知未验证"状态）。
- **被排除项**：独立 `failure` 表（过度设计）。
- **后果**：UI 必须 8+1 文案；下游知识进化系统不得把 `legacy_unverified` 当 `success` 喂入。

### ADR-008：arm64-only MVP，universal2 推 P2
- **状态**：已接受
- **决策**：仅出 arm64 DMG；x86_64 用户启动时显示"暂未支持 Intel Mac"拦截 UI（最小文案 MVP 期手写）。
- **被排除项**：universal2 fat（体积 380-420MB 突破 KPI）；双 DMG（CI 时间 +50%）。
- **后果**：监控 Intel 用户反馈占比；触发条件成立则启动 P2。

### ADR-009：私有 git-lfs + AES-256 zip
- **状态**：已接受
- **决策**：仓库 `samples-private/`（独立 repo）使用 git-lfs；每个样本先经脱敏脚本，再用 `openssl aes-256-cbc -pbkdf2 -salt -iter 100000` 加密；CI 用 `MARKITDOWN_SAMPLES_KEY` secret 解密到 ephemeral 工作目录。
- **被排除项**：公共仓（版权）；S3（增加跨服务凭证管理）。
- **后果**：T0 必须先于 T8 完成；CI 必须可访问 secret。

### ADR-010：runtime-manifest 启动一次性自检
- **补强（2026-05-13，task_002 E-2 裁决）**：imports 探针 `docx` 改为 `mammoth`。理由：markitdown[docx] 路径实际由 `mammoth` 实现 DOCX → Markdown，`python-docx` 并非 markitdown 真实依赖。self-check 的语义是"验证 markitdown 跑得起来"，不是"验证 SDK 可 import docx"。同步从 requirements.lock 删除 `python-docx`（节省 ~2.6M）。

- **状态**：已接受
- **决策**：应用启动时调用 `verify_runtime_manifest()`：读 `Resources/runtime-manifest.json` → 对 `imports` 数组逐个 `Resources/markitdown-venv/bin/python -c "import X"` → 任一失败抛错误码并禁用所有转录入口。每次 markitdown 调用不再重复探测，**只**用缓存结果。
- **被排除项**：每次 `extract()` 重新探测（性能损耗）；不探测（与"显式失败"原则冲突）。
- **后果**：升级 DMG 后首启必须重跑自检（manifest 写入 `schema_version` + `runtime_id`，与 app version 解耦）。

---

## 3. 系统架构

```
┌──────────────────────────────────────────────────────────────────────┐
│ NoteCapt.app/                                                        │
│ └─ Contents/                                                         │
│    ├─ MacOS/notecapt                       ← Tauri 主进程            │
│    ├─ Resources/                                                     │
│    │  ├─ python/  (python-build-standalone 3.12.7)                  │
│    │  ├─ markitdown-venv/bin/python (symlink → ../../python/bin/...) │
│    │  ├─ runtime-manifest.json  (versions + imports + schema_version) │
│    │  └─ ...                                                         │
│    └─ _CodeSignature/                                                │
└──────────────────────────────────────────────────────────────────────┘

Rust 调用拓扑:
  scheduler (route guard: pdf-scan / audio) → markitdown::extract
       └─ python_candidates() 严格三级（H1）
       └─ run_with_timeout(MARKITDOWN_TIMEOUT)
       └─ 失败分类 → FailureCode → conversion_meta.failure_code
```

### 模块责任
| 模块 | 文件 | 责任 |
|---|---|---|
| 打包链 | `scripts/prepare-embedded-python.sh`, `prepare-embedded-markitdown-runtime.sh`, `sign-bundle.sh`(new), `notarize.sh`(new), `build-macos-dmg.sh` | 产出可分发 DMG |
| 提取器 | `src-tauri/src/extraction/extractors/markitdown.rs` | 子进程调用 markitdown，入口 assert audio/video |
| 调度路由 | `src-tauri/src/extraction/scheduler.rs` | mime 嗅探 + 路由 + runtime 自检结果消费 |
| 错误码 | `src-tauri/src/extraction/models.rs`（new `FailureCode`） | 8+1 枚举与 DB ↔ UI 映射 |
| Migration | `src-tauri/src/db/migration.rs`, `db/conversion_meta.rs` | failure_code 字段 + legacy_unverified 回填 |
| Runtime 自检 | `src-tauri/src/extraction/runtime_check.rs`(new) | 启动一次性 import 校验 |
| 样本仓 | 独立 repo `samples-private`（不在主仓） | 真实样本加密存储 |

---

## 4. 数据模型变更

```sql
-- migration N（task_008）
ALTER TABLE conversion_meta ADD COLUMN failure_code TEXT NULL;
CREATE INDEX idx_conversion_meta_failure_code ON conversion_meta(failure_code);

-- migration N+1（task_014）回填
UPDATE conversion_meta
SET failure_code = 'legacy_unverified'
WHERE status = 'success'
  AND (content IS NULL OR content = '' OR length(trim(content)) = 0);
```

`runtime-manifest.json` schema：
```json
{
  "schema_version": 1,
  "runtime_id": "py3.12.7-pbs20241016-md0.1.5-extras-v1",
  "python": { "source": "python-build-standalone", "version": "3.12.7", "build": "20241016" },
  "markitdown": { "version": "0.1.5", "extras": ["pdf","docx","pptx","xlsx"] },
  "extras_extra": ["beautifulsoup4==4.12.3", "ebooklib==0.18"],
  "imports": ["ebooklib","bs4","pdfminer","pptx","mammoth","openpyxl","PIL"],
  "build_timestamp": "2026-05-13T...",
  "arch": "aarch64-apple-darwin"
}
```

---

## 5. Task 清单（17 个，写入 progress.md）

```
- [ ] task_000_sample_desensitization_sop          — Sprint 0：脱敏 SOP + 加密样本仓
- [ ] task_001_prepare_embedded_python_rpath_verify — T-A：嵌入 Python + rpath 自检
- [ ] task_002_markitdown_extras_pin_manifest      — T-B：extras pin + runtime-manifest 生成
- [ ] task_003_venv_shim_symlink_cold_boot         — T-C：venv-shim symlink + 冷启 import
- [ ] task_004_entitlements_reverse_sign           — T-D：entitlements + 逆序签名
- [ ] task_005_notarize_staple_gatekeeper          — T-E：公证 + staple + Gatekeeper 验证
- [ ] task_006_build_macos_dmg_integration         — T-F：DMG 整合 + 体积门禁
- [ ] task_007_runtime_manifest_self_check         — 启动自检 + E_RUNTIME_MISSING/E_EXTRA_MISSING
- [ ] task_008_error_codes_and_failure_code_migration — 8 错误码 + failure_code 字段
- [ ] task_009_scan_pdf_route_guard                — 扫描 pdf 路由防呆（mime + 文件头）
- [ ] task_010_audio_route_guard                   — 音频路由防呆 + extract 入口 assert
- [ ] task_011_preserve_vs_modify_matrix           — 保留 vs 修改二维矩阵
- [ ] task_012_real_sample_matrix_ci_secret        — 7 格式 × ≥5 真实样本矩阵
- [ ] task_013_clean_vm_smoke_test                 — 干净 macOS 12/14 VM 冒烟
- [ ] task_014_legacy_unverified_migration         — legacy_unverified 回填 migration
- [ ] task_015_rollback_sop_manifest_schema_version — 回滚 SOP + N-1 镜像
```

---

## 6. Task 依赖拓扑

```
task_000  ──┐                                  (Sprint 0：前置必须)
            │
            ▼
task_001 → task_002 → task_003 → task_004 → task_005 → task_006   (打包链强制串行)
                                                  │
                                                  ▼
                              task_008 ──→ task_007              (8 错误码先落 → 启动自检消费)
                                              │
                            task_009 ┐        │
                            task_010 ┼────────┴──→ task_011 → task_012 → task_013 → task_014 → task_015
                                     (路由防呆与保留矩阵)

可并行：
  - task_009 与 task_010 可并行（不同模块）
  - task_008 可在打包链中（T-A..T-D）期间并行（不依赖 DMG）
  - task_015 可在 task_012/013 通过后立即启动
```

---

## 7. 安全考量
- 入口签名 + entitlements 仅声明两项最小必要项（PRD F6）。
- 真实样本仓 AES-256 + 私有 lfs；CI secret 不出 runner（不进 log）。
- ASR 网络流量与 markitdown 流量物理隔离（不同子进程、不同凭证）。

---

## 8. 风险登记表

| 风险 | 概率 | 影响 | 缓解 |
|---|---|---|---|
| pbs 在 macOS 13.x 子版本 dyld 崩 | 中 | 高 | T-E 冷启 + N-1 镜像（task_015）|
| markitdown 0.1.5 epub 自身 bug | 中 | 高 | task_012 验证；如复现：升级 / 自实现（不在本期 scope）|
| CI 无 macOS runner | 高 | 中 | task_012 本地 + CI lint/unit 分流（PRD 已声明 Out）|
| Intel 用户占比 > 10% | 低 | 中 | P2 启动双 DMG（监控驱动）|
| lopdf / pdf crate 嗅探误判 | 中 | 中 | task_009 真实扫描+真实文本各 ≥3 样本互验 |
| symlink 在 zip→dmg 中失效 | 低 | 高 | task_006 DMG 制作用 `hdiutil`，保留 symlink；CI smoke 验证 |
| 签名脚本未签全部 .so | 中 | 高 | task_004 `codesign --verify --deep --strict` 作为 AC |

---

## 9. Task 粒度自检（每个 task）
- 全部 17 task 已通过：单一目标 / 可独立测试 / ≤2000 行变更 / 依赖清晰 / AC 可验证。
- 唯一例外：task_012（真实样本矩阵）量级较大但属于"配置 + 验证"，不超出单 Agent 单会话上限。
