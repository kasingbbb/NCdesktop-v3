#!/usr/bin/env bash
# prepare-venv-shim.sh
#
# ADR-003: symlink venv-shim (NOT `python -m venv --copies`).
# Reason: `python -m venv --copies` breaks the @executable_path rpath of the
# bundled standalone Python distribution, causing dyld load failures inside
# the signed .app (H4). A pure-symlink shim preserves the original binary's
# rpath while still exposing a `python` / `python3` entry point at the path
# the runtime scheduler expects (Resources/markitdown-venv/bin/python).
#
# Constraints (per task_003 AC):
#   - AC-1: create relative symlinks python -> ../../python/bin/python3.12,
#           python3 -> python.
#   - AC-2: no `python -m venv`, no `virtualenv`, no `cp -R`.
#   - AC-4: symlinks must be RELATIVE (readlink result must not start with /),
#           so the .app remains relocatable.
#   - shim dir must contain only the two symlinks (no pyvenv.cfg / site-packages).
#   - Idempotent: re-running is a no-op when shim is already correct.

set -euo pipefail

# Resolve script dir so the script works from any cwd; operate against the
# NCdesktop project root (one level above scripts/).
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${PROJECT_ROOT}"

SHIM_DIR="src-tauri/resources/markitdown-venv/bin"
SHIM_PARENT="src-tauri/resources/markitdown-venv"
LIB_SYMLINK="${SHIM_PARENT}/lib"
LIB_TARGET_REL="../python/lib"   # macOS dyld @executable_path/../lib 解析锚点（2026-05-28 真机修复）
PYTHON_TARGET_REL="../../python/bin/python3.12"
PYTHON_ABS="src-tauri/resources/python/bin/python3.12"

# Pre-flight: upstream task_001 must have placed the standalone Python.
if [[ ! -x "${PYTHON_ABS}" ]]; then
    echo "ERROR: expected standalone Python at ${PYTHON_ABS} (task_001 prerequisite)." >&2
    exit 1
fi

mkdir -p "${SHIM_DIR}"

# Idempotency check: both symlinks present and pointing to the correct
# RELATIVE targets => skip.
existing_python_link=""
existing_python3_link=""
if [[ -L "${SHIM_DIR}/python" ]]; then
    existing_python_link="$(readlink "${SHIM_DIR}/python")"
fi
if [[ -L "${SHIM_DIR}/python3" ]]; then
    existing_python3_link="$(readlink "${SHIM_DIR}/python3")"
fi

if [[ "${existing_python_link}" == "${PYTHON_TARGET_REL}" \
   && "${existing_python3_link}" == "python" ]]; then
    echo "shim already present, skipping"
else
    # AC-1 + AC-4: create RELATIVE symlinks.
    # -s symlink, -n do not deref existing symlink target dir, -f overwrite.
    ln -snf "${PYTHON_TARGET_REL}" "${SHIM_DIR}/python"
    ln -snf "python" "${SHIM_DIR}/python3"
    echo "created shim symlinks under ${SHIM_DIR}/"
fi

# 2026-05-28 真机打包修复：dyld 解析 @executable_path 用 symlink 路径
# （markitdown-venv/bin/python）找 ../lib/libpython3.12.dylib → markitdown-venv/lib/libpython3.12.dylib
# 不存在 → SIGKILL。修：在 markitdown-venv/ 加 lib symlink 指向 ../python/lib。
if [[ -L "${LIB_SYMLINK}" ]]; then
    existing_lib_link="$(readlink "${LIB_SYMLINK}")"
    if [[ "${existing_lib_link}" != "${LIB_TARGET_REL}" ]]; then
        ln -snf "${LIB_TARGET_REL}" "${LIB_SYMLINK}"
        echo "updated lib symlink: ${LIB_SYMLINK} -> ${LIB_TARGET_REL}"
    fi
elif [[ ! -e "${LIB_SYMLINK}" ]]; then
    ln -snf "${LIB_TARGET_REL}" "${LIB_SYMLINK}"
    echo "created lib symlink: ${LIB_SYMLINK} -> ${LIB_TARGET_REL}"
fi

# AC-4 self-check: confirm both symlinks are RELATIVE (no leading /).
for link_name in python python3; do
    link_path="${SHIM_DIR}/${link_name}"
    if [[ ! -L "${link_path}" ]]; then
        echo "ERROR: ${link_path} is not a symlink." >&2
        exit 1
    fi
    target="$(readlink "${link_path}")"
    if [[ "${target}" == /* ]]; then
        echo "ERROR: ${link_path} -> ${target} is an absolute path (AC-4 violation)." >&2
        exit 1
    fi
done

# Constraint self-check: shim dir must contain ONLY the two symlinks.
# (No pyvenv.cfg, no site-packages, no lib/, no copied binaries.)
unexpected=()
while IFS= read -r entry; do
    base="$(basename "${entry}")"
    if [[ "${base}" == "python" || "${base}" == "python3" ]]; then
        if [[ -L "${entry}" ]]; then
            continue
        fi
        unexpected+=("${entry} (exists but not a symlink)")
    else
        unexpected+=("${entry}")
    fi
done < <(find "${SHIM_DIR}" -mindepth 1 -maxdepth 1)

# Also ensure no stray files at the parent shim root (e.g. pyvenv.cfg).
# 例外：`lib` symlink → ../python/lib（2026-05-28 dyld 修复，允许存在）。
while IFS= read -r entry; do
    base="$(basename "${entry}")"
    if [[ "${base}" == "bin" ]]; then
        continue
    fi
    if [[ "${base}" == "lib" && -L "${entry}" ]]; then
        continue
    fi
    unexpected+=("${entry}")
done < <(find "src-tauri/resources/markitdown-venv" -mindepth 1 -maxdepth 1)

if [[ ${#unexpected[@]} -gt 0 ]]; then
    echo "ERROR: unexpected entries inside markitdown-venv (shim must be pure symlinks):" >&2
    for u in "${unexpected[@]}"; do
        echo "  - ${u}" >&2
    done
    exit 1
fi

echo "prepare-venv-shim.sh: OK"
