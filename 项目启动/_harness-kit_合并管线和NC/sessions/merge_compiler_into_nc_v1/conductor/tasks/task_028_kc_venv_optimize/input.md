# Task 输入 — task_028_kc_venv_optimize

## 目标
实装 F23 kc-venv 体积优化：在 prepare 脚本基础上追加剥离 / 清理动作，把 kc-venv 从 ~150MB 压到 ~80MB（DMG 总增量 < 100MB）。

## 前置条件
- 依赖 task：task_027（prepare-embedded-kc-runtime.sh 已实装）
- 必须先存在的文件/接口：
  - kc-venv 已通过 task_027 注入到 `.app/Contents/Resources/kc/venv/`

## 验收标准（Acceptance Criteria）
1. **AC-1**：新建 `scripts/optimize-kc-venv.sh`：
   ```bash
   #!/usr/bin/env bash
   set -euo pipefail
   KC_VENV="$1"   # e.g. ./build/NoteCapt.app/Contents/Resources/kc/venv
   
   # 已在 task_027 删过 __pycache__，本脚本进一步清理：
   
   # 1. 删除 dist-info/RECORD（pip 安装记录，运行时不需要）
   find "$KC_VENV" -name "RECORD" -path "*.dist-info/*" -delete
   
   # 2. 删除 .pyi stubs（运行时不需要）
   find "$KC_VENV" -name "*.pyi" -delete
   
   # 3. 删除 tests/ 目录（在 site-packages 内的）
   find "$KC_VENV/lib" -type d -name "tests" -exec rm -rf {} + 2>/dev/null || true
   find "$KC_VENV/lib" -type d -name "test" -exec rm -rf {} + 2>/dev/null || true
   
   # 4. 删除 *.dist-info/license / *.txt 等大文档
   find "$KC_VENV" -name "*.dist-info" -type d -exec sh -c 'rm -f "$1"/LICENSE* "$1"/license* "$1"/AUTHORS' _ {} \;
   
   # 5. 验证体积
   SIZE=$(du -sm "$KC_VENV" | cut -f1)
   echo "[optimize] kc-venv size: ${SIZE}MB"
   if [ "$SIZE" -gt 100 ]; then
       echo "WARN: kc-venv > 100MB (got ${SIZE}MB)"
   fi
   ```
2. **AC-2**：在 `prepare-embedded-kc-runtime.sh` 末尾追加 `./scripts/optimize-kc-venv.sh "$KC_TARGET/venv"`（或主 DMG 脚本中）
3. **AC-3**：验证体积阈值：
   - 优化后 kc-venv ≤ 100MB（理想 ~80MB）
   - DMG 总增量 ≤ 100MB（含 KC 源码 + venv）
4. **AC-4**：smoke test：优化后的 KC 仍能成功启动 + 响应 /api/v1/health（如有 CI 跑）
5. **AC-5**：optimize 脚本独立可重运行（幂等）

## 技术约束
- 不剥离运行时必需的 .so / .dylib
- 不剥离 jieba 的词典文件（KC 中文分词必需）
- 优化后 kc-venv 仍能在 Python 3.11 下运行
- 不强制 PYTHONOPTIMIZE=2（保留 docstrings，方便调试）

## 参考文件
- Architect output.md §"体积优化前/后实测"
- task_027 input.md
- 实测调研结论：剥离 gradio/pandas/numpy 已在 task_027 通过 kc-requirements.txt 排除；本 task 处理 venv 内**已装包**的进一步清理

## 预估影响范围
- 新建文件：
  - `scripts/optimize-kc-venv.sh`
- 修改文件：
  - `scripts/prepare-embedded-kc-runtime.sh`：末尾追加调用

## Reviewer 重点关注项
- 不能剥过头（验证 KC 仍能启动）
- 体积阈值告警（> 100MB warn）
- 幂等性（重跑不破坏）

## 复杂度
S（1d 工作量，~200 行脚本 + 测试）
