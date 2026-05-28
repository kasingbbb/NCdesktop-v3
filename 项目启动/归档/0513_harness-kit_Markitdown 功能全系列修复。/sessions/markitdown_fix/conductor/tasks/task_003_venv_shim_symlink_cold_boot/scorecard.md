# Review Scorecard — task_003_venv_shim_symlink_cold_boot (T-C)

## 审查思考过程

1. **Task 意图**：用纯 symlink 方式在 `Resources/markitdown-venv/bin/` 建立 `python` / `python3` 入口，绕开 `python -m venv --copies` 对 standalone Python `@executable_path` rpath 的破坏（H4 / ADR-003）；并提供干净 shell 冷启动 imports 探针验证脚本（ADR-010 / runtime-manifest），imports 字面必须含 mammoth（E-2 修订）。
2. **AC 检查结果**：
   - AC-1 ✅：`prepare-venv-shim.sh` 用 `ln -snf` 创建 `python -> ../../python/bin/python3.12`、`python3 -> python`；`readlink` 实测一致。
   - AC-2 ✅：脚本顶部 1–18 行注释显式引用 ADR-003，全脚本未出现 `python -m venv` / `virtualenv` / `--copies` / `cp -R`。
   - AC-3 ⚠️：`verify-venv-shim.sh` L78–82 imports 字面顺序与 input.md / ADR-010 一致（`ebooklib, bs4, pdfminer, pptx, mammoth, openpyxl, PIL`，含 mammoth）；但 `env -i` 调用透传了 `HOME="${HOME}"`，偏离 AC-3 字面 `env -i PATH=/usr/bin:/bin`，且 HOME 直接引导 Python 解析 `~/.local/lib/.../site-packages`，与"冷启动隔离用户环境"语义冲突 → 见 MAJOR-1。
   - AC-4 ✅：prepare 与 verify 双重自检 readlink 不以 `/` 开头；实测 `../../python/bin/python3.12` 与 `python` 均相对。
   - AC-5 ⚠️：标 PENDING-USER-MACHINE，已说明项目无 macOS CI runner、本机非干净 VM；本机已用 `env -i` 模拟通过。属合理待办（input.md AC-5 本身允许"由本地手测代办"）。
3. **关键发现**：
   - 单一 MAJOR 问题：HOME 透传削弱了 AC-3 的"冷启动隔离"语义。可能在用户机器存在 `~/.local/lib/python3.12/site-packages` 时让 imports "假成功"（命中用户 site-packages 而非 standalone）。
   - 其余红线（shim 纯净、不含 cp -R/venv、imports 含 mammoth）全部命中；脚本幂等、自检完备。

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 25% | 4 | AC-1/2/4 完全 PASS；AC-3 imports 字面正确但 `HOME` 透传偏离字面契约，存在用户 site-packages 污染风险；AC-5 PENDING 合理。 |
| 安全性 | 25% | 4 | 脚本 `set -euo pipefail`、ln -snf 幂等，无任意命令注入面。唯一减分点是 HOME 透传可能让冷启自检在"用户 site 兜底"下假阳性，等价于对 H4 边界放水。 |
| 代码质量 | 15% | 5 | 注释清晰、变量命名规范、幂等分支与自检分层逻辑独立可读；prepare 与 verify 职责单一。 |
| 测试覆盖 | 15% | 4 | 矩阵覆盖正常 / 幂等 / 边界（pyvenv.cfg 注入）/ 异常（绝对 symlink）/ 干净 shell 7 项 imports；AC-5 干净 VM PENDING 但已 input.md 授权。 |
| 架构一致性 | 10% | 5 | 显式 ADR-003 引用；imports 顺序与 ADR-010 补强段字面一致（mammoth 非 docx）；目录形态与 ADR-003 / runtime 探测路径一致。 |
| 可维护性 | 10% | 5 | 脚本头部"为何这么做"陈述充分；`SCRIPT_DIR` 自解析使其不绑死 cwd；输出消息直白便于 CI 解析。 |

**综合分：4.35 / 5**（加权：0.25·4 + 0.25·4 + 0.15·5 + 0.15·4 + 0.10·5 + 0.10·5 = 4.35）

## 总体判断

- [x] **FIX**

理由：核心 AC 已落地且红线全部满足，单一 MAJOR（HOME 透传）涉及 AC-3"干净"语义偏离，1 行可修，不到 BLOCKER 程度。

## 问题列表

### BLOCKER
无。

### MAJOR（强烈建议修复）

1. **问题**：`verify-venv-shim.sh` L81 透传 `HOME="${HOME}"`，与 AC-3 字面 `env -i PATH=/usr/bin:/bin` 偏离，且 Python 会用 `HOME` 解析 `~/.local/lib/python3.12/site-packages`（user site）。若评审者 / CI 机器恰好用户 site 装了 `ebooklib` / `mammoth` 等，imports 探针会"假成功"，达不到"证明 standalone Python + 嵌入 site-packages 自给自足"的目的——这正是 AC-3 想隔离的场景。
   - **代码位置**：`NCdesktop/scripts/verify-venv-shim.sh:81`
   - **修复方向**：删除 HOME 透传；如确实需要压制 pip user-site 警告，改用 Python 内置 flag 而非环境变量。两种推荐方案任选其一：
     - 方案 A（最贴近 AC-3 字面）：`env -i PATH=/usr/bin:/bin "${SHIM_PY}" -E -s -c "${PROBE}"`（`-E` 忽略 `PYTHON*` 环境变量；`-s` 跳过 user site）。
     - 方案 B：`env -i PATH=/usr/bin:/bin HOME=/var/empty "${SHIM_PY}" -c "${PROBE}"`（提供一个保证无 `.local/lib` 的 HOME，避免任何 user site 命中）。
   - **验证标准**：(a) 命令行不再包含 `HOME="${HOME}"`；(b) 在评审者本机用 `mkdir -p ~/.local/lib/python3.12/site-packages && touch ~/.local/lib/python3.12/site-packages/ebooklib.py`（或临时 pip --user 安装一个 stub）后重跑 `verify-venv-shim.sh`，imports 仍走 standalone site-packages，输出 `ok` 且 exit 0；(c) 注释同步更新，写明"AC-3 字面契约要求不透传 HOME"。

### MINOR（可选）

1. `prepare-venv-shim.sh` 第 89 行的浅扫 `find -mindepth 1 -maxdepth 1` 对 `bin/`、第 98 行同样浅扫根目录——按当前 shim 形态（只 2 层）足够，但若未来 ADR 演进允许 `lib/site-packages/` 嵌套，扫描将漏检深层污染。建议在脚本里加一行注释说明"浅扫的前提是 shim 形态仅 2 层；任何形态变更需同步扩到 `-maxdepth 3` 或递归"。**判定 MINOR-RECOMMEND**：不阻塞 PASS，作为可维护性提示。
2. AC-5 PENDING 项已合理标注，但建议在 output.md "已知局限"第 1 条增加一句"复跑命令"模板（如 `ssh vm 'cd NCdesktop && ./scripts/verify-venv-shim.sh'`），便于交付侧用户一键执行。

## 给 Dev 的修复指引（FIX）

### 问题清单（按优先级排序）

#### MAJOR
1. 见上 MAJOR-1：删 `HOME="${HOME}"`，改用 `-E -s` 或 `HOME=/var/empty`，并补一条对应的负向测试（用户 site 内放 stub 仍走 standalone）。

#### MINOR
1. 见上 MINOR-1：注释明确 shim 形态浅扫的前提条件。
2. 见上 MINOR-2：output.md 补复跑命令模板。

### 修复范围约束
- **只修以上列出的问题**，不要连带重构 prepare-venv-shim.sh / 不动 task_001、task_002 产物。
- 修复完成后重跑：`bash -n` 静态检查 + `./scripts/prepare-venv-shim.sh`（幂等）+ `./scripts/verify-venv-shim.sh`，并新增"user site stub"负向测试一次。
- 修复不应触及 task_004 / task_007 的运行时探测 / self-check 路径。

---

## R2 复审追加（FIX 第 1 轮）

- **复审日期**：2026-05-13
- **R2 判决**：**PASS**

### A/B/C/D/E 五项逐条结论

- **A. MAJOR-1 是否真闭合**：✅ 闭合。`verify-venv-shim.sh` L93-94 已改为 `env -i PATH=/usr/bin:/bin "${SHIM_PY}" -E -s -c "${PROBE}"`，无 `HOME="${HOME}"` 透传；脚本头 L15-24 注释明示"AC-3 字面契约要求不透传 HOME"并解释 `-E -s` 双重隔离动机；方案 A 落实到位。
- **B. user-site stub 负向测试是否真在脚本中**：✅ 真实存在。L112-161 实装 Step 3：`USER_SITE_DIR=${HOME}/.local/lib/python3.12/site-packages`，`STUB_FILE=ebooklib.py`，`STUB_CREATED_BY_US=1` 标记位，`trap cleanup_stub EXIT` 兜底清理且只清理自己创建的文件，`[[ -e STUB_FILE ]]` 已存在则 skip 不污染；stub 写入 `raise ImportError(...)`，重跑 probe 期望 `ok` 且 exit 0，否则 exit 1。逻辑严密、可重入。
- **C. 6 项自测矩阵重跑是否真 PASS**：✅ output.md L215-237 给出 6 项命令+期望+实际；含完整 `./scripts/verify-venv-shim.sh` 输出（主 probe `ok exit=0` + decoy probe `ok exit=0` + `user-site decoy OK` + `verify-venv-shim.sh: OK`），dev 真跑痕迹齐全。
- **D. 未触及非授权区**：✅ 文件 mtime 验证：`verify-venv-shim.sh` 20:43（本轮）、`prepare-venv-shim.sh` 18:46（前轮）、`prepare-embedded-python.sh` 18:07（task_001 旧改动，非本轮）；git status 显示本轮 FIX 唯一变更目标即 verify 脚本（与 dev 自报一致）。未触及 desensitize/encrypt/decrypt-samples、prepare-embedded-python、runtime-manifest、requirements.lock、SOP、workflow 等任一文件。
- **E. R1 已 PASS 项无回归**：✅ `set -euo pipefail` 在 L26；prepare-venv-shim.sh 未改（mtime 与 R1 一致）；imports 字面 L90 仍含 `mammoth`（非 `docx`）且 7 项顺序与 ADR-010 / E-2 修订一致；脚本头注释与 prepare 中 ADR-003 引用一致无矛盾。

### 红线四项

- HOME 透传残留：**PASS**（已删除）
- 决定性证据完全编造：**PASS**（脚本内 stub 测试段真实存在 L112-161）
- 触及非授权区：**PASS**（仅改 verify-venv-shim.sh，mtime + git status 双重佐证）
- 破坏 R1 已 PASS AC：**PASS**（mammoth 在、set -euo 在、prepare 未动）

### 综合分

R1 综合分 4.35 / 5。MAJOR-1 已闭合，AC-3 功能正确性 4→5，安全性 4→5；其余维度不变。

加权重算：0.25·5 + 0.25·5 + 0.15·5 + 0.15·4 + 0.10·5 + 0.10·5 = **4.85 / 5**

### 判决理由

唯一 MAJOR 已 1 行字面闭合并加入决定性负向测试段，红线四项全部 PASS，回归无影响。可直接进 task_003 终态。
