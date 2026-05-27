# Task Output — task_027_dmg_packaging_kc

**Status**: Dev 实装完成；DMG 真机打包 + .app 启动验证 → PM dry-run。
**Branch**: `feat/windows-unit-13-cloud-ai`
**Master HEAD (pre)**: `c0ebde15`
**Complexity**: M (3d 估时；本次实装 ~3h 写代码 + 测试)

---

## 1. 交付清单

| # | 文件 | 类型 | 行数 | 说明 |
|---|---|---|---|---|
| 1 | `scripts/prepare-embedded-kc-runtime.sh` | 新建 (chmod +x) | ~210 | 主注入脚本；3 参数 + `--dry-run` flag |
| 2 | `scripts/kc-requirements.txt` | 新建 | ~60 | KC venv pip deps；含红线注释 |
| 3 | `scripts/__tests__/prepare-embedded-kc-runtime.test.sh` | 新建 (chmod +x) | ~155 | 14 项静态断言；T1-T5 |
| 4 | `项目启动/_harness-kit_合并管线和NC/.../task_027_dmg_packaging_kc/output.md` | 本文件 | — | PM 真机验证清单 |

**未改动**:
- `scripts/build-macos-dmg.sh` — 主 DMG 编排脚本；KC 注入步骤未自动注入到此处，原因：KC repo path 是参数（外部传入），不便硬编码。PM 在真机上手动追加一行调用即可（见 §3 步骤 3）。本次留作 task_028 / PM 决策。
- `src-tauri/tauri.conf.json` — 无需改；`kc/` 目录是运行时由 Tauri 直接读 `.app/Contents/Resources/kc/`，不走 Tauri resources 配置。
- 任何 Rust 代码 / Cargo lib — 0 cargo test 退化。

---

## 2. 实装重点

### 2.1 `prepare-embedded-kc-runtime.sh` 关键决策

| 决策点 | 实装 | 理由 |
|---|---|---|
| 参数签名 | `[--dry-run] APP_PATH KC_REPO_PATH [PYTHON_BIN]` | input.md AC-1；`--dry-run` 是 Dev 额外补充（静态测试需要） |
| Python 版本默认 | `python3.11` | input.md / KC langchain ≥3.10 要求 |
| 源码白名单 | `compiler/` + `run_api.py` 仅 2 项 | input.md AC-1 Step 3；跳过 notecapt/ gradio_demo.py examples/ tests/ |
| Pip 安装策略 | 含传递依赖 (`pip install -r`, 不用 `--no-deps`) | KC langchain 链版本范围松；由 pip 解析。区别于 markitdown 走 `--no-deps + lock` |
| Manifest 写入 | inline Python heredoc，env 透传变量 | 避免 shell 引号注入；不引 jq；与 markitdown 风格一致（`du`/`json` 都走 `python3`） |
| 幂等保证 | `rm -rf "${KC_TARGET}"` 后重建 | input.md §技术约束 |
| Manifest schema | 在现有 manifest 上 `m['kc'] = {...}`，保留 markitdown/python 字段 | ADR-010 single source of truth；不 clobber 前序字段 |
| Manifest 新增字段 | `runtime_id` / `version` / `commit_sha` / `python_version` / `venv_size_bytes` / `build_timestamp` | AC-4 必填 + Dev 补 size_bytes（DMG 体积守护用） |
| commit_sha 容错 | KC 不是 git checkout 时标 `unknown` | 真机 KC 可能是 release tarball |
| 跨平台 | 不用 `sed -i`；`find -exec rm -rf {} +` macOS/Linux 通用 | input.md §技术约束 |
| 体积验证 | 末尾 `du -sh "${KC_TARGET}"` | AC-6（PM 真机看输出） |

### 2.2 `kc-requirements.txt` 红线策略

- **入包**（13 项）: fastapi / uvicorn[standard] / python-multipart / pydantic / pydantic-settings / langchain / langchain-openai / openai / requests / aiosqlite / httpx / markdown-it-py / jieba / tiktoken / pyyaml
- **红线**（注释中显式标出，测试 T5 守护）: gradio (+45MB), pandas (+25MB), numpy (+18MB), huggingface_hub (+15MB), torch (+500MB+), transformers, jupyter

### 2.3 静态测试 (T1-T5, 14 PASS / 1 SKIP)

```
PASS: 14    FAIL: 0    SKIP: 1
```

- T1 bash -n: 主脚本语法 OK
- T2 shellcheck: 环境未装，SKIP（PM 真机可补跑）
- T3 --dry-run: mock APP + mock KC repo 下 exit 0，且 **不写盘**（无 `kc/` 目录创建）
- T4 错误路径: 5 项 — no-args / missing .app / missing KC repo / KC repo missing compiler/ / missing python binary 全部正确 exit ≠ 0
- T5 kc-requirements.txt: 7 红线 grep miss，5 核心依赖 grep hit

---

## 3. PM 真机验证清单

> **环境**: macOS arm64 + Python 3.11 + KC repo + Tauri build chain（pnpm + cargo）。
> **预期产物**: `.app` 含 `Contents/Resources/kc/{venv,src}`；DMG 总体积 ≤300MB。

### 3.1 步骤（5 大步）

```bash
# Step 0: 切到 master HEAD，进入 NC 目录
git checkout feat/windows-unit-13-cloud-ai
cd "项目启动/NCdesktop"

# Step 1: 提供 KC repo 本地 path (示例)
KC_REPO_PATH="$HOME/projects/KnowledgeCompiler"
[[ -d "${KC_REPO_PATH}/compiler" ]] && [[ -f "${KC_REPO_PATH}/run_api.py" ]] || \
  echo "KC repo invalid — check path"

# Step 2: 静态测试（CI 守护，秒级）
bash scripts/__tests__/prepare-embedded-kc-runtime.test.sh
# 期望: PASS: 14+    FAIL: 0    (装了 shellcheck 则 15)

# Step 3: 完整 Tauri build (release)，~3-5 min
pnpm install
pnpm tauri build --bundles app
# 产出: src-tauri/target/release/bundle/macos/NoteCapt.app

# Step 4: 注入 KC venv + 源码（本 task 主交付）
bash scripts/prepare-embedded-kc-runtime.sh \
  "src-tauri/target/release/bundle/macos/NoteCapt.app" \
  "${KC_REPO_PATH}" \
  "python3.11"
# 期望最后两行:
#   [prepare-kc-runtime] kc resource size:
#   xxx M    .../Contents/Resources/kc
#   [prepare-kc-runtime] Done. injected to .../Contents/Resources/kc

# Step 5: 体积验证 (AC-6) + manifest 验证 (AC-4)
du -sh src-tauri/target/release/bundle/macos/NoteCapt.app/Contents/Resources/kc
# 期望: ~150-180MB (剥离前)

cat src-tauri/target/release/bundle/macos/NoteCapt.app/Contents/Resources/runtime-manifest.json
# 期望含字段:
#   "kc": {
#     "runtime_id": "ncdesktop-kc-runtime",
#     "version": "...",
#     "commit_sha": "<40-char hex or 'unknown'>",
#     "python_version": "Python 3.11.x",
#     "venv_size_bytes": <int>,
#     "build_timestamp": "..."
#   }
#   且 "markitdown" / "python" / "imports" 字段未丢
```

### 3.2 启动验证（手动）

```bash
open src-tauri/target/release/bundle/macos/NoteCapt.app
# 期望:
#   - NC 主窗口正常打开
#   - 在 NC 内触发知识压缩功能 → KC 子进程能起来 (检查 ps aux | grep run_api)
#   - 日志 (~/Library/Logs/NoteCapt/) 含 KC 启动条目，无 ImportError
```

### 3.3 完整 DMG 打包（可选，~10min）

```bash
bash scripts/build-macos-dmg.sh --release
# ⚠️ 注意: 当前 build-macos-dmg.sh 还未串接 prepare-embedded-kc-runtime.sh。
# PM 需在 step 4b 之后、step 5 sign 之前，手动追加一行（或 patch 此脚本）:
#   bash "${ROOT_DIR}/scripts/prepare-embedded-kc-runtime.sh" \
#     "${APP_BUNDLE_PATH}" "${KC_REPO_PATH:-./vendor/KnowledgeCompiler}"
# task_028 可正式落入此脚本（含 KC_REPO_PATH env 变量约定）。
```

### 3.4 失败回滚

```bash
# 任意步骤失败 → kc 注入是幂等的，rm -rf 即清：
rm -rf src-tauri/target/release/bundle/macos/NoteCapt.app/Contents/Resources/kc
# 然后重跑 Step 4
```

---

## 4. 已知遗留（task_028 处理）

1. **build-macos-dmg.sh 未自动注入 KC**：PM 真机决定 `KC_REPO_PATH` env 后再串。Dev 不擅自硬编码 vendor 路径。
2. **venv 剥离**：input.md AC-6 提到"剥离后 76MB"，task_028 单独优化（删 .pyi / __pycache__ 已删过 / 删 tests/）。本 task 仅产出"剥离前" venv。
3. **签名**：input.md §技术约束明示"不签名（签名在主脚本下一步）"。本脚本不触 codesign。
4. **真机 DMG 增量大小验证**：~80MB (PRD §6) 需 PM 真机比对 master 当前 DMG 与本 task 后 DMG。

---

## 5. 静态质量门

| 门 | 状态 | 命令 / 证据 |
|---|---|---|
| bash -n (主脚本) | PASSED | `bash -n scripts/prepare-embedded-kc-runtime.sh` exit 0 |
| bash -n (测试脚本) | PASSED | `bash -n scripts/__tests__/prepare-embedded-kc-runtime.test.sh` exit 0 |
| 静态测试套件 | 14 PASS / 0 FAIL / 1 SKIP | `bash scripts/__tests__/prepare-embedded-kc-runtime.test.sh` |
| shellcheck | SKIP (未装) | PM 真机若装 shellcheck，运行测试套件 T2 自动跑 |
| cargo lib test | 未触动 (0 退化) | 本 task 不动 Rust 代码 |
| chmod +x | PASSED | 两个 .sh 文件 mode 含 user-execute |

---

## 6. 提交信息

```
build(packaging): task_027 — prepare-embedded-kc-runtime.sh + kc-requirements.txt（PM 真机验证待跑）
```
