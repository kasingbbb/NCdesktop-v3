#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# prepare-embedded-ffmpeg.sh
#
# 目的: 下载固定 release 的 **静态** ffmpeg (macOS arm64, 仅依赖系统库) 到
#       src-tauri/resources/ffmpeg，供 DMG 自包含分发。视频导入抽音频
#       (extraction::video_audio) 在用户机无 Homebrew ffmpeg 时回退到此内置二进制。
#
# 为什么不用 Homebrew 的 ffmpeg:
#   /opt/homebrew/bin/ffmpeg 动态链接 Cellar/*/lib/*.dylib，直接 cp 进 .app 会
#   因缺 dylib 在用户机崩。本脚本拉的是 eugeneware/ffmpeg-static 的静态构建，
#   `otool -L` 仅系统库(/usr/lib + 系统 framework)，可重定位、可签名。
#
# 严格遵守(对齐 prepare-embedded-python.sh 红线):
#   - 下载 URL + SHA256 为硬编码常量，禁动态拼接。
#   - 平台守卫: 仅 macOS arm64。
#
# 用法:
#   ./scripts/prepare-embedded-ffmpeg.sh           # 幂等(已存在且校验通过则跳过)
#   ./scripts/prepare-embedded-ffmpeg.sh --force   # 显式重下
# ---------------------------------------------------------------------------
set -euo pipefail

# ---- 硬编码常量(禁止动态拼接) ----------------------------------------------
readonly FFMPEG_DOWNLOAD_URL="https://github.com/eugeneware/ffmpeg-static/releases/download/b6.0/ffmpeg-darwin-arm64"
# 本机 (Darwin arm64, 2026-05-31) 实测下载 + `ffmpeg -version`=6.0 交叉验证。
readonly FFMPEG_SHA256="a90e3db6a3fd35f6074b013f948b1aa45b31c6375489d39e572bea3f18336584"

# ---- 路径常量 ---------------------------------------------------------------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly SCRIPT_DIR
readonly ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
readonly RESOURCES_DIR="${ROOT_DIR}/src-tauri/resources"
readonly FFMPEG_OUT="${RESOURCES_DIR}/ffmpeg"

# ---- 参数 -------------------------------------------------------------------
FORCE="0"
for arg in "$@"; do
  case "${arg}" in
    --force) FORCE="1" ;;
    -h|--help) sed -n '2,24p' "${BASH_SOURCE[0]}"; exit 0 ;;
    *) echo "[prepare-embedded-ffmpeg] unknown arg: ${arg}" >&2; exit 2 ;;
  esac
done

# ---- 平台守卫 ---------------------------------------------------------------
HOST_OS="$(uname -s)"; HOST_ARCH="$(uname -m)"
if [[ "${HOST_OS}" != "Darwin" || "${HOST_ARCH}" != "arm64" ]]; then
  echo "[prepare-embedded-ffmpeg] ERROR: 仅支持 macOS arm64，当前 ${HOST_OS}/${HOST_ARCH}" >&2
  exit 1
fi

verify_sha256() {
  local file="$1"
  local got
  got="$(shasum -a 256 "${file}" | awk '{print $1}')"
  [[ "${got}" == "${FFMPEG_SHA256}" ]]
}

# ---- 幂等: 已存在且功能正常则跳过 ------------------------------------------
# 注意：不能再比 sha256——本脚本末尾会 ad-hoc 重签，改动二进制字节，使 sha256
# 与下载常量不再相等。改用功能性判据（可执行 + 仅系统库 + -version 可跑），
# 对"已签名的同一个 ffmpeg"稳定为真，避免每次构建白下 43M。
if [[ "${FORCE}" != "1" && -x "${FFMPEG_OUT}" ]] \
   && ! otool -L "${FFMPEG_OUT}" 2>/dev/null | grep -qE "/opt/homebrew|/usr/local/" \
   && "${FFMPEG_OUT}" -version >/dev/null 2>&1; then
  echo "[prepare-embedded-ffmpeg] ffmpeg already present + functional, skipping (use --force to redownload)"
  exit 0
fi

mkdir -p "${RESOURCES_DIR}"
TMP="$(mktemp -t nc-ffmpeg.XXXXXX)"
trap 'rm -f "${TMP}"' EXIT

echo "[prepare-embedded-ffmpeg] downloading static ffmpeg (arm64) ..."
curl -fSL --connect-timeout 20 --retry 2 -o "${TMP}" "${FFMPEG_DOWNLOAD_URL}"

echo "[prepare-embedded-ffmpeg] verifying sha256 ..."
if ! verify_sha256 "${TMP}"; then
  echo "[prepare-embedded-ffmpeg] FAIL: sha256 mismatch" >&2
  echo "  expected: ${FFMPEG_SHA256}" >&2
  echo "  got:      $(shasum -a 256 "${TMP}" | awk '{print $1}')" >&2
  exit 1
fi

# 仅系统库守卫: 静态构建不得依赖 /opt/homebrew 等非系统路径
if otool -L "${TMP}" 2>/dev/null | grep -qE "/opt/homebrew|/usr/local/"; then
  echo "[prepare-embedded-ffmpeg] FAIL: 下载的 ffmpeg 依赖非系统库(非静态)，拒绝打包" >&2
  otool -L "${TMP}" >&2
  exit 1
fi

mv "${TMP}" "${FFMPEG_OUT}"
trap - EXIT
chmod +x "${FFMPEG_OUT}"

# ad-hoc 重签(去掉下载来源的签名/quarantine 风险；真机签名走 sign-bundle.sh 反序覆盖)
codesign --force --sign - "${FFMPEG_OUT}" 2>/dev/null || true
xattr -d com.apple.quarantine "${FFMPEG_OUT}" 2>/dev/null || true

echo "[prepare-embedded-ffmpeg] OK → ${FFMPEG_OUT}"
"${FFMPEG_OUT}" -version 2>/dev/null | head -1 || true
