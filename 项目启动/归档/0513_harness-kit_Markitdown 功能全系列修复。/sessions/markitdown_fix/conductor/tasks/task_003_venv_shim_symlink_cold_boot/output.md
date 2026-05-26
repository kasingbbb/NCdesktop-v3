# Task 交付 — task_003_venv_shim_symlink_cold_boot (T-C)

## 实现摘要
按 ADR-003 / H4，建立 `Resources/markitdown-venv/bin/python` symlink venv-shim，**不使用** `python -m venv` 或 `cp -R`，以保留 standalone Python 的 `@executable_path` rpath，确保打包后的 `.app` 不出现 dyld load failure。

- `scripts/prepare-venv-shim.sh`：幂等创建两个**相对** symlink（`python -> ../../python/bin/python3.12`、`python3 -> python`）；自校验"仅 symlink + 必须相对"；脚本顶端显式注释引用 ADR-003 + 原因。
- `scripts/verify-venv-shim.sh`：先做结构校验（相对 symlink + 目录纯净），再用 `env -i PATH=/usr/bin:/bin HOME="${HOME}"` 启干净 shell，跑 ADR-010 / runtime-manifest 字面对齐的 imports 探针 `import ebooklib, bs4, pdfminer, pptx, mammoth, openpyxl, PIL; print("ok")`（**mammoth 非 docx**，遵守 E-2 2026-05-13 修订）。

## 修改的文件
| 文件路径 | 变更类型 | 说明 |
|---|---|---|
| `NCdesktop/scripts/prepare-venv-shim.sh` | 新建 | 幂等创建 symlink venv-shim，含 AC-1/2/4 自校验 |
| `NCdesktop/scripts/verify-venv-shim.sh` | 新建 | 干净 shell 冷启动 imports 探针（AC-3）+ AC-4 + 纯 symlink 约束校验 |
| `NCdesktop/src-tauri/resources/markitdown-venv/bin/python` | 产物（symlink） | `-> ../../python/bin/python3.12` |
| `NCdesktop/src-tauri/resources/markitdown-venv/bin/python3` | 产物（symlink） | `-> python` |

## 对 Architect 方案的遵守声明
- [x] 目录结构与 ADR-003 一致（`Resources/markitdown-venv/bin/` 内仅 symlink）
- [x] API 路径/命名与 Architect 方案一致（runtime 探测路径未改动）
- [x] 数据模型与 Architect 方案一致（N/A — 脚本任务）
- [x] 未引入计划外的新依赖（纯 bash + coreutils）
- [x] **明勾 ADR-003**：脚本顶端 12 行注释直接引用决策与原因；实现严禁 `python -m venv` / `--copies` / `cp -R` / `virtualenv`
- [x] **AC-3 imports 列表字面一致**：`ebooklib, bs4, pdfminer, pptx, mammoth, openpyxl, PIL`（7 项，含 `mammoth`，对齐 E-2 修订 + ADR-010 + runtime-manifest）
- 偏离说明：无

## 测试命令
```bash
cd NCdesktop
bash -n scripts/prepare-venv-shim.sh
bash -n scripts/verify-venv-shim.sh
./scripts/prepare-venv-shim.sh
readlink src-tauri/resources/markitdown-venv/bin/python
readlink src-tauri/resources/markitdown-venv/bin/python3
./scripts/prepare-venv-shim.sh        # 幂等
./scripts/verify-venv-shim.sh
```

## 测试结果
```
$ ./scripts/prepare-venv-shim.sh
created shim symlinks under src-tauri/resources/markitdown-venv/bin/
prepare-venv-shim.sh: OK

$ ls -la src-tauri/resources/markitdown-venv/bin/
lrwxr-xr-x ... python  -> ../../python/bin/python3.12
lrwxr-xr-x ... python3 -> python

$ readlink src-tauri/resources/markitdown-venv/bin/python
../../python/bin/python3.12
$ readlink src-tauri/resources/markitdown-venv/bin/python3
python

$ ./scripts/prepare-venv-shim.sh        # 二次运行（幂等）
shim already present, skipping
prepare-venv-shim.sh: OK

$ ./scripts/verify-venv-shim.sh
AC-4 OK: symlinks are relative (../../python/bin/python3.12, python)
structure OK: shim dir contains only the two symlinks
----- import probe output -----
ok
----- exit=0 -----
verify-venv-shim.sh: OK
```

## 自测验证矩阵
| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|---|---|---|---|
| ✅ 正常路径 | 首次运行 prepare + verify | 已测 | PASS — 两 symlink 建立，imports 探针输出 `ok`，exit 0 |
| ✅ 正常路径 | 幂等（二次运行 prepare） | 已测 | PASS — 输出 `shim already present, skipping`，无副作用 |
| ⚠️ 边界条件 | shim 目录内塞入 `pyvenv.cfg` 后跑 prepare | 已测 | PASS — 脚本检测到非 symlink 文件并 exit 1（保护 H4 约束） |
| ⚠️ 边界条件 | symlink 是相对路径（readlink 不以 `/` 开头） | 已测 | PASS — `../../python/bin/python3.12`、`python` 均相对 |
| ❌ 异常路径 | 人为将 `python` 改成绝对 symlink，跑 verify | 已测 | PASS — verify 报 `FAIL: ... is absolute (AC-4)` exit 1 |
| ❌ 异常路径 | 干净 shell（`env -i`）下 imports 全部 7 项探针 | 已测 | PASS — 输出 `ok`，证明无需 PYTHONPATH/VIRTUAL_ENV/PYTHONHOME |
| ⚠️ 边界条件 | macOS 12 / macOS 14 干净 VM 冷启（AC-5） | **PENDING-USER-MACHINE** | 本机为开发机非干净 VM；无 macOS CI runner。需用户在干净 VM 复跑 `verify-venv-shim.sh` |

## 浏览器/运行时验证
**N/A** — 本任务交付物为 shell 脚本与文件系统产物（symlink），不含 UI 或可启动服务。运行时探测路径 `src-tauri/src/extraction/scheduler.rs:531-532` 由后续 task_007 启动期 self-check 接管，本 task 范围内不修改。

## 已知局限
1. **AC-5 干净 VM 验证 PENDING**：当前开发机非干净 macOS 12/14 arm64 VM，且项目无 macOS CI runner。`verify-venv-shim.sh` 已设计为可在任何 macOS 上跑，干净 shell 测试已通过 `env -i` 在本机模拟，但完整的"无 brew/无开发工具链干净 VM"验证需用户在交付侧机器上手测。
2. 脚本假定 cwd 可定位到 `NCdesktop/` 项目根（通过 `SCRIPT_DIR` 自动解析），如果有人把 `scripts/` 目录移走则失效——这是约定俗成的层级假设，非缺陷。
3. 当前 shim 仅覆盖 macOS（symlink 形态）；Windows 打包需另出 task（不在本 task 范围）。

## 需要 Reviewer 特别关注的地方
1. **`prepare-venv-shim.sh` 的"纯净目录"自检逻辑**（脚本约第 72-105 行）：用了两段 `find -mindepth 1 -maxdepth 1` 分别扫 `bin/` 与 `markitdown-venv/` 根，确保任何 `pyvenv.cfg` / `lib/` / `site-packages/` 等 venv 残留都会被拒绝。请确认这两段范围足够（是否需要扩展到深层递归？我的判断是：第一次运行从 `mkdir -p` 起就只可能产生我们写入的 symlink，深层污染场景仅来自人为/外部脚本，浅扫已足够防御性）。
2. **干净 shell 中是否保留 `HOME` 的取舍**（`verify-venv-shim.sh` 约第 71 行）：保留 `HOME` 是为了避免 pip user-site 警告污染 stdout；`PYTHONPATH`/`PYTHONHOME`/`VIRTUAL_ENV` 严格清空。如 Reviewer 认为 AC-3 "干净" 应连 `HOME` 也剥离，可改 `env -i PATH=/usr/bin:/bin`（已确认在本机仍能 `ok`，但风险是某些 macOS 版本下 Python 可能因找不到 user dir 给 stderr 噪声）。

---

**Reviewer 入口**：output.md 路径已就位；prepare/verify 脚本已加可执行位；产物 symlink 已落盘并通过 AC-1~AC-4 全量校验，AC-5 因环境限制标记 PENDING-USER-MACHINE。

---

## FIX-LOG（Reviewer 第 1 轮 FIX 响应）

### 触发与范围
- Scorecard：`scorecard.md` 综合分 4.35 / 5，判定 FIX。
- 必修：**MAJOR-1**（`verify-venv-shim.sh` L81 `HOME="${HOME}"` 透传 → 用户 site-packages 假阳性风险，违背 AC-3 字面 `env -i PATH=/usr/bin:/bin` + "证明 standalone 自给自足"语义）。
- 本轮修复范围严格限定在 `verify-venv-shim.sh`；**未触及** `prepare-venv-shim.sh` / task_001/002 产物 / task_004/007 探测路径 / 任何 dev-desensitize 区文件。

### 方案选择：方案 A（`-E -s`，删 HOME 透传）
- 理由：方案 A 最贴近 AC-3 字面契约（命令行只剩 `env -i PATH=/usr/bin:/bin`），并通过 Python 自身 flag 提供"双重隔离"——`-E` 忽略所有 `PYTHON*` 环境变量（即使有人未来再注入），`-s` 显式跳过 user site。方案 B（`HOME=/var/empty`）虽然功能等价，但在命令行字面上仍多出一个 `HOME=`，比 AC-3 字面契约多出 token；A 更"最小契约对齐"。
- 兼容性：`-E -s` 是 CPython 自带 flag，standalone 3.12 完全支持。

### 关键 diff 摘要（`NCdesktop/scripts/verify-venv-shim.sh`）

**修改 1：注释头补充 FIX 理由（在文件顶部 docstring 末尾追加）**

```diff
 # Imports probe set MUST be byte-identical to ADR-010 / runtime-manifest.json
 # (E-2 revision 2026-05-13): ebooklib, bs4, pdfminer, pptx, mammoth, openpyxl, PIL
 # Note: `mammoth` (NOT `docx`) — this is the post-E-2 contract.
+#
+# FIX (Reviewer round 1, MAJOR-1): the cold-boot probe MUST NOT inherit HOME.
+# AC-3 contract is literally `env -i PATH=/usr/bin:/bin`. Passing HOME lets
+# Python resolve `~/.local/lib/python3.12/site-packages` (the user site dir);
+# if the host happens to have e.g. `pip install --user ebooklib` etc, the
+# probe would "succeed" via the user site rather than the embedded standalone
+# site-packages — a false positive that defeats the very point of AC-3.
+# We use `-E -s` for defense-in-depth: `-E` ignores PYTHON* env vars (already
+# stripped by `env -i`, but ensures forward-compatibility if anyone re-injects),
+# `-s` disables user site-packages so the import resolution is strictly the
+# standalone interpreter's own site-packages.
```

**修改 2：删除 `HOME="${HOME}"` 透传，加入 `-E -s` 双重隔离（脚本约 L74–82）**

```diff
 # Step 2: clean-shell cold-boot import probe (AC-3).
-# env -i strips ALL inherited env. We deliberately re-introduce only PATH
-# (system tools) and HOME (to avoid pip user-site warnings on first run).
-# PYTHONPATH / PYTHONHOME / VIRTUAL_ENV are NOT set — this is the point of
-# the cold-boot test.
+# env -i strips ALL inherited env. We re-introduce ONLY PATH (system tools).
+# HOME is deliberately NOT passed (AC-3 literal: `env -i PATH=/usr/bin:/bin`)
+# so Python cannot resolve `~/.local/lib/.../site-packages` and accidentally
+# satisfy imports via the user site dir. `-E` ignores PYTHON* env vars,
+# `-s` disables user site-packages — double isolation guarantee.
 PROBE='import ebooklib, bs4, pdfminer, pptx, mammoth, openpyxl, PIL; print("ok")'

 set +e
-output="$(env -i PATH=/usr/bin:/bin HOME="${HOME}" \
-    "${SHIM_PY}" -c "${PROBE}" 2>&1)"
+output="$(env -i PATH=/usr/bin:/bin \
+    "${SHIM_PY}" -E -s -c "${PROBE}" 2>&1)"
 status=$?
 set -e
```

**修改 3：新增 Step 3 user-site decoy 负向测试（脚本末尾，主探针 ok 之后）**

设计要点：
- 仅当 `~/.local/lib/python3.12/site-packages/ebooklib.py` **不存在**时才创建 stub（避免污染用户已有文件）。
- stub 内容为 `raise ImportError("user site stub - should NOT be reached (AC-3 isolation broken)")`，一旦被加载会让 probe 非 0 退出。
- 重跑 probe（同样的 `env -i PATH=/usr/bin:/bin ... -E -s` 调用），**期望 `ok` 且 exit 0**——说明 standalone site-packages 是权威解析源，user site 完全不参与。
- 测试完成后通过 `trap cleanup_stub EXIT` 无条件清理 stub 文件，并尝试 `rmdir` 我们创建的空目录链（`-` 嵌套 site-packages / python3.12 / lib），如果用户原本有别的内容则 `rmdir` 静默失败，不破坏宿主。
- 标记位 `STUB_CREATED_BY_US=1` 确保我们只清理自己创建的文件。
- 该测试**幂等可重跑**，**可在用户机本地手跑**（无需 root / 任何外部工具）。

新增段（约 113–166 行）：

```bash
# Step 3: user-site decoy negative test (FIX MAJOR-1).
USER_SITE_DIR="${HOME}/.local/lib/python3.12/site-packages"
STUB_FILE="${USER_SITE_DIR}/ebooklib.py"
STUB_CREATED_BY_US=0

cleanup_stub() {
    if [[ "${STUB_CREATED_BY_US}" == "1" && -f "${STUB_FILE}" ]]; then
        rm -f "${STUB_FILE}"
        rmdir "${USER_SITE_DIR}" 2>/dev/null || true
        rmdir "$(dirname "${USER_SITE_DIR}")" 2>/dev/null || true
        rmdir "$(dirname "$(dirname "${USER_SITE_DIR}")")" 2>/dev/null || true
    fi
}
trap cleanup_stub EXIT

if [[ -e "${STUB_FILE}" ]]; then
    echo "skip user-site decoy test: ${STUB_FILE} already exists (not ours, leaving alone)"
else
    mkdir -p "${USER_SITE_DIR}"
    cat > "${STUB_FILE}" <<'PYSTUB'
raise ImportError("user site stub - should NOT be reached (AC-3 isolation broken)")
PYSTUB
    STUB_CREATED_BY_US=1

    set +e
    decoy_output="$(env -i PATH=/usr/bin:/bin \
        "${SHIM_PY}" -E -s -c "${PROBE}" 2>&1)"
    decoy_status=$?
    set -e

    if [[ ${decoy_status} -ne 0 ]] || ! grep -q '^ok$' <<<"${decoy_output}"; then
        echo "FAIL: user-site decoy was reachable — AC-3 isolation is broken." >&2
        exit 1
    fi
    echo "user-site decoy OK: standalone site-packages is authoritative"
fi
```

### 决定性证据（FIX 前后对比）

为了**实测**证明 MAJOR-1 的风险是真实的、且本次修复确实闭合了它，在本机临时手工对照运行两种形式（均针对同一 stub poisoned 状态）：

| 形式 | 命令 | stdout/stderr | exit |
|---|---|---|---|
| **OLD（buggy）** | `env -i PATH=/usr/bin:/bin HOME="$HOME" python -c PROBE` | `ImportError: DECOY-WAS-LOADED`（来自 `~/.local/lib/.../ebooklib.py`） | **1** |
| **NEW（fix）** | `env -i PATH=/usr/bin:/bin python -E -s -c PROBE` | `ok` | **0** |

→ 证实：旧形式在用户机器存在 user-site `ebooklib` 时**确会假成功为 ImportError 路径**（或反之，假成功为 `ok`），等价于 AC-3 失效；新形式严格走 standalone site-packages，与是否存在 user-site stub 无关。

### 6 项自测矩阵重跑结果

| # | 场景 | 命令 | 期望 | 实际 |
|---|---|---|---|---|
| 1 | 首次 prepare | `./scripts/prepare-venv-shim.sh` | symlink 建立 / OK | **PASS**（`shim already present, skipping; prepare-venv-shim.sh: OK`，shim 在前序 round 已落盘） |
| 2 | prepare 幂等 | 二次运行同上 | 无副作用 / OK | **PASS**（`shim already present, skipping`） |
| 3 | pyvenv.cfg 污染 | `touch markitdown-venv/pyvenv.cfg && prepare` | exit 1 拒绝 | **PASS**（`ERROR: unexpected entries ... pyvenv.cfg; exit=1`） |
| 4 | AC-4 相对 symlink | `readlink` 两条 symlink | 均不以 `/` 开头 | **PASS**（`../../python/bin/python3.12`、`python`） |
| 5 | 绝对 symlink 异常 | `ln -snf $abs_path python && verify` | verify 报 `FAIL: ... is absolute (AC-4)` exit 1 | **PASS** |
| 6 | 干净 shell 7 项 imports（**FIX 后**） | `./scripts/verify-venv-shim.sh` | imports → `ok`，decoy → `ok`，全段 OK | **PASS**（主 probe `ok exit=0` + decoy `ok exit=0` + `user-site decoy OK` + `verify-venv-shim.sh: OK`） |

完整 `./scripts/verify-venv-shim.sh` 输出：

```
AC-4 OK: symlinks are relative (../../python/bin/python3.12, python)
structure OK: shim dir contains only the two symlinks
----- import probe output -----
ok
----- exit=0 -----
----- decoy probe output -----
ok
----- decoy exit=0 -----
user-site decoy OK: standalone site-packages is authoritative
verify-venv-shim.sh: OK
```

### 回归检查
- AC-1（symlink 形态）：未触 prepare，readlink 输出与原始 PASS 状态一致。
- AC-2（无 `python -m venv` / `cp -R`）：本轮仅改 verify，prepare 文本未改。
- AC-3（**修复目标**）：命令行已对齐字面契约 `env -i PATH=/usr/bin:/bin`，并通过 user-site decoy 负向测试证伪"漏检 user site"风险。
- AC-4（相对 symlink）：未触 prepare 与 verify 的结构校验段。
- AC-5（干净 VM）：仍为 PENDING-USER-MACHINE，状态不变；input.md 已授权。

### 授权区遵守
- 仅修改 `NCdesktop/scripts/verify-venv-shim.sh`（dev-pack 允许修改区）。
- **未触及** dev-desensitize 区任何文件（`desensitize-sample.sh` / `encrypt-samples.sh` / `decrypt-samples.sh` / SOP / workflow 均未读未写）。
- 未绕过 `set -euo pipefail`。
- HOME 路径全部派生自 `${HOME}`，无硬编码。

### 已知局限（FIX 后新增）
- user-site decoy 测试在"已有 `~/.local/lib/python3.12/site-packages/ebooklib.py`"的宿主上会**跳过**（避免误删用户文件），日志会显式打印 `skip user-site decoy test`。这是设计上的保守取舍：本测试的核心价值是"在干净宿主上证明隔离生效"，而不是"破坏宿主已有内容"。若 CI/审查者机器恰好有 user-site `ebooklib`，建议先 `pip uninstall --user ebooklib` 后再跑 verify。
