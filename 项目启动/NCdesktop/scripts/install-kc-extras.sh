#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# install-kc-extras.sh
#
# Phase 0（质量优先方案）：把 v2 pipeline_b 需要的大件依赖
# （sentence-transformers → 拉 torch，合计 ~2GB）装到 **bundle 外** 的 app 数据目录
# `kc_extras/site-packages`，让 DMG 保持 ~300M 不膨胀，本机即可解锁 v2 全质量
# （跨文档聚类 / 术语表）。app 启动 KC 时会自动把该目录挂到 PYTHONPATH
# （见 src-tauri/src/kc/process.rs `kc_site_packages_pythonpath`）。
#
# 关键：必须用 **已安装 .app 里那个嵌入 python**（python-build-standalone 3.12.x）来装，
# 保证 native wheel 的 ABI 与运行时解释器一致——否则 import 会崩。
#
# 用法：
#   ./scripts/install-kc-extras.sh                 # 用 /Applications/NoteCapt.app
#   ./scripts/install-kc-extras.sh /path/NoteCapt.app
#
# 迁到另一台电脑（无网/省事）：直接把
#   ~/Library/Application Support/com.notecapt.desktop/kc_extras
# 整个目录拷过去同一路径即可（前提：两台都是 macOS arm64 + 同一 .app 版本）。
# ---------------------------------------------------------------------------
set -euo pipefail

APP_PATH="${1:-/Applications/NoteCapt.app}"
BUNDLE_ID="com.notecapt.desktop"
EXTRAS_DIR="${HOME}/Library/Application Support/${BUNDLE_ID}/kc_extras/site-packages"
EMB_PY="${APP_PATH}/Contents/Resources/python/bin/python3.12"

echo "[install-kc-extras] app:        ${APP_PATH}"
echo "[install-kc-extras] python:     ${EMB_PY}"
echo "[install-kc-extras] target dir: ${EXTRAS_DIR}"

if [[ ! -x "${EMB_PY}" ]]; then
  echo "[install-kc-extras] ERROR: 找不到嵌入 python：${EMB_PY}" >&2
  echo "  请确认 NoteCapt.app 已安装，或传入正确的 .app 路径。" >&2
  exit 1
fi

echo "[install-kc-extras] python 版本：$("${EMB_PY}" --version 2>&1)"
mkdir -p "${EXTRAS_DIR}"

# --target 安装为扁平包目录（无 venv），native .so 全落在 EXTRAS_DIR 内。
# sentence-transformers 会拉入 torch / transformers / huggingface_hub / safetensors 等。
echo "[install-kc-extras] 开始安装（torch 较大，请耐心）…"
"${EMB_PY}" -m pip install \
  --no-cache-dir \
  --disable-pip-version-check \
  --target "${EXTRAS_DIR}" \
  --upgrade \
  "sentence-transformers>=2.7,<4" \
  "torch"

echo "[install-kc-extras] 校验 import（用嵌入 python + 该目录）…"
PYTHONPATH="${EXTRAS_DIR}" "${EMB_PY}" - <<'PY'
import importlib
for m in ("torch", "sentence_transformers"):
    mod = importlib.import_module(m)
    print(f"  OK {m} {getattr(mod, '__version__', '?')}")
import torch
print("  torch device test:", torch.tensor([1.0]).sum().item())
PY

# 预热 KC embedder 模型（BAAI/bge-small-zh-v1.5，~100MB），下载到默认 HF 缓存
# (~/.cache/huggingface)，与 app 内 KC 子进程同一用户共享，避免首次摄入卡在下载。
echo "[install-kc-extras] 预下载 embedding 模型 BAAI/bge-small-zh-v1.5（~100MB）…"
PYTHONPATH="${EXTRAS_DIR}" "${EMB_PY}" - <<'PY'
from sentence_transformers import SentenceTransformer
m = SentenceTransformer("BAAI/bge-small-zh-v1.5")
v = m.encode(["四步创业法 测试句子"])
print("  模型就绪，向量维度:", len(v[0]))
PY

DU=$(du -sh "${EXTRAS_DIR}" 2>/dev/null | awk '{print $1}')
echo "[install-kc-extras] 完成。kc_extras 体积：${DU}"
echo "[install-kc-extras] 重启 NoteCapt 后，v2 pipeline_b（聚类/术语/master_index 主题分类）即解锁全质量。"
