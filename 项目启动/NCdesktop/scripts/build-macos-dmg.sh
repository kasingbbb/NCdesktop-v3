#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# build-macos-dmg.sh — One-shot macOS DMG build orchestrator (task_006 / T-F)
#
# Integrates T-A..T-E into a single, fail-fast pipeline that produces a
# distributable, notarized DMG on macOS arm64. Each sub-script is the
# authoritative owner of its phase; THIS script only schedules + gates.
#
# Pipeline (AC-1, strict order — any failure aborts via `set -euo pipefail`):
#   1) scripts/prepare-embedded-python.sh            (task_001 / T-A)
#   2) scripts/prepare-embedded-markitdown-runtime.sh(task_002 / T-B)
#   3) scripts/prepare-venv-shim.sh                  (task_003 / T-C)
#   4) pnpm tauri build --bundles app                (Rust + frontend bundle)
#   5) scripts/sign-bundle.sh "$APP"                 (task_004 / T-D, reverse-order)
#   6) hdiutil create -srcfolder ... <dmg>           (DMG, symlinks preserved)
#   7) codesign … "$DMG"                              (hardened runtime DMG sign)
#   8) scripts/notarize.sh "$DMG"                    (task_005 / T-E, includes staple)
#   9) du -sh + symlink self-check (mount DMG)       (AC-2 + AC-3 gates)
#  10) shasum -a 256 "$DMG" → dist/<version>.sha256  (AC-6 release artifact)
#
# Gates (input.md acceptance criteria):
#   AC-2  ≤300MB size gate. We compute size in KB (du -sk → unit-free integer)
#         to avoid the "289M vs 1.2G" string-comparison trap.
#   AC-3  Mount DMG read-only, assert markitdown-venv/bin/python is a symlink
#         whose target is a *relative* path. Failure exits non-zero.
#   AC-4  set -euo pipefail + trap '...' ERR EXIT for failed-step diagnosis
#         and mount-point cleanup. --release / --debug flag honored.
#   AC-6  SHA256 → dist/<version>.sha256 (GNU `shasum -c`-compatible format).
#
# Red lines (input.md §技术约束):
#   - NEVER `cp -RL` / `cp -L` on directories (would dereference symlinks and
#     break ADR-003). Use `hdiutil create -srcfolder` which preserves symlinks.
#   - NEVER skip any of T-A..T-E. Each sub-script must be invoked even if its
#     idempotent guard would no-op; idempotency lives inside the sub-scripts.
# ─────────────────────────────────────────────────────────────────────────────

set -euo pipefail

# ── Paths & config ──────────────────────────────────────────────────────────
ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
APP_NAME="${APP_NAME:-NoteCapt}"

# --release / --debug flag (AC-4)
PROFILE="release"
for arg in "$@"; do
  case "$arg" in
    --release) PROFILE="release" ;;
    --debug)   PROFILE="debug"   ;;
    -h|--help)
      cat <<'EOF'
Usage: build-macos-dmg.sh [--release|--debug]

  --release   Build with Tauri release profile (default)
  --debug     Build with Tauri debug profile

Required env for distribution build:
  CODESIGN_IDENTITY     — Developer ID Application identity
  NOTARY_KEY_ID         — App Store Connect API key id
  NOTARY_ISSUER_ID      — App Store Connect issuer UUID
  NOTARY_KEY_P8_PATH    — Absolute path to AuthKey_XXXX.p8

If any of the above is missing, the corresponding step is SKIPPED with a
loud warning. Local dev builds (no signing/notarization) still produce a
DMG, but it will trigger Gatekeeper prompts on user machines.
EOF
      exit 0
      ;;
  esac
done

APP_BUNDLE_PATH="${ROOT_DIR}/src-tauri/target/${PROFILE}/bundle/macos/${APP_NAME}.app"
RESOURCES_DIR="${APP_BUNDLE_PATH}/Contents/Resources"
DMG_DIR="${ROOT_DIR}/src-tauri/target/${PROFILE}/bundle/dmg"
DMG_PATH="${DMG_DIR}/${APP_NAME}-embedded-runtime.dmg"
DIST_DIR="${ROOT_DIR}/dist"

# Tauri config — version is read here for the SHA256 filename (AC-6).
TAURI_CONF="${ROOT_DIR}/src-tauri/tauri.conf.json"

# Toggle for self-test mode — when SELFTEST=1 we expose the gate functions
# but skip the heavy pipeline. Used by the static / mock test matrix.
SELFTEST="${SELFTEST:-0}"

# Size gate threshold (KB). 300 MB == 300 * 1024 KB == 307_200 KB.
# 2026-05-31：用户拍板"放开 300M 限制"（DMG 主要本机用 + torch 走 bundle 外 kc_extras，
# 不进包）。默认门放宽到 512000KB(500M)；仍可用 SIZE_LIMIT_KB 环境变量覆盖。
SIZE_LIMIT_KB="${SIZE_LIMIT_KB:-512000}"

# ── Trap for failed-step diagnosis + mount cleanup (AC-4) ───────────────────
CURRENT_STEP="<init>"
MOUNT_POINT=""

cleanup() {
  local rc=$?
  if [[ -n "${MOUNT_POINT}" && -d "${MOUNT_POINT}" ]]; then
    # Best-effort detach; ignore failure (already unmounted by caller, etc.)
    hdiutil detach "${MOUNT_POINT}" -quiet 2>/dev/null || true
    rmdir "${MOUNT_POINT}" 2>/dev/null || true
  fi
  if (( rc != 0 )); then
    echo "" >&2
    echo "════════════════════════════════════════════════════════════" >&2
    echo "[build-macos-dmg] FAILED at step: ${CURRENT_STEP}" >&2
    echo "[build-macos-dmg] exit code: ${rc}" >&2
    echo "════════════════════════════════════════════════════════════" >&2
  fi
  return $rc
}
trap cleanup EXIT

step() {
  CURRENT_STEP="$1"
  echo ""
  echo "===[${CURRENT_STEP}]==="
}

# ── Pure functions (extracted for self-test) ────────────────────────────────

# AC-2: size gate. Argument is size in KB (integer). Returns non-zero if over.
# Exposed as a function so the mock matrix can drive it directly.
check_size_gate() {
  local size_kb="$1"
  local limit_kb="${2:-${SIZE_LIMIT_KB}}"
  if (( size_kb > limit_kb )); then
    local size_mb=$(( size_kb / 1024 ))
    local limit_mb=$(( limit_kb / 1024 ))
    echo "[build-macos-dmg] SIZE GATE FAIL: DMG is ${size_mb}M (${size_kb} KB), exceeds ${limit_mb}M limit (${limit_kb} KB)" >&2
    return 1
  fi
  return 0
}

# AC-6: read version from tauri.conf.json. Uses python3 (always present on
# macOS) instead of jq (not guaranteed) — gives identical semantics with
# zero new tool dependency.
read_tauri_version() {
  local conf="$1"
  python3 -c "
import json, sys
with open('${conf}', 'r') as f:
    print(json.load(f).get('version', ''))
"
}

# AC-3: verify a single symlink path is relative (not anchored at '/').
# Returns 0 if relative, non-zero otherwise. Argument is the readlink output.
is_relative_symlink_target() {
  local target="$1"
  if [[ -z "${target}" ]]; then
    return 1
  fi
  case "${target}" in
    /*) return 1 ;;
    *)  return 0 ;;
  esac
}

# ── Self-test short-circuit ─────────────────────────────────────────────────
# Allow `SELFTEST=1 bash build-macos-dmg.sh` to source the functions without
# triggering the full pipeline. Used by the static + mock test matrix.
if [[ "${SELFTEST}" == "1" ]]; then
  echo "[build-macos-dmg] SELFTEST=1 — functions loaded, pipeline skipped"
  return 0 2>/dev/null || exit 0
fi

cd "${ROOT_DIR}"

# ── [step 1/10] prepare-embedded-python.sh ──────────────────────────────────
step "step 1/10 prepare-embedded-python"
bash "${ROOT_DIR}/scripts/prepare-embedded-python.sh"

# ── [step 2/10] prepare-embedded-markitdown-runtime.sh ──────────────────────
step "step 2/10 prepare-embedded-markitdown-runtime"
bash "${ROOT_DIR}/scripts/prepare-embedded-markitdown-runtime.sh"

# ── [step 3/10] prepare-venv-shim.sh ────────────────────────────────────────
step "step 3/10 prepare-venv-shim"
bash "${ROOT_DIR}/scripts/prepare-venv-shim.sh"

# ── [step 3b/10] prepare-embedded-ffmpeg.sh ─────────────────────────────────
# 下载静态 arm64 ffmpeg 到 src-tauri/resources/ffmpeg，供视频导入抽音频内置回退
# （用户机无 Homebrew ffmpeg 时仍可用）。幂等：已存在且 sha256 通过则跳过。
step "step 3b/10 prepare-embedded-ffmpeg"
bash "${ROOT_DIR}/scripts/prepare-embedded-ffmpeg.sh"

# ── [step 4/10] tauri build ─────────────────────────────────────────────────
step "step 4/10 tauri build"
# Frontend bundle must exist before Tauri's beforeBuildCommand picks it up,
# but tauri.conf.json's beforeBuildCommand already wires `pnpm build`.
# `--bundles app` is used because the DMG is hand-rolled by this script
# (we need the staging dir to inject the embedded runtime).
if [[ "${PROFILE}" == "debug" ]]; then
  pnpm tauri build --bundles app --debug
else
  pnpm tauri build --bundles app
fi

if [[ ! -d "${APP_BUNDLE_PATH}" ]]; then
  echo "[build-macos-dmg] app bundle not found after tauri build: ${APP_BUNDLE_PATH}" >&2
  exit 1
fi

# Inject the runtime into the .app bundle (preserves symlinks: `cp -R`
# without -L. AC-3 red line: NEVER `cp -RL` / `cp -L` on directories).
step "step 4b/10 inject runtime into .app"
mkdir -p "${RESOURCES_DIR}"
rm -rf "${RESOURCES_DIR}/python" "${RESOURCES_DIR}/markitdown-venv" "${RESOURCES_DIR}/runtime-manifest.json" "${RESOURCES_DIR}/ffmpeg"
# hotfix 2026-05-26: python 源路径与 manifest 源路径必须一致 ——
# `prepare-embedded-python.sh` 写到 `src-tauri/resources/python`（task_001 落点），
# `prepare-embedded-markitdown-runtime.sh` pip install 到同目录的 site-packages，
# `prepare-embedded-markitdown-runtime.sh:144` 写 `src-tauri/resources/runtime-manifest.json`。
# 历史 line 200 写的 `build/runtime/python` 是 2026-05-14 hotfix 漏改的残留路径
# （manifest 路径修了、python 路径没修），导致 step 4b 必失败：cp: No such file。
cp -R "${ROOT_DIR}/src-tauri/resources/python"           "${RESOURCES_DIR}/python"
cp    "${ROOT_DIR}/src-tauri/resources/runtime-manifest.json" "${RESOURCES_DIR}/runtime-manifest.json"
# Recreate the venv-shim with *intra-bundle relative* symlinks (the build/
# runtime shim points at absolute build paths and would break once relocated).
mkdir -p "${RESOURCES_DIR}/markitdown-venv/bin"
ln -sf "../../python/bin/python3"    "${RESOURCES_DIR}/markitdown-venv/bin/python3"
ln -sf "../../python/bin/python3"    "${RESOURCES_DIR}/markitdown-venv/bin/python"
ln -sf "../../python/bin/markitdown" "${RESOURCES_DIR}/markitdown-venv/bin/markitdown" 2>/dev/null || true
ln -sf "../python/lib"               "${RESOURCES_DIR}/markitdown-venv/lib"

# 内置静态 ffmpeg → Resources/ffmpeg（视频导入抽音频的自包含回退；
# extraction::video_audio::locate_ffmpeg 优先探 ../Resources/ffmpeg）。
cp "${ROOT_DIR}/src-tauri/resources/ffmpeg" "${RESOURCES_DIR}/ffmpeg"
chmod +x "${RESOURCES_DIR}/ffmpeg"

# ── [step 4c/10] inject KC runtime + optimize kc-venv ───────────────────────
# KC (Knowledge Compiler) runtime 注入到 .app/Contents/Resources/kc/。
# 需 KC_REPO_PATH 环境变量指向 KnowledgeCompiler 仓库（含 compiler/ + run_api.py）。
# 优雅降级：KC_REPO_PATH 未配置或无效时只 WARN + skip，不让整个 build 失败
# （这样没有 KC repo 的人也能打出无 KC 功能的包）。
# 顺序：必须在 sign-bundle.sh（step 5）之前，让 KC venv 一并进入反序签名覆盖。
step "step 4c/10 inject KC runtime + optimize kc-venv"
if [[ -n "${KC_REPO_PATH:-}" ]] && [[ -d "${KC_REPO_PATH}/compiler" ]] && [[ -f "${KC_REPO_PATH}/run_api.py" ]]; then
  # SIGBUS 修复（2026-05-31）：不再传 python3.12（pyenv）作第 3 参；省略后
  # prepare-embedded-kc-runtime.sh 默认用 bundle 内自包含 pbs python
  # (src-tauri/resources/python/bin/python3.12) 建 KC venv，使解释器/stdlib/.so
  # 全在签名 bundle 内，消除代码签名分页校验 SIGBUS。
  bash "${ROOT_DIR}/scripts/prepare-embedded-kc-runtime.sh" \
    "${APP_BUNDLE_PATH}" "${KC_REPO_PATH}"
  bash "${ROOT_DIR}/scripts/optimize-kc-venv.sh" \
    --strip-bin "${RESOURCES_DIR}/kc/venv"

  # 2026-05-31 修复（真机 BLOCKER）：optimize 的 `--strip-bin` 会改写 Mach-O，
  # 使 numpy/scipy/faiss/hdbscan 等原生扩展的代码签名失效（codesign -v 报
  # "code or signature have been modified"）→ 运行时 macOS AMFI 直接 SIGKILL(137)，
  # KC sidecar 一 import 这些库就崩、enrich 静默回退、用户看不到 KC 效果。
  # 修：strip 之后对 KC venv 内所有 .so/.dylib 做 ad-hoc 重签，恢复可加载的有效签名。
  # （若设了 CODESIGN_IDENTITY，step 5 的 sign-bundle 会再做正式深签覆盖，本步无害。）
  KC_SIGN_OK=0
  KC_SIGN_FAIL=0
  while IFS= read -r -d '' lib; do
    if codesign --force -s - --timestamp=none "${lib}" >/dev/null 2>&1; then
      KC_SIGN_OK=$((KC_SIGN_OK + 1))
    else
      KC_SIGN_FAIL=$((KC_SIGN_FAIL + 1))
    fi
  done < <(find "${RESOURCES_DIR}/kc/venv" \( -name '*.so' -o -name '*.dylib' \) -print0)
  echo "[build-macos-dmg] KC venv ad-hoc 重签 .so/.dylib: ok=${KC_SIGN_OK} fail=${KC_SIGN_FAIL}"
else
  echo "[build-macos-dmg] WARN: KC_REPO_PATH 未配置或无效，跳过 KC 注入（产出的 DMG 不含 KC 功能）"
fi

# ── [step 5/10] sign-bundle.sh (task_004 reverse-order signing) ─────────────
step "step 5/10 sign-bundle"
SIGN_IDENTITY="${CODESIGN_IDENTITY:-${APPLE_SIGN_IDENTITY:-}}"
if [[ -n "${SIGN_IDENTITY}" ]]; then
  CODESIGN_IDENTITY="${SIGN_IDENTITY}" \
    bash "${ROOT_DIR}/scripts/sign-bundle.sh" "${APP_BUNDLE_PATH}"
else
  echo "[build-macos-dmg] WARN: CODESIGN_IDENTITY unset — skipping sign-bundle.sh"
  echo "[build-macos-dmg]       (Distribution builds REQUIRE signing; see ADR-004.)"
fi

# ── [step 6/10] hdiutil create — DMG (preserves symlinks) ──────────────────
step "step 6/10 hdiutil create dmg"
mkdir -p "${DMG_DIR}"
STAGING_DIR="$(mktemp -d -t notecapt-staging.XXXXXX)"

# `cp -R` preserves symlinks (does NOT dereference). NEVER use -L / -RL here.
cp -R "${APP_BUNDLE_PATH}" "${STAGING_DIR}/${APP_NAME}.app"
ln -s /Applications "${STAGING_DIR}/Applications"

cat > "${STAGING_DIR}/首次安装说明.txt" <<'NOTEBODY'
安装说明 (NoteCapt 内测版)
──────────────────────────────

1. 把 NoteCapt.app 拖入右侧 Applications 文件夹

2. 首次打开时若 macOS 提示"无法验证开发者"：
   方法一（推荐）：在 Finder 中右键点击 NoteCapt.app → 打开 → 再点"打开"
   方法二（一次性解除）：在终端执行
       xattr -cr /Applications/NoteCapt.app

3. 文件转换功能（PDF / DOCX / PPTX / XLSX / HTML / EPUB → Markdown）已内置，
   无需安装 Python 或任何额外依赖。
NOTEBODY

rm -f "${DMG_PATH}"
# hdiutil create with -srcfolder preserves symlinks bit-for-bit.
# UDZO = compressed read-only (standard for distribution DMGs).
hdiutil create \
  -volname "${APP_NAME}" \
  -srcfolder "${STAGING_DIR}" \
  -ov \
  -format UDZO \
  "${DMG_PATH}"
rm -rf "${STAGING_DIR}"

# ── [step 7/10] codesign the DMG itself ─────────────────────────────────────
step "step 7/10 codesign dmg"
if [[ -n "${SIGN_IDENTITY}" ]]; then
  # The DMG container is signed *after* hdiutil create so the signature
  # covers the final compressed bits. Same hardened-runtime + entitlements
  # as the .app inside (ADR-004 + ADR-005).
  codesign \
    --force \
    --options runtime \
    --timestamp \
    --entitlements "${ROOT_DIR}/scripts/entitlements.plist" \
    -s "${SIGN_IDENTITY}" \
    -- "${DMG_PATH}"
  codesign --verify --strict --verbose=2 "${DMG_PATH}"
else
  echo "[build-macos-dmg] WARN: CODESIGN_IDENTITY unset — skipping DMG codesign"
fi

# ── [step 8/10] notarize.sh (includes stapler internally) ───────────────────
step "step 8/10 notarize"
if [[ -n "${NOTARY_KEY_ID:-}" && -n "${NOTARY_ISSUER_ID:-}" && -n "${NOTARY_KEY_P8_PATH:-}" ]]; then
  bash "${ROOT_DIR}/scripts/notarize.sh" "${DMG_PATH}"
else
  echo "[build-macos-dmg] WARN: NOTARY_KEY_ID / NOTARY_ISSUER_ID / NOTARY_KEY_P8_PATH not all set — skipping notarization"
  echo "[build-macos-dmg]       (Distribution builds MUST notarize; see ADR-005.)"
fi

# ── [step 9/10] stapler staple (idempotent — notarize.sh also staples) ──────
step "step 9/10 stapler staple (idempotent re-check)"
# notarize.sh already runs `xcrun stapler staple` and asserts the success
# literal. We re-run stapler here ONLY when notarize.sh was skipped, so an
# already-stapled DMG isn't re-stapled unnecessarily. `stapler staple` on a
# fully-stapled DMG returns success (the ticket is re-validated, not re-bound)
# so a second call would be safe — we skip it purely to keep logs clean.
if [[ -z "${NOTARY_KEY_ID:-}" || -z "${NOTARY_ISSUER_ID:-}" || -z "${NOTARY_KEY_P8_PATH:-}" ]]; then
  echo "[build-macos-dmg] (notarization was skipped → nothing to staple)"
else
  echo "[build-macos-dmg] (staple already executed inside notarize.sh)"
fi

# ── [step 10/10] size gate + symlink self-check + size report + sha256 ──────
step "step 10/10 gates + reports"
mkdir -p "${DIST_DIR}"

# AC-2: ≤300MB size gate.
# `du -sk` returns size in 1024-byte blocks — unit-free integer, no parsing.
# `du -sh` is reserved for the human-readable report file.
DMG_KB="$(du -sk "${DMG_PATH}" | awk '{print $1}')"
DMG_HUMAN="$(du -sh "${DMG_PATH}" | awk '{print $1}')"

# Subcomponent sizes (input.md AC-2 requires both Resources/python and
# Resources/markitdown-venv in the report).
PY_HUMAN="$(du -sh "${RESOURCES_DIR}/python" 2>/dev/null | awk '{print $1}' || echo 'N/A')"
VENV_HUMAN="$(du -sh "${RESOURCES_DIR}/markitdown-venv" 2>/dev/null | awk '{print $1}' || echo 'N/A')"

# Git rev for the report (falls back gracefully outside a git checkout).
GIT_REV="$(git -C "${ROOT_DIR}" rev-parse --short HEAD 2>/dev/null || echo 'unknown')"

SIZE_REPORT="${DIST_DIR}/dmg_size_report.txt"
{
  echo "# NoteCapt DMG size report"
  echo "timestamp:        $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "git_rev:          ${GIT_REV}"
  echo "profile:          ${PROFILE}"
  echo "dmg_path:         ${DMG_PATH}"
  echo ""
  echo "dmg_total:        ${DMG_HUMAN}  (${DMG_KB} KB)"
  echo "resources/python: ${PY_HUMAN}"
  echo "resources/venv:   ${VENV_HUMAN}"
  echo ""
  echo "size_limit_kb:    ${SIZE_LIMIT_KB}  (300 MB)"
} > "${SIZE_REPORT}"
echo "[build-macos-dmg] size report → ${SIZE_REPORT}"
cat "${SIZE_REPORT}"

# Run the gate (after writing the report — operators want the report even
# on size failures so they can see what blew up the budget).
check_size_gate "${DMG_KB}" "${SIZE_LIMIT_KB}"

# AC-3: symlink integrity self-check via DMG mount.
# Mount read-only, no-browse, no-verify (faster), no-autoopen.
MOUNT_POINT="$(mktemp -d -t notecapt-mount.XXXXXX)"
hdiutil attach "${DMG_PATH}" \
  -mountpoint "${MOUNT_POINT}" \
  -nobrowse -noverify -noautoopen -readonly

MOUNTED_APP="${MOUNT_POINT}/${APP_NAME}.app"
SHIM_PATH="${MOUNTED_APP}/Contents/Resources/markitdown-venv/bin/python"

if [[ ! -L "${SHIM_PATH}" ]]; then
  echo "[build-macos-dmg] AC-3 FAIL: ${SHIM_PATH} is not a symlink inside the mounted DMG" >&2
  ls -l "$(dirname "${SHIM_PATH}")" >&2 || true
  exit 1
fi

SHIM_TARGET="$(readlink "${SHIM_PATH}")"
echo "[build-macos-dmg] AC-3 symlink target: ${SHIM_TARGET}"

if ! is_relative_symlink_target "${SHIM_TARGET}"; then
  echo "[build-macos-dmg] AC-3 FAIL: symlink target is not a relative path: ${SHIM_TARGET}" >&2
  exit 1
fi

# Detach explicitly here so the cleanup trap doesn't need to (idempotent).
hdiutil detach "${MOUNT_POINT}" -quiet
rmdir "${MOUNT_POINT}" 2>/dev/null || true
MOUNT_POINT=""
echo "[build-macos-dmg] AC-3 OK — symlink preserved, target is relative"

# AC-6: SHA256 → dist/<version>.sha256 (GNU shasum -c-compatible format).
VERSION="$(read_tauri_version "${TAURI_CONF}")"
if [[ -z "${VERSION}" ]]; then
  echo "[build-macos-dmg] FAIL: could not read version from ${TAURI_CONF}" >&2
  exit 1
fi
SHA_FILE="${DIST_DIR}/${VERSION}.sha256"
# `shasum -a 256` output format: "<hash>  <path>". We rewrite the path to a
# basename so the file is portable (a downstream user runs `shasum -c` next
# to the DMG, not at the absolute path on our build machine).
DMG_BASENAME="$(basename "${DMG_PATH}")"
DMG_SHA="$(shasum -a 256 "${DMG_PATH}" | awk '{print $1}')"
printf '%s  %s\n' "${DMG_SHA}" "${DMG_BASENAME}" > "${SHA_FILE}"
echo "[build-macos-dmg] sha256 → ${SHA_FILE}"
cat "${SHA_FILE}"

echo ""
echo "════════════════════════════════════════════"
echo "  DMG ready:    ${DMG_PATH}"
echo "  Size:         ${DMG_HUMAN}  (${DMG_KB} KB / limit ${SIZE_LIMIT_KB} KB)"
echo "  SHA256:       ${SHA_FILE}"
echo "  Size report:  ${SIZE_REPORT}"
echo "════════════════════════════════════════════"
