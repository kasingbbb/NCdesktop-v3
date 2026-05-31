#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# prepare-embedded-kc-runtime.test.sh
#
# Task: task_027_dmg_packaging_kc — 静态测试（不打真 DMG，不依赖 KC repo）
#
# 覆盖范围（PM 真机验证之前的 CI 守护）:
#   T1: bash -n 语法 check （编译期守护）
#   T2: shellcheck 0 warning （若 shellcheck 已装；未装则 skip）
#   T3: --dry-run 在 mock APP_PATH / KC_REPO_PATH 下能成功输出 plan 不写盘
#   T4: 缺参数 / 缺 .app / 缺 KC repo 时正确报错 exit ≠ 0
#   T5: kc-requirements.txt 不含红线包 (gradio / pandas / numpy /
#       huggingface_hub / torch / transformers)
#
# 运行: bash scripts/__tests__/prepare-embedded-kc-runtime.test.sh
# 退出码: 0 = 全过；任一 fail = 非零
# ---------------------------------------------------------------------------
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCRIPTS_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
TARGET_SCRIPT="${SCRIPTS_DIR}/prepare-embedded-kc-runtime.sh"
KC_REQS="${SCRIPTS_DIR}/kc-requirements.txt"

PASS=0
FAIL=0
SKIP=0

ok()   { echo "  [PASS] $*"; PASS=$((PASS+1)); }
ng()   { echo "  [FAIL] $*" >&2; FAIL=$((FAIL+1)); }
skip() { echo "  [SKIP] $*"; SKIP=$((SKIP+1)); }
hdr()  { echo ""; echo "── $* ──"; }

# ---- T1: bash -n ----------------------------------------------------------
hdr "T1: bash -n syntax check"
if bash -n "${TARGET_SCRIPT}"; then
  ok "bash -n ${TARGET_SCRIPT}"
else
  ng "bash -n ${TARGET_SCRIPT} returned non-zero"
fi

# ---- T2: shellcheck (optional) --------------------------------------------
hdr "T2: shellcheck (optional)"
if command -v shellcheck >/dev/null 2>&1; then
  # SC1091 (sourced file not found) and SC2086 (word splitting) — accept,
  # we run heredocs with explicit quoting.
  if shellcheck -e SC1091 "${TARGET_SCRIPT}"; then
    ok "shellcheck clean"
  else
    ng "shellcheck reported issues"
  fi
else
  skip "shellcheck not installed in environment"
fi

# ---- T3: --dry-run with mock APP_PATH + KC_REPO_PATH ----------------------
hdr "T3: --dry-run smoke (mock app + mock kc repo)"
TMPDIR="$(mktemp -d -t kc-runtime-test.XXXXXX)"
trap 'rm -rf "${TMPDIR}"' EXIT

# Mock .app bundle (just need the directory)
MOCK_APP="${TMPDIR}/Mock.app"
mkdir -p "${MOCK_APP}/Contents/Resources"

# Mock KC repo with whitelisted source files
MOCK_KC="${TMPDIR}/MockKC"
mkdir -p "${MOCK_KC}/compiler"
mkdir -p "${MOCK_KC}/notecapt"  # task_001 调研误判修正后入白名单
touch "${MOCK_KC}/notecapt/__init__.py"
touch "${MOCK_KC}/compiler/__init__.py"
touch "${MOCK_KC}/run_api.py"

# Pick a python that exists on this machine (test must work in CI)
PYTHON_FOR_TEST="python3"
if ! command -v "${PYTHON_FOR_TEST}" >/dev/null 2>&1; then
  PYTHON_FOR_TEST="python"
fi

if bash "${TARGET_SCRIPT}" --dry-run "${MOCK_APP}" "${MOCK_KC}" "${PYTHON_FOR_TEST}" >/dev/null 2>&1; then
  ok "--dry-run exit 0 with valid inputs"
  # Confirm no writes happened
  if [[ ! -e "${MOCK_APP}/Contents/Resources/kc" ]]; then
    ok "--dry-run did NOT create kc/ directory"
  else
    ng "--dry-run unexpectedly created kc/ directory"
  fi
else
  ng "--dry-run failed with valid mock inputs (python=${PYTHON_FOR_TEST})"
fi

# ---- T4: 错误输入正确报错 -------------------------------------------------
hdr "T4: error paths exit non-zero"

# T4a: 无参数
if bash "${TARGET_SCRIPT}" >/dev/null 2>&1; then
  ng "no-args should exit non-zero, got 0"
else
  ok "no-args exits non-zero"
fi

# T4b: .app 不存在
if bash "${TARGET_SCRIPT}" --dry-run "${TMPDIR}/nonexistent.app" "${MOCK_KC}" "${PYTHON_FOR_TEST}" >/dev/null 2>&1; then
  ng "missing .app should exit non-zero, got 0"
else
  ok "missing .app exits non-zero"
fi

# T4c: KC repo 不存在
if bash "${TARGET_SCRIPT}" --dry-run "${MOCK_APP}" "${TMPDIR}/nonexistent-kc" "${PYTHON_FOR_TEST}" >/dev/null 2>&1; then
  ng "missing KC repo should exit non-zero, got 0"
else
  ok "missing KC repo exits non-zero"
fi

# T4d: KC repo 缺少 compiler/
MOCK_KC_PARTIAL="${TMPDIR}/MockKCPartial"
mkdir -p "${MOCK_KC_PARTIAL}"
touch "${MOCK_KC_PARTIAL}/run_api.py"  # 故意缺 compiler/
if bash "${TARGET_SCRIPT}" --dry-run "${MOCK_APP}" "${MOCK_KC_PARTIAL}" "${PYTHON_FOR_TEST}" >/dev/null 2>&1; then
  ng "KC repo missing compiler/ should exit non-zero, got 0"
else
  ok "KC repo missing compiler/ exits non-zero"
fi

# T4e: python 不存在
if bash "${TARGET_SCRIPT}" --dry-run "${MOCK_APP}" "${MOCK_KC}" "definitely-not-a-python-binary-xyz123" >/dev/null 2>&1; then
  ng "missing python binary should exit non-zero, got 0"
else
  ok "missing python binary exits non-zero"
fi

# ---- T5: kc-requirements.txt 红线检查 -------------------------------------
hdr "T5: kc-requirements.txt 红线 (no gradio/pandas/numpy/huggingface_hub/torch/transformers)"
if [[ ! -f "${KC_REQS}" ]]; then
  ng "kc-requirements.txt not found: ${KC_REQS}"
else
  # PM 2026-05-28 方案 A 后修订：numpy/scipy/faiss-cpu/hdbscan 从红线移除
  #（KC pipeline_b_semantic 主链路真需要）。仍保留 6 个红线包。
  REDLINE_PATTERNS=(
    "^gradio[[:space:]]*[=><~]"
    "^pandas[[:space:]]*[=><~]"
    "^huggingface_hub[[:space:]]*[=><~]"
    "^torch[[:space:]]*[=><~]"
    "^transformers[[:space:]]*[=><~]"
    "^jupyter[[:space:]]*[=><~]"
  )
  # Strip comments before grep — 注释里的红线名提醒不算违规
  STRIPPED="$(grep -v '^[[:space:]]*#' "${KC_REQS}" | grep -v '^[[:space:]]*$' || true)"
  any_fail=0
  for pat in "${REDLINE_PATTERNS[@]}"; do
    if echo "${STRIPPED}" | grep -E -q "${pat}"; then
      ng "redline package matched: ${pat}"
      any_fail=1
    fi
  done
  if [[ "${any_fail}" == "0" ]]; then
    ok "no redline packages in kc-requirements.txt"
  fi

  # 反向校验：核心依赖必须存在（保证 KC 服务能起来）
  REQUIRED_PATTERNS=(
    "^fastapi"
    "^uvicorn"
    "^pydantic"
    "^langchain"
    "^openai"
  )
  for pat in "${REQUIRED_PATTERNS[@]}"; do
    if echo "${STRIPPED}" | grep -E -q "${pat}"; then
      ok "required dep present: ${pat}"
    else
      ng "required dep missing: ${pat}"
    fi
  done
fi

# ---- T6: D4 manifest fail-fast + merge 守护 --------------------------------
# 提取脚本里第一个 PYEOF heredoc（manifest 写入块），用 env var 驱动直接跑，
# 不需要真 venv。验证：缺失 manifest → exit≠0；损坏 manifest → exit≠0；
# 合法 manifest → exit 0 且 markitdown 顶层字段被保留（merge 不 clobber）。
hdr "T6: D4 manifest fail-fast + merge preserves markitdown fields"

# 用 python 抽出脚本里第一个 <<'PYEOF' ... PYEOF 块到临时文件
MANIFEST_PY="${TMPDIR}/manifest_block.py"
if "${PYTHON_FOR_TEST}" - "${TARGET_SCRIPT}" "${MANIFEST_PY}" <<'EXTRACT'; then
import re, sys
src = open(sys.argv[1], encoding="utf-8").read()
m = re.search(r"<<'PYEOF'\n(.*?)\nPYEOF", src, re.DOTALL)
if not m:
    sys.exit("could not extract manifest heredoc")
open(sys.argv[2], "w", encoding="utf-8").write(m.group(1))
EXTRACT
  : # extracted ok
else
  ng "T6: failed to extract manifest heredoc from script"
fi

# 公共 env（除 MANIFEST_PATH 外都固定）
export PREP_KC_PYPROJECT="${TMPDIR}/nonexistent-pyproject.toml"
export PREP_KC_COMMIT_SHA="deadbeef"
export PREP_KC_PYTHON_VERSION="Python 3.12.0"
export PREP_KC_VENV_SIZE_BYTES="12345"
export PREP_KC_BUILD_TS="2026-05-30T00:00:00Z"
export PREP_KC_RUNTIME_ID="ncdesktop-kc-runtime"

if [[ -f "${MANIFEST_PY}" ]]; then
  # T6a: manifest 缺失 → fail-fast exit≠0
  export PREP_KC_MANIFEST_PATH="${TMPDIR}/no-such-manifest.json"
  if "${PYTHON_FOR_TEST}" "${MANIFEST_PY}" >/dev/null 2>&1; then
    ng "T6a: missing manifest should exit non-zero, got 0 (D4 regression)"
  else
    ok "T6a: missing manifest fails fast (no phantom manifest created)"
  fi

  # T6b: 损坏 manifest → fail-fast exit≠0
  BAD_MANIFEST="${TMPDIR}/corrupt-manifest.json"
  printf '{ this is not valid json' > "${BAD_MANIFEST}"
  export PREP_KC_MANIFEST_PATH="${BAD_MANIFEST}"
  if "${PYTHON_FOR_TEST}" "${MANIFEST_PY}" >/dev/null 2>&1; then
    ng "T6b: corrupt manifest should exit non-zero, got 0 (D4 regression)"
  else
    ok "T6b: corrupt manifest fails fast (does not clobber)"
  fi

  # T6c: 合法 manifest（带 markitdown 顶层字段）→ exit 0 且字段保留
  GOOD_MANIFEST="${TMPDIR}/good-manifest.json"
  printf '{"schema_version":1,"runtime_id":"nc-md","markitdown":{"version":"0.1"},"imports":["x"]}' > "${GOOD_MANIFEST}"
  export PREP_KC_MANIFEST_PATH="${GOOD_MANIFEST}"
  if "${PYTHON_FOR_TEST}" "${MANIFEST_PY}" >/dev/null 2>&1; then
    # 验证 markitdown / runtime_id / imports 仍在，且新增了 kc 字段
    if "${PYTHON_FOR_TEST}" - "${GOOD_MANIFEST}" <<'CHECK'
import json, sys
d = json.load(open(sys.argv[1], encoding="utf-8"))
assert "markitdown" in d, "markitdown top-level field clobbered"
assert d.get("runtime_id") == "nc-md", "runtime_id clobbered"
assert d.get("imports") == ["x"], "imports clobbered"
assert "kc" in d and d["kc"]["runtime_id"] == "ncdesktop-kc-runtime", "kc field missing"
CHECK
    then
      ok "T6c: valid manifest merged — markitdown fields preserved + kc added"
    else
      ng "T6c: merge clobbered markitdown top-level fields"
    fi
  else
    ng "T6c: valid manifest merge should exit 0, got non-zero"
  fi
else
  ng "T6: manifest block not extracted; skipping T6a-c"
fi

# 清理 T6 注入的 env（避免污染后续）
unset PREP_KC_MANIFEST_PATH PREP_KC_PYPROJECT PREP_KC_COMMIT_SHA \
      PREP_KC_PYTHON_VERSION PREP_KC_VENV_SIZE_BYTES PREP_KC_BUILD_TS \
      PREP_KC_RUNTIME_ID

# ---- 汇总 -----------------------------------------------------------------
echo ""
echo "════════════════════════════════════════════"
echo "  prepare-embedded-kc-runtime.test.sh"
echo "  PASS: ${PASS}    FAIL: ${FAIL}    SKIP: ${SKIP}"
echo "════════════════════════════════════════════"

if (( FAIL > 0 )); then
  exit 1
fi
exit 0
