#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# prepare-embedded-kc-runtime.sh
#
# Task: task_027_dmg_packaging_kc (M, Week 6)
# 目的: 把 Knowledge Compiler (KC) 的 Python venv + 源码 (compiler/ + run_api.py)
#       注入到 .app/Contents/Resources/kc/，并向 runtime-manifest.json 写入
#       "kc" 字段 (version / commit_sha / python_version / venv_size_bytes)。
#
# 严格遵守:
#   - ADR-010: runtime-manifest.json 是单一事实源；本脚本在 markitdown 注入
#              之后追加 kc 字段，不动 markitdown / python 字段。
#   - PRD §6 / Architect §"ADR-010": 仅复制 compiler/ + run_api.py，跳过
#              notecapt/、gradio_demo.py、examples/、tests/（避免运行时多余
#              80MB+ 体积、避免 import 路径污染）。
#   - 红线   : 不引入 gradio / pandas / numpy / huggingface_hub 顶层；体积
#              控目标 ~150-180MB（剥离前 venv）。
#
# 用法:
#   ./scripts/prepare-embedded-kc-runtime.sh APP_PATH KC_REPO_PATH [PYTHON_BIN]
#   ./scripts/prepare-embedded-kc-runtime.sh --dry-run APP_PATH KC_REPO_PATH [PYTHON_BIN]
#   ./scripts/prepare-embedded-kc-runtime.sh -h
#
#   APP_PATH       e.g. src-tauri/target/release/bundle/macos/NoteCapt.app
#   KC_REPO_PATH   e.g. vendor/KnowledgeCompiler  (含 compiler/ + run_api.py)
#   PYTHON_BIN     e.g. python3.11  (default; KC langchain 系链需 ≥3.10)
#
# 幂等: 同一 APP_PATH + 同 KC commit 重跑会先 rm -rf 旧 kc/，重装等价。
# 跨平台: macOS / Linux 通用（不依赖 sed -i，避免 BSD vs GNU 分歧）。
#
# PM 真机验证: 见 task input.md AC-5/AC-6（du -sh ~150-180MB）。
# ---------------------------------------------------------------------------
set -euo pipefail

# ---- 常量 ------------------------------------------------------------------
readonly RUNTIME_ID="ncdesktop-kc-runtime"
readonly KC_RESOURCE_SUBPATH="Contents/Resources/kc"
readonly MANIFEST_SUBPATH="Contents/Resources/runtime-manifest.json"

# 复制 KC 源码白名单 — 仅 compiler/ 包 + run_api.py。
# 红线: notecapt / gradio_demo / examples / tests / docs 不入包。
readonly KC_SRC_INCLUDE=(
  "compiler"
  "run_api.py"
)

# ---- 路径 ------------------------------------------------------------------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly SCRIPT_DIR
readonly ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
readonly KC_REQUIREMENTS="${SCRIPT_DIR}/kc-requirements.txt"

# ---- 参数解析 --------------------------------------------------------------
DRY_RUN=0
POSITIONAL=()
for arg in "$@"; do
  case "${arg}" in
    --dry-run)
      DRY_RUN=1
      ;;
    -h|--help)
      sed -n '2,30p' "${BASH_SOURCE[0]}"
      exit 0
      ;;
    --*)
      echo "[prepare-kc-runtime] unknown flag: ${arg}" >&2
      exit 2
      ;;
    *)
      POSITIONAL+=("${arg}")
      ;;
  esac
done

if [[ ${#POSITIONAL[@]} -lt 2 ]]; then
  echo "[prepare-kc-runtime] ERROR: missing args" >&2
  echo "  usage: $0 [--dry-run] APP_PATH KC_REPO_PATH [PYTHON_BIN]" >&2
  exit 2
fi

APP_PATH="${POSITIONAL[0]}"
KC_REPO_PATH="${POSITIONAL[1]}"
PYTHON_BIN="${POSITIONAL[2]:-python3.11}"

# ---- 前置检查 --------------------------------------------------------------
log() { echo "[prepare-kc-runtime] $*"; }
fail() { echo "[prepare-kc-runtime] ERROR: $*" >&2; exit 1; }

# .app 路径必须存在 (tauri build 已产出)
if [[ ! -d "${APP_PATH}" ]]; then
  fail ".app bundle not found: ${APP_PATH}"
fi

# KC 仓库路径必须含 compiler/ 与 run_api.py
if [[ ! -d "${KC_REPO_PATH}" ]]; then
  fail "KC repo not found: ${KC_REPO_PATH}"
fi
for item in "${KC_SRC_INCLUDE[@]}"; do
  if [[ ! -e "${KC_REPO_PATH}/${item}" ]]; then
    fail "KC repo missing required source: ${KC_REPO_PATH}/${item}"
  fi
done

# python 可用性 (dry-run 也校验，避免真机才发现 python 缺失)
if ! command -v "${PYTHON_BIN}" >/dev/null 2>&1; then
  fail "python binary not in PATH: ${PYTHON_BIN}"
fi

# kc-requirements.txt 必须存在
if [[ ! -f "${KC_REQUIREMENTS}" ]]; then
  fail "kc-requirements.txt not found: ${KC_REQUIREMENTS}"
fi

# ---- 计算目标路径 ----------------------------------------------------------
KC_TARGET="${APP_PATH}/${KC_RESOURCE_SUBPATH}"
MANIFEST_PATH="${APP_PATH}/${MANIFEST_SUBPATH}"

# ---- dry-run: 输出 plan 不实际写盘 -----------------------------------------
if [[ "${DRY_RUN}" == "1" ]]; then
  log "DRY-RUN mode — no filesystem writes will occur"
  log "  app path:           ${APP_PATH}"
  log "  kc repo:            ${KC_REPO_PATH}"
  log "  python bin:         ${PYTHON_BIN}"
  log "  kc requirements:    ${KC_REQUIREMENTS}"
  log "  target install:     ${KC_TARGET}"
  log "  manifest:           ${MANIFEST_PATH}"
  log "  src include (only): ${KC_SRC_INCLUDE[*]}"
  log "DRY-RUN OK — exit 0"
  exit 0
fi

# ---- Step 1: Stage venv (tmp dir) ------------------------------------------
log "Step 1/6: creating staging venv with ${PYTHON_BIN}"
STAGE_PARENT="$(mktemp -d -t kc-staging.XXXXXX)"
STAGE_VENV="${STAGE_PARENT}/venv"
STAGE_SRC="${STAGE_PARENT}/src"

cleanup_stage() {
  # 失败时 stage_parent 不删，便于诊断；正常路径末尾 trap 删
  if [[ -n "${STAGE_PARENT:-}" && -d "${STAGE_PARENT}" ]]; then
    rm -rf "${STAGE_PARENT}"
  fi
}

"${PYTHON_BIN}" -m venv "${STAGE_VENV}"

# 升级 pip 一次（避免老 pip 解析 kc-requirements 失败）
"${STAGE_VENV}/bin/pip" install --no-cache-dir --upgrade pip --disable-pip-version-check

# ---- Step 2: pip install -r kc-requirements.txt ----------------------------
log "Step 2/6: pip install -r ${KC_REQUIREMENTS} into staging venv"
"${STAGE_VENV}/bin/pip" install \
  --no-cache-dir \
  --disable-pip-version-check \
  -r "${KC_REQUIREMENTS}"

# ---- Step 3: 拷贝 KC 源码（白名单） ----------------------------------------
log "Step 3/6: copying KC source (whitelist: ${KC_SRC_INCLUDE[*]})"
mkdir -p "${STAGE_SRC}"
for item in "${KC_SRC_INCLUDE[@]}"; do
  cp -R "${KC_REPO_PATH}/${item}" "${STAGE_SRC}/"
done

# ---- Step 4: 清理 __pycache__ / *.pyc ---------------------------------------
log "Step 4/6: stripping __pycache__ and *.pyc"
# `-exec rm -rf {} +` 在 macOS/Linux 通用；失败容忍（目录可能不存在）
find "${STAGE_PARENT}" -type d -name __pycache__ -exec rm -rf {} + 2>/dev/null || true
find "${STAGE_PARENT}" -type f -name "*.pyc" -delete 2>/dev/null || true

# ---- Step 5: 注入到 .app（先清空旧的，幂等） -------------------------------
log "Step 5/6: injecting into ${KC_TARGET}"
rm -rf "${KC_TARGET}"
mkdir -p "${KC_TARGET}"
# cp -R 保留符号链接 (venv/bin/python -> python3 这种)；不要 -L / -RL
cp -R "${STAGE_VENV}" "${KC_TARGET}/venv"
cp -R "${STAGE_SRC}/." "${KC_TARGET}/src/"

# ---- Step 6: 写 runtime-manifest.json ---------------------------------------
log "Step 6/6: updating runtime-manifest.json"

# KC 版本: 从 pyproject.toml 抓 version = "X.Y.Z"（容错；缺失则 unknown）
KC_PYPROJECT="${KC_REPO_PATH}/pyproject.toml"

# KC commit_sha: 必须 KC 仓库是 git checkout；非 git 则标 unknown
KC_COMMIT_SHA="unknown"
if [[ -d "${KC_REPO_PATH}/.git" ]] || (cd "${KC_REPO_PATH}" && git rev-parse --git-dir >/dev/null 2>&1); then
  KC_COMMIT_SHA="$(cd "${KC_REPO_PATH}" && git rev-parse HEAD 2>/dev/null || echo unknown)"
fi

# venv 大小 (bytes) — 用于 verify-dmg-contents 体积守护
VENV_SIZE_BYTES="$(du -sk "${KC_TARGET}/venv" 2>/dev/null | awk '{print $1 * 1024}')"
[[ -z "${VENV_SIZE_BYTES}" ]] && VENV_SIZE_BYTES=0

# python --version 输出形如 "Python 3.11.9"
PYTHON_VERSION="$("${PYTHON_BIN}" --version 2>&1 | head -1)"

# Build timestamp (UTC ISO-8601)
BUILD_TS="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

# 用 python3 修改/创建 manifest（zero-dep；不引 jq）
# 用 env 透传变量（避免 inline shell 引号嵌入 JSON 注入风险）
export PREP_KC_MANIFEST_PATH="${MANIFEST_PATH}"
export PREP_KC_PYPROJECT="${KC_PYPROJECT}"
export PREP_KC_COMMIT_SHA="${KC_COMMIT_SHA}"
export PREP_KC_PYTHON_VERSION="${PYTHON_VERSION}"
export PREP_KC_VENV_SIZE_BYTES="${VENV_SIZE_BYTES}"
export PREP_KC_BUILD_TS="${BUILD_TS}"
export PREP_KC_RUNTIME_ID="${RUNTIME_ID}"

"${PYTHON_BIN}" - <<'PYEOF'
import json
import os
import re
import sys

manifest_path = os.environ["PREP_KC_MANIFEST_PATH"]
pyproject = os.environ["PREP_KC_PYPROJECT"]

# 抓 KC version 从 pyproject.toml
version = "unknown"
if os.path.exists(pyproject):
    try:
        with open(pyproject, "r", encoding="utf-8") as fh:
            blob = fh.read()
        m = re.search(r'^version\s*=\s*"([^"]+)"', blob, re.MULTILINE)
        if m:
            version = m.group(1)
    except Exception as exc:  # pragma: no cover — defensive
        print(f"[prepare-kc-runtime] WARN: cannot parse pyproject: {exc}", file=sys.stderr)

# 读现有 manifest（若存在）以保留 markitdown/python 字段
manifest = {}
if os.path.exists(manifest_path):
    try:
        with open(manifest_path, "r", encoding="utf-8") as fh:
            manifest = json.load(fh)
    except Exception as exc:
        print(f"[prepare-kc-runtime] WARN: existing manifest invalid, recreating: {exc}", file=sys.stderr)
        manifest = {}

# 写 kc 字段（不动其他字段）
manifest["kc"] = {
    "runtime_id": os.environ["PREP_KC_RUNTIME_ID"],
    "version": version,
    "commit_sha": os.environ["PREP_KC_COMMIT_SHA"],
    "python_version": os.environ["PREP_KC_PYTHON_VERSION"],
    "venv_size_bytes": int(os.environ["PREP_KC_VENV_SIZE_BYTES"]),
    "build_timestamp": os.environ["PREP_KC_BUILD_TS"],
}

# 如果 schema_version 缺失（manifest 是空的），补一个
manifest.setdefault("schema_version", 1)

with open(manifest_path, "w", encoding="utf-8") as fh:
    json.dump(manifest, fh, indent=2, ensure_ascii=False)
    fh.write("\n")

print(f"[prepare-kc-runtime] manifest updated: {manifest_path}")
print(f"[prepare-kc-runtime]   kc.version       = {manifest['kc']['version']}")
print(f"[prepare-kc-runtime]   kc.commit_sha    = {manifest['kc']['commit_sha']}")
print(f"[prepare-kc-runtime]   kc.python_version= {manifest['kc']['python_version']}")
print(f"[prepare-kc-runtime]   kc.venv_size_bytes={manifest['kc']['venv_size_bytes']}")
PYEOF

# 清理 stage（成功路径）
cleanup_stage

# ---- 体积报告 (AC-6) -------------------------------------------------------
log "kc resource size (before optimize):"
du -sh "${KC_TARGET}"

# ---- 可选: 调 task_028 optimize-kc-venv.sh 进一步剥离 ----------------------
# task_028 配套 hook：PREP_KC_OPTIMIZE=1 时调 optimize-kc-venv.sh 剥离 venv
# 至 ~80MB。默认关闭 — PM 可在 build-macos-dmg.sh 中显式启用。
if [[ "${PREP_KC_OPTIMIZE:-0}" == "1" ]]; then
  OPTIMIZE_SCRIPT="${SCRIPT_DIR}/optimize-kc-venv.sh"
  if [[ -x "${OPTIMIZE_SCRIPT}" ]]; then
    log "PREP_KC_OPTIMIZE=1: invoking optimize-kc-venv.sh"
    if "${OPTIMIZE_SCRIPT}" "${KC_TARGET}/venv"; then
      log "kc resource size (after optimize):"
      du -sh "${KC_TARGET}"
    else
      log "WARN: optimize-kc-venv.sh failed; continuing with un-stripped venv"
    fi
  else
    log "WARN: PREP_KC_OPTIMIZE=1 but optimize script missing/non-executable: ${OPTIMIZE_SCRIPT}"
  fi
fi

log "Done. injected to ${KC_TARGET}"
