#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# optimize-kc-venv.sh
#
# Task: task_028_kc_venv_optimize (S, 1d)
# 目的: 在 task_027 prepare-embedded-kc-runtime.sh 注入的 kc-venv 基础上，
#       追加剥离 / 清理动作，把 kc-venv 从 ~150MB 压到 ~80MB（DMG 总增量
#       <100MB）。
#
# 剥离策略 (6 步):
#   1. 删除 dist-info/RECORD            (pip 安装记录，运行时不需要)
#   2. 删除 .pyi stubs                  (类型 stub，运行时不需要)
#   3. 删除 site-packages 内 tests/ + test/ 目录
#   4. 删除 *.dist-info/LICENSE* / AUTHORS / 大文档
#   5. 可选 strip 二进制 (macOS strip; 静态以 STRIP_BIN=1 控制)
#   6. 输出剥离前/后大小对比 + 超阈值告警
#
# 严守:
#   - 不动 jieba 词典文件（KC 中文分词必需；input.md §技术约束）
#   - 不剥运行时必需的 .so / .dylib
#   - 不强制 PYTHONOPTIMIZE=2（保留 docstrings 方便调试）
#   - 幂等: 二次运行不破坏（find -delete 失败容忍）
#
# 用法:
#   ./scripts/optimize-kc-venv.sh KC_VENV_PATH
#   ./scripts/optimize-kc-venv.sh --dry-run KC_VENV_PATH
#   ./scripts/optimize-kc-venv.sh --strip-bin KC_VENV_PATH    # 加 macOS strip
#   ./scripts/optimize-kc-venv.sh -h
#
#   KC_VENV_PATH   e.g. ./build/NoteCapt.app/Contents/Resources/kc/venv
#
# 退出码:
#   0  — 优化成功（即使最终体积仍 > 阈值也只是 WARN）
#   2  — 参数错误 / venv 路径不存在
#   1  — 其他 IO / find 失败
#
# 跨平台: macOS / Linux 通用（find -delete / -exec 用法两端兼容）。
#
# PM 真机验证: 见 task_028 output.md §"PM 真机验证清单"。
# ---------------------------------------------------------------------------
set -euo pipefail

# ---- 常量 ------------------------------------------------------------------
# 阈值（MB）— 超过则 WARN，不 fail（PM 真机若 over 可决定是否回滚）
readonly SIZE_WARN_MB=100
# 理想目标体积（MB）— 仅用于日志展示
readonly SIZE_TARGET_MB=80

# ---- 参数解析 --------------------------------------------------------------
DRY_RUN=0
STRIP_BIN=0
POSITIONAL=()
for arg in "$@"; do
  case "${arg}" in
    --dry-run)
      DRY_RUN=1
      ;;
    --strip-bin)
      STRIP_BIN=1
      ;;
    -h|--help)
      sed -n '2,40p' "${BASH_SOURCE[0]}"
      exit 0
      ;;
    --*)
      echo "[optimize-kc-venv] unknown flag: ${arg}" >&2
      exit 2
      ;;
    *)
      POSITIONAL+=("${arg}")
      ;;
  esac
done

if [[ ${#POSITIONAL[@]} -lt 1 ]]; then
  echo "[optimize-kc-venv] ERROR: missing KC_VENV_PATH" >&2
  echo "  usage: $0 [--dry-run] [--strip-bin] KC_VENV_PATH" >&2
  exit 2
fi

KC_VENV="${POSITIONAL[0]}"

# ---- 日志 ------------------------------------------------------------------
log() { echo "[optimize-kc-venv] $*"; }
fail() { echo "[optimize-kc-venv] ERROR: $*" >&2; exit 1; }

# ---- 前置检查 --------------------------------------------------------------
if [[ ! -d "${KC_VENV}" ]]; then
  echo "[optimize-kc-venv] ERROR: venv path not found: ${KC_VENV}" >&2
  exit 2
fi

# 基本结构 sanity check — venv 应含 bin/ + lib/
if [[ ! -d "${KC_VENV}/lib" ]]; then
  fail "venv missing lib/ (not a python venv?): ${KC_VENV}"
fi

# ---- 体积测量（剥离前）-----------------------------------------------------
size_mb() {
  # du -sm 在 macOS / Linux 上单位都是 MB（du -sk 也通用，这里直接 -sm）
  du -sm "$1" 2>/dev/null | awk '{print $1}'
}
size_bytes() {
  # du -sk * 1024 兜底（dry-run 报告也用得到）
  du -sk "$1" 2>/dev/null | awk '{print $1 * 1024}'
}

SIZE_BEFORE_MB="$(size_mb "${KC_VENV}")"
SIZE_BEFORE_BYTES="$(size_bytes "${KC_VENV}")"
log "venv path: ${KC_VENV}"
log "size before: ${SIZE_BEFORE_MB}MB (${SIZE_BEFORE_BYTES} bytes)"

# ---- DRY-RUN: 只数文件、不删 -----------------------------------------------
if [[ "${DRY_RUN}" == "1" ]]; then
  log "DRY-RUN mode — no filesystem deletes will occur"
  RECORD_CNT="$(find "${KC_VENV}" -name RECORD -path "*.dist-info/*" 2>/dev/null | wc -l | tr -d ' ')"
  PYI_CNT="$(find "${KC_VENV}" -name "*.pyi" 2>/dev/null | wc -l | tr -d ' ')"
  TESTS_CNT="$(find "${KC_VENV}/lib" -type d \( -name tests -o -name test \) 2>/dev/null | wc -l | tr -d ' ')"
  LICENSE_CNT="$(find "${KC_VENV}" -path "*.dist-info/*" \( -iname "license*" -o -iname "AUTHORS" -o -iname "NOTICE*" \) 2>/dev/null | wc -l | tr -d ' ')"
  log "  would delete RECORD files:   ${RECORD_CNT}"
  log "  would delete .pyi stubs:     ${PYI_CNT}"
  log "  would delete tests/ dirs:    ${TESTS_CNT}"
  log "  would delete LICENSE/etc:    ${LICENSE_CNT}"
  log "DRY-RUN OK — exit 0"
  exit 0
fi

# ---------------------------------------------------------------------------
# 真正剥离动作（6 步）
# ---------------------------------------------------------------------------

# ---- Step 1: 删 dist-info/RECORD --------------------------------------------
log "Step 1/6: removing dist-info/RECORD"
# `-delete` 在 macOS/Linux 通用；find 失败容忍（venv 内可能没 dist-info）
find "${KC_VENV}" -name RECORD -path "*.dist-info/*" -delete 2>/dev/null || true

# ---- Step 2: 删 .pyi stubs --------------------------------------------------
log "Step 2/6: removing .pyi stubs"
find "${KC_VENV}" -type f -name "*.pyi" -delete 2>/dev/null || true

# ---- Step 3: 删 site-packages 内 tests/ + test/ ----------------------------
# 仅在 lib/ 下扫，避免误删 bin/ 内文件
log "Step 3/6: removing site-packages tests/ and test/ directories"
# 红线: 不删 jieba 的 dict/ + idf.txt（KC 中文分词必需）
# jieba 子目录形如 lib/python3.x/site-packages/jieba/...，不含 tests/ test/，
# 但额外加白名单守护（防御性编程）
find "${KC_VENV}/lib" -type d \( -name tests -o -name test \) \
  ! -path "*/jieba/*" \
  -exec rm -rf {} + 2>/dev/null || true

# ---- Step 4: 删 dist-info LICENSE / AUTHORS / 大文档 -----------------------
log "Step 4/6: removing dist-info LICENSE/AUTHORS/NOTICE"
# 只删 dist-info 目录内的（包根目录的不动，保留 attribution）
find "${KC_VENV}" -type d -name "*.dist-info" -print0 2>/dev/null | \
  while IFS= read -r -d '' di; do
    rm -f "${di}"/LICENSE* "${di}"/license* "${di}"/AUTHORS \
          "${di}"/NOTICE* "${di}"/COPYING* "${di}"/INSTALLER 2>/dev/null || true
  done

# ---- Step 5: 可选 strip 二进制（macOS strip） ------------------------------
if [[ "${STRIP_BIN}" == "1" ]]; then
  log "Step 5/6: strip .so / .dylib (--strip-bin requested)"
  if ! command -v strip >/dev/null 2>&1; then
    log "  WARN: 'strip' not in PATH; skipping binary strip"
  else
    # macOS strip 单文件、保留符号表：-x 删 non-global，-S 删 debug
    # 不用 -u（unstable），-S 是最安全的
    STRIP_OK=0
    STRIP_FAIL=0
    while IFS= read -r -d '' bin; do
      if strip -S "${bin}" >/dev/null 2>&1; then
        STRIP_OK=$((STRIP_OK + 1))
      else
        STRIP_FAIL=$((STRIP_FAIL + 1))
      fi
    done < <(find "${KC_VENV}" -type f \( -name "*.so" -o -name "*.dylib" \) -print0 2>/dev/null)
    log "  stripped: ${STRIP_OK}    failed: ${STRIP_FAIL}"
  fi
else
  log "Step 5/6: skip binary strip (use --strip-bin to enable)"
fi

# ---- Step 6: 体积报告 + 阈值告警 -------------------------------------------
SIZE_AFTER_MB="$(size_mb "${KC_VENV}")"
SIZE_AFTER_BYTES="$(size_bytes "${KC_VENV}")"
# 计算剥离量（MB），用 awk 避免 bash 算整数除法误差
SAVED_MB="$(awk -v b="${SIZE_BEFORE_MB}" -v a="${SIZE_AFTER_MB}" 'BEGIN{print b - a}')"
SAVED_PCT="$(awk -v b="${SIZE_BEFORE_MB}" -v a="${SIZE_AFTER_MB}" \
  'BEGIN{ if(b==0){print "0.0"} else {printf "%.1f", (b-a)*100.0/b} }')"

log "Step 6/6: size report"
log "  before:  ${SIZE_BEFORE_MB}MB"
log "  after:   ${SIZE_AFTER_MB}MB"
log "  saved:   ${SAVED_MB}MB (${SAVED_PCT}%)"
log "  target:  ~${SIZE_TARGET_MB}MB    warn-threshold: ${SIZE_WARN_MB}MB"

if [[ "${SIZE_AFTER_MB}" -gt "${SIZE_WARN_MB}" ]]; then
  log "WARN: kc-venv > ${SIZE_WARN_MB}MB (got ${SIZE_AFTER_MB}MB)"
fi

log "Done. optimized ${KC_VENV}"
