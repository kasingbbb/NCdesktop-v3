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
#   PYTHON_BIN     e.g. python3.12  (default; KC pyproject 要求 >=3.12)
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

# 复制 KC 源码白名单 — compiler/ + notecapt/ + run_api.py。
# 红线: gradio_demo / examples / tests / docs 不入包（demo / 测试 / 文档）。
#
# 历史更正：task_001 Architect 调研误把 notecapt/（KC 仓库内的同名 1.2MB 包，
# 含 api/core/pipelines/，被 compiler/interfaces/main.py 强依赖）当作 NC-specific
# demo 排除，导致 KC 启动 ModuleNotFoundError: No module named 'notecapt'。
# 2026-05-28 真机打包发现并纠偏，notecapt 入白名单。
readonly KC_SRC_INCLUDE=(
  "compiler"
  "notecapt"
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
PYTHON_BIN="${POSITIONAL[2]:-python3.12}"

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

# python --version 输出形如 "Python 3.12.x"
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

# ---- KC-MOD-5 patch: run_api.py 支持 --host / --port argv ------------------
# 2026-05-28 真机打包发现 KC run_api.py 原版用 settings.HOST/PORT 忽略 argv，
# NC 的 task_008 KcProcessManager 传 `--host --port` 不生效 → 健康检查 timeout。
# 本 patch 让 run_api.py 解析 argv + 优先 argv > settings。待 KC 仓库正式实装
# KC-MOD-5 后此 patch 自动失效（python -c 检测已含 argparse 则跳过）。
# dry-run 模式跳过（mock fixture 没真的 src/run_api.py）。
export RUN_API_PATH="${KC_TARGET}/src/run_api.py"
if [[ "${DRY_RUN}" != "1" ]] && [[ -f "${RUN_API_PATH}" ]]; then
  if ! grep -q "argparse" "${RUN_API_PATH}"; then
    log "applying KC-MOD-5 patch: run_api.py argv support"
    "${PYTHON_BIN}" - <<'PYEOF'
import os, sys
path = os.environ["RUN_API_PATH"]
new_content = '''#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""
FastAPI 入口脚本 - 启动 Knowledge Compiler API 服务

NC patch (KC-MOD-5, 2026-05-28 真机打包加入):
  NC 通过 `run_api.py --host 127.0.0.1 --port <dyn>` 传端口；原版用
  settings.HOST/PORT 忽略 argv → NC health check timeout。
  本 patch 添加 argparse 让 --host / --port 覆盖 settings 值。
  KC 仓库正式实装 KC-MOD-5 后此 patch 应回流并移除。
"""
import argparse
import sys
import os

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from compiler.interfaces.main import app
from compiler.infrastructure.config import get_settings
import uvicorn


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Knowledge Compiler API")
    parser.add_argument("--host", default=None, help="Override settings.HOST")
    parser.add_argument("--port", type=int, default=None, help="Override settings.PORT")
    args, _unknown = parser.parse_known_args()
    return args


def main():
    settings = get_settings()
    args = parse_args()
    host = args.host if args.host else settings.HOST
    port = args.port if args.port else settings.PORT

    print("=" * 60)
    print("Knowledge Compiler API")
    print(f"   Version: {settings.API_VERSION}")
    print(f"   Host: {host}:{port}")
    print(f"   AI Features: {'Enabled' if settings.ENABLE_AI_FEATURES else 'Disabled'}")
    print("=" * 60)

    uvicorn.run(
        app,
        host=host,
        port=port,
        reload=settings.DEBUG,
        log_level=settings.LOG_LEVEL.lower(),
    )


if __name__ == "__main__":
    main()
'''
with open(path, "w", encoding="utf-8") as fh:
    fh.write(new_content)
print(f"[prepare-kc-runtime] KC-MOD-5 patch applied: {path}")
PYEOF
  else
    log "KC-MOD-5 already present (skipping patch)"
  fi
fi
export -n RUN_API_PATH

# ---- KC-MOD-1 patch: ingest 响应加 enhanced_markdown 字段 -------------------
# 2026-05-28 真机打包发现 KC `/api/v1/ingest` 原版只返回 enhanced_path（磁盘
# 路径），不返回 enhanced_markdown（字符串）；NC client.rs (task_007) 期望
# inline string → 走 KcCallError::Malformed 路径 → enrich fallback markitdown
# 原版 MD（无 KC chunk 标签）。
#
# 本 patch 改 KC 一侧 2 文件：
# 1) compiler/interfaces/models.py: IngestResponse 加 enhanced_markdown 字段
# 2) compiler/interfaces/routes.py: ingest endpoint 读 enhanced_path 文件回填
#
# 待 KC 仓库正式实装 KC-MOD-1 后此 patch 应回流并移除。幂等：检测已含字段则跳过。
export KC_MODELS_PATH="${KC_TARGET}/src/compiler/interfaces/models.py"
export KC_ROUTES_PATH="${KC_TARGET}/src/compiler/interfaces/routes.py"
if [[ "${DRY_RUN}" != "1" ]] && [[ -f "${KC_MODELS_PATH}" ]] && [[ -f "${KC_ROUTES_PATH}" ]]; then
  if ! grep -q "enhanced_markdown" "${KC_MODELS_PATH}"; then
    log "applying KC-MOD-1 patch: models.py + routes.py enhanced_markdown field"
    "${PYTHON_BIN}" - <<'PYEOF'
import os
mp = os.environ["KC_MODELS_PATH"]
rp = os.environ["KC_ROUTES_PATH"]

# models.py: 在 enhanced_path 行后插入 enhanced_markdown 字段
with open(mp, "r", encoding="utf-8") as fh:
    m_content = fh.read()
old_line = '    enhanced_path: str = Field(..., description="增强 Markdown 路径")'
new_block = (
    '    enhanced_path: str = Field(..., description="增强 Markdown 路径")\n'
    '    enhanced_markdown: Optional[str] = Field(None, description="增强 Markdown 字符串（NC patch: KC-MOD-1，2026-05-28 真机加）")'
)
m_content = m_content.replace(old_line, new_block, 1)
with open(mp, "w", encoding="utf-8") as fh:
    fh.write(m_content)

# routes.py: 在 IngestResponse(success=True,...) 之前读 enhanced_path 文件
with open(rp, "r", encoding="utf-8") as fh:
    r_content = fh.read()
old_return = (
    '        return IngestResponse(\n'
    '            success=True,\n'
    '            doc_id=result.get("doc_id", ""),\n'
    '            title=result.get("title", ""),\n'
    '            enhanced_path=result.get("enhanced_path", ""),\n'
    '            index_path=result.get("index_path", ""),'
)
new_return = (
    '        # NC patch (KC-MOD-1, 2026-05-28 真机加)：读 enhanced_path 文件\n'
    '        # 内容回填 enhanced_markdown 字段，让 NC client 直接消费 inline string\n'
    '        enhanced_path = result.get("enhanced_path", "")\n'
    '        enhanced_markdown = None\n'
    '        if enhanced_path:\n'
    '            try:\n'
    '                from pathlib import Path\n'
    '                p = Path(enhanced_path)\n'
    '                if p.is_file():\n'
    '                    enhanced_markdown = p.read_text(encoding="utf-8")\n'
    '            except Exception as _e:\n'
    '                enhanced_markdown = None\n'
    '\n'
    '        return IngestResponse(\n'
    '            success=True,\n'
    '            doc_id=result.get("doc_id", ""),\n'
    '            title=result.get("title", ""),\n'
    '            enhanced_path=enhanced_path,\n'
    '            enhanced_markdown=enhanced_markdown,\n'
    '            index_path=result.get("index_path", ""),'
)
r_content = r_content.replace(old_return, new_return, 1)
with open(rp, "w", encoding="utf-8") as fh:
    fh.write(r_content)

print(f"[prepare-kc-runtime] KC-MOD-1 patch applied: {mp}, {rp}")
PYEOF
  else
    log "KC-MOD-1 already present (skipping patch)"
  fi
fi
export -n KC_MODELS_PATH KC_ROUTES_PATH

# ---- macOS ad-hoc codesign（防 Apple Silicon hardened runtime 杀 .so/.dylib）-
# 2026-05-28 真机打包发现 macOS 14+ 对未签名 dylib 在 hardened runtime 下
# 直接 SIGKILL（例：faiss-cpu / scipy 的 native .so）。修复：用 ad-hoc 签名
# （-s -）给 venv 内所有 .so / .dylib 加签。仅 macOS 跑；Linux 跳过；
# dry-run 跳过（mock fixture 没真 venv）。
if [[ "${DRY_RUN}" != "1" ]] && [[ "$(uname -s)" == "Darwin" ]]; then
  if command -v codesign >/dev/null 2>&1; then
    log "applying macOS ad-hoc codesign to all .so/.dylib"
    SIGN_COUNT=0
    SIGN_FAIL=0
    while IFS= read -r -d '' sf; do
      if codesign --force -s - --options runtime --timestamp=none "${sf}" >/dev/null 2>&1; then
        SIGN_COUNT=$((SIGN_COUNT + 1))
      else
        SIGN_FAIL=$((SIGN_FAIL + 1))
      fi
    done < <(find "${KC_TARGET}/venv" -type f \( -name "*.so" -o -name "*.dylib" \) -print0 2>/dev/null)
    log "  ad-hoc signed: ${SIGN_COUNT}    failed: ${SIGN_FAIL}"
  else
    log "WARN: codesign not found; native .so/.dylib may be SIGKILL'd by hardened runtime"
  fi
fi

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
