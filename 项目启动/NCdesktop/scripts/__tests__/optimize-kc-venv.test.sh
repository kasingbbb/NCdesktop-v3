#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# optimize-kc-venv.test.sh
#
# Task: task_028_kc_venv_optimize — 静态测试（不依赖真 KC venv）
#
# 覆盖范围（PM 真机剥离之前的 CI 守护）:
#   T1: bash -n 语法 check
#   T2: shellcheck（若装，否则 skip）
#   T3: --dry-run 在 mock venv 下输出 plan 不删文件
#   T4: 缺参数 / 不存在路径 / 非 venv 目录 时正确报错 exit ≠ 0
#   T5: 真删剥离 — mock venv 含 RECORD + .pyi + tests/ + LICENSE，跑脚本后
#       这些文件确实被删，但 jieba/dict.txt 保留 + 顶层包根 LICENSE 保留
#   T6: 幂等 — 同一 mock venv 跑 2 次脚本不报错
#   T7: 体积报告输出含 "size before" / "size after" / "saved" 行
#
# 运行: bash scripts/__tests__/optimize-kc-venv.test.sh
# 退出码: 0 = 全过；任一 fail = 非零
# ---------------------------------------------------------------------------
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCRIPTS_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
TARGET_SCRIPT="${SCRIPTS_DIR}/optimize-kc-venv.sh"

PASS=0
FAIL=0
SKIP=0

ok()   { echo "  [PASS] $*"; PASS=$((PASS+1)); }
ng()   { echo "  [FAIL] $*" >&2; FAIL=$((FAIL+1)); }
skip() { echo "  [SKIP] $*"; SKIP=$((SKIP+1)); }
hdr()  { echo ""; echo "── $* ──"; }

# ---- mock venv fixture builder --------------------------------------------
# 构造一个最小的 mock venv：
#   bin/python  (空文件)
#   lib/python3.11/site-packages/
#     somepkg/__init__.py
#     somepkg/types.pyi               <- step 2 应删
#     somepkg/tests/test_foo.py       <- step 3 应删
#     somepkg-1.0.dist-info/RECORD    <- step 1 应删
#     somepkg-1.0.dist-info/METADATA  <- 保留
#     somepkg-1.0.dist-info/LICENSE   <- step 4 应删
#     somepkg-1.0.dist-info/AUTHORS   <- step 4 应删
#     somepkg-1.0.dist-info/NOTICE    <- step 4 应删
#     somepkg-1.0.dist-info/INSTALLER <- step 4 应删
#     somepkg/LICENSE                 <- 包根 LICENSE 保留（不在 dist-info 内）
#     jieba/dict.txt                  <- 保留（KC 中文分词必需）
#     jieba/tests/__init__.py         <- 红线: jieba/tests 不应被删
#       （但当前 Step 3 实装是 ! -path "*/jieba/*" 排除，需要测试守护）
make_mock_venv() {
  local root="$1"
  mkdir -p "${root}/bin"
  : > "${root}/bin/python"
  local sp="${root}/lib/python3.11/site-packages"
  mkdir -p "${sp}/somepkg/tests"
  mkdir -p "${sp}/somepkg-1.0.dist-info"
  mkdir -p "${sp}/jieba/tests"
  : > "${sp}/somepkg/__init__.py"
  : > "${sp}/somepkg/types.pyi"
  echo "def test_foo(): pass" > "${sp}/somepkg/tests/test_foo.py"
  echo "somepkg/__init__.py,sha256=abc,123" > "${sp}/somepkg-1.0.dist-info/RECORD"
  echo "Metadata-Version: 2.1" > "${sp}/somepkg-1.0.dist-info/METADATA"
  echo "Apache 2.0" > "${sp}/somepkg-1.0.dist-info/LICENSE"
  echo "Alice" > "${sp}/somepkg-1.0.dist-info/AUTHORS"
  echo "Notice text" > "${sp}/somepkg-1.0.dist-info/NOTICE"
  echo "pip" > "${sp}/somepkg-1.0.dist-info/INSTALLER"
  echo "MIT" > "${sp}/somepkg/LICENSE"
  echo "我 1\n他 1" > "${sp}/jieba/dict.txt"
  : > "${sp}/jieba/__init__.py"
  : > "${sp}/jieba/tests/__init__.py"
}

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
  if shellcheck -e SC1091 "${TARGET_SCRIPT}"; then
    ok "shellcheck clean"
  else
    ng "shellcheck reported issues"
  fi
else
  skip "shellcheck not installed in environment"
fi

# ---- T3: --dry-run on mock venv (no deletes) ------------------------------
hdr "T3: --dry-run smoke (mock venv, no deletes)"
TMPDIR="$(mktemp -d -t kc-optimize-test.XXXXXX)"
trap 'rm -rf "${TMPDIR}"' EXIT
MOCK_VENV="${TMPDIR}/venv"
make_mock_venv "${MOCK_VENV}"

# 计录跑前文件清单
BEFORE_FILES="$(find "${MOCK_VENV}" -type f | sort)"

DRY_OUT="$(bash "${TARGET_SCRIPT}" --dry-run "${MOCK_VENV}" 2>&1)"
DRY_RC=$?
if [[ ${DRY_RC} -eq 0 ]]; then
  ok "dry-run exit 0"
else
  ng "dry-run exit code = ${DRY_RC}"
fi

AFTER_DRY_FILES="$(find "${MOCK_VENV}" -type f | sort)"
if [[ "${BEFORE_FILES}" == "${AFTER_DRY_FILES}" ]]; then
  ok "dry-run did not delete files"
else
  ng "dry-run modified filesystem (regression!)"
fi

if echo "${DRY_OUT}" | grep -q "DRY-RUN"; then
  ok "dry-run output contains DRY-RUN marker"
else
  ng "dry-run output missing DRY-RUN marker"
fi

# ---- T4: error paths ------------------------------------------------------
hdr "T4: error exit codes"

# T4.1: missing arg
if bash "${TARGET_SCRIPT}" >/dev/null 2>&1; then
  ng "missing arg should exit non-zero"
else
  ok "missing arg correctly errors"
fi

# T4.2: nonexistent path
if bash "${TARGET_SCRIPT}" /no/such/path >/dev/null 2>&1; then
  ng "nonexistent path should exit non-zero"
else
  ok "nonexistent path correctly errors"
fi

# T4.3: not a venv (no lib/)
NOTAVENV="${TMPDIR}/notavenv"
mkdir -p "${NOTAVENV}/bin"
if bash "${TARGET_SCRIPT}" "${NOTAVENV}" >/dev/null 2>&1; then
  ng "non-venv dir (missing lib/) should exit non-zero"
else
  ok "non-venv dir correctly errors"
fi

# T4.4: unknown flag
if bash "${TARGET_SCRIPT}" --bogus-flag "${MOCK_VENV}" >/dev/null 2>&1; then
  ng "unknown flag should exit non-zero"
else
  ok "unknown flag correctly errors"
fi

# ---- T5: real strip on mock venv ------------------------------------------
hdr "T5: real strip removes target files; preserves protected files"
# T5 用全新 fixture（避免 T3 dry-run 残留）
MOCK_VENV2="${TMPDIR}/venv2"
make_mock_venv "${MOCK_VENV2}"

bash "${TARGET_SCRIPT}" "${MOCK_VENV2}" >/dev/null 2>&1
STRIP_RC=$?
if [[ ${STRIP_RC} -eq 0 ]]; then
  ok "strip mode exit 0"
else
  ng "strip mode exit ${STRIP_RC}"
fi

# 应删: RECORD / *.pyi / dist-info LICENSE/AUTHORS/NOTICE/INSTALLER / tests/
if [[ ! -f "${MOCK_VENV2}/lib/python3.11/site-packages/somepkg-1.0.dist-info/RECORD" ]]; then
  ok "step 1: RECORD deleted"
else
  ng "step 1: RECORD survived"
fi

if [[ ! -f "${MOCK_VENV2}/lib/python3.11/site-packages/somepkg/types.pyi" ]]; then
  ok "step 2: .pyi deleted"
else
  ng "step 2: .pyi survived"
fi

if [[ ! -d "${MOCK_VENV2}/lib/python3.11/site-packages/somepkg/tests" ]]; then
  ok "step 3: site-packages/somepkg/tests/ deleted"
else
  ng "step 3: somepkg/tests/ survived"
fi

if [[ ! -f "${MOCK_VENV2}/lib/python3.11/site-packages/somepkg-1.0.dist-info/LICENSE" ]]; then
  ok "step 4: dist-info LICENSE deleted"
else
  ng "step 4: dist-info LICENSE survived"
fi

if [[ ! -f "${MOCK_VENV2}/lib/python3.11/site-packages/somepkg-1.0.dist-info/AUTHORS" ]]; then
  ok "step 4: dist-info AUTHORS deleted"
else
  ng "step 4: dist-info AUTHORS survived"
fi

if [[ ! -f "${MOCK_VENV2}/lib/python3.11/site-packages/somepkg-1.0.dist-info/NOTICE" ]]; then
  ok "step 4: dist-info NOTICE deleted"
else
  ng "step 4: dist-info NOTICE survived"
fi

if [[ ! -f "${MOCK_VENV2}/lib/python3.11/site-packages/somepkg-1.0.dist-info/INSTALLER" ]]; then
  ok "step 4: dist-info INSTALLER deleted"
else
  ng "step 4: dist-info INSTALLER survived"
fi

# 应保留: dist-info METADATA / 顶层包 LICENSE / jieba/dict.txt / jieba/tests/
if [[ -f "${MOCK_VENV2}/lib/python3.11/site-packages/somepkg-1.0.dist-info/METADATA" ]]; then
  ok "preserved: dist-info METADATA"
else
  ng "regression: dist-info METADATA deleted (运行时可能需要)"
fi

if [[ -f "${MOCK_VENV2}/lib/python3.11/site-packages/somepkg/LICENSE" ]]; then
  ok "preserved: top-level pkg LICENSE (not in dist-info)"
else
  ng "regression: top-level pkg LICENSE deleted"
fi

if [[ -f "${MOCK_VENV2}/lib/python3.11/site-packages/jieba/dict.txt" ]]; then
  ok "preserved: jieba/dict.txt (中文分词必需)"
else
  ng "BLOCKER: jieba/dict.txt deleted (KC 中文分词会崩!)"
fi

# 红线: jieba/tests 不应被删（脚本 Step 3 用 ! -path "*/jieba/*" 排除）
if [[ -d "${MOCK_VENV2}/lib/python3.11/site-packages/jieba/tests" ]]; then
  ok "preserved: jieba/tests/ (red-line: ! -path *jieba* 排除)"
else
  ng "red-line breach: jieba/tests/ deleted (脚本 Step 3 排除失效)"
fi

# ---- T6: idempotent — second run does not error ---------------------------
hdr "T6: idempotent (second run is no-op + exit 0)"
if bash "${TARGET_SCRIPT}" "${MOCK_VENV2}" >/dev/null 2>&1; then
  ok "second run exit 0 (idempotent)"
else
  ng "second run errored (not idempotent)"
fi

# ---- T7: size report contains expected lines ------------------------------
hdr "T7: size report output format"
MOCK_VENV3="${TMPDIR}/venv3"
make_mock_venv "${MOCK_VENV3}"
RUN_OUT="$(bash "${TARGET_SCRIPT}" "${MOCK_VENV3}" 2>&1)"
if echo "${RUN_OUT}" | grep -q "size before:"; then
  ok "output contains 'size before:'"
else
  ng "output missing 'size before:'"
fi
if echo "${RUN_OUT}" | grep -q "after:"; then
  ok "output contains 'after:'"
else
  ng "output missing 'after:'"
fi
if echo "${RUN_OUT}" | grep -q "saved:"; then
  ok "output contains 'saved:'"
else
  ng "output missing 'saved:'"
fi

# ---- Summary --------------------------------------------------------------
echo ""
echo "============================================================"
echo "  TEST SUMMARY: ${PASS} PASS / ${FAIL} FAIL / ${SKIP} SKIP"
echo "============================================================"

if [[ "${FAIL}" -gt 0 ]]; then
  exit 1
fi
exit 0
