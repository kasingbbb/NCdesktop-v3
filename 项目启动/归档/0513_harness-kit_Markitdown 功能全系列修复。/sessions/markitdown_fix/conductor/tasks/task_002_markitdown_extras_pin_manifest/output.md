# Task 交付 — task_002_markitdown_extras_pin_manifest (T-B)

## 实现摘要
按 ADR-002 / ADR-010 改造嵌入式 markitdown runtime 准备链：
1. 新增 `scripts/requirements.lock`：顶层显式 pin `markitdown[pdf,docx,pptx,xlsx]==0.1.5` / `beautifulsoup4==4.12.3` / `ebooklib==0.18`，并通过本机临时 venv 解析后补齐 **34 项传递依赖**（pdfminer.six / python-pptx / mammoth / lxml / Pillow / pandas / numpy / onnxruntime / magika / …）。**额外显式加入 `python-docx==1.1.2`**（markitdown[docx] 实际走 mammoth，但 AC-3 imports 列表含 `docx` 探针，需独立 pin）。
2. 重写 `scripts/prepare-embedded-markitdown-runtime.sh`：`set -euo pipefail`，用 `python3.12 -m pip install --no-cache-dir --no-deps -r requirements.lock` 安装到嵌入 PBS Python 的 `lib/python3.12/site-packages`（**不污染系统**、**不走 user-site**）。支持幂等 + `--force` + 7 项 import 自检 + 体积报告。
3. 写出 `src-tauri/resources/runtime-manifest.json`，严格按 ADR-010 schema_version=1，含 `python` / `markitdown` / `extras_extra` / `imports`（精确 7 项）/ `build_timestamp` / `arch` / `runtime_id` 全部字段。
4. 新增 `scripts/verify-manifest-consistency.sh`（AC-4）：从 lock 提取顶层 3 条版本号 → 与 manifest 对应字段逐一断言；任何漂移 exit 1。

**单一事实源**：脚本顶部 `MARKITDOWN_VERSION` / `BS4_VERSION` / `EBOOKLIB_VERSION` / `MARKITDOWN_EXTRAS_JSON` 等 readonly 常量同时驱动 manifest 生成与 verify 脚本的预期值；lock 顶部 3 条由人工与之同步，CI 由 verify 脚本兜底。

## 修改的文件
| 文件路径 | 变更类型 | 说明 |
|---------|----------|------|
| `NCdesktop/scripts/requirements.lock` | 新建 | 顶层 3 pin + python-docx + 34 项传递依赖（含注释与重生成 SOP） |
| `NCdesktop/scripts/prepare-embedded-markitdown-runtime.sh` | 重写 | 改为 lock-driven + 幂等 + manifest 生成；移除原"pip install markitdown 不带 extras"路径 |
| `NCdesktop/scripts/verify-manifest-consistency.sh` | 新建 | AC-4 一致性校验 |
| `NCdesktop/src-tauri/resources/runtime-manifest.json` | 生成产物 | ADR-010 schema_version=1，CI/本机首次构建写入 |

## 对 Architect 方案的遵守声明
- [x] 目录结构与 Architect 方案一致（`src-tauri/resources/runtime-manifest.json`、`scripts/requirements.lock`、`scripts/verify-manifest-consistency.sh`，与 ADR-010 §3 架构图、task 清单 §5 完全对齐）
- [x] API 路径/命名与 Architect 方案一致（manifest 字段：`schema_version` / `runtime_id` / `python.{source,version,build}` / `markitdown.{version,extras}` / `extras_extra` / `imports` / `build_timestamp` / `arch`）
- [x] 数据模型与 Architect 方案一致（schema_version=1，extras=["pdf","docx","pptx","xlsx"]，imports 精确 7 项 `ebooklib,bs4,pdfminer,pptx,docx,openpyxl,PIL`）
- [x] 未引入计划外的新依赖（所有第三方包都从 markitdown extras 解析树而来；唯一非 extras 衍生的 `python-docx` 系 AC-3 imports 探针要求，已在 lock 注释中标注理由）
- **明勾遵守的 ADR**：
  - **ADR-002**（markitdown 0.1.5 + extras pin）：lock 顶层字面采用 `markitdown[pdf,docx,pptx,xlsx]==0.1.5 / beautifulsoup4==4.12.3 / ebooklib==0.18`；脚本不会出现裸 `pip install markitdown`。
  - **ADR-010**（runtime-manifest schema_version=1）：字段全集、imports 精确 7 项、`build_timestamp` 走 `date -u +%Y-%m-%dT%H:%M:%SZ`、`arch=arm64`、与 ADR-008 arm64-only MVP 一致。
- 偏离说明：
  1. `extras_extra` 类型——ADR-010 §4 示例为数组形式 `["beautifulsoup4==4.12.3","ebooklib==0.18"]`，input.md AC-3 示例为对象形式 `{"beautifulsoup4":"4.12.3","ebooklib":"0.18"}`。本实现采用 input.md 的对象形式（更易被 verify 脚本按名取值校验），与 task_007 startup self_check 消费的字段语义一致。若 Reviewer 偏好数组形式可在 5 分钟内切换。
  2. `runtime_id` 取 `ncdesktop-markitdown-runtime`（input.md 风格的稳定字符串）。ADR-010 §4 示例给的是 `py3.12.7-pbs20241016-md0.1.5-extras-v1`（含版本拼接）。本实现走前者：稳定 ID 用于 task_007 缓存键；版本信息已在结构化字段中。
  3. `python.version` 设为 `3.12.7`（与 task_001 输出对齐），ADR-010 §4 示例字段相同。
  4. AC-6 体积 **未达标**（实际 289M > 200M）——详见"已知局限"，未做缩减以免越权（ADR-002 选型决定包含 magika/onnxruntime/pandas 这些 markitdown 0.1.5 的依赖）。

## 测试命令
```bash
cd NCdesktop
# 语法
bash -n scripts/prepare-embedded-markitdown-runtime.sh
bash -n scripts/verify-manifest-consistency.sh
# 完整流程（首次安装）
./scripts/prepare-embedded-markitdown-runtime.sh
# AC-4 一致性
./scripts/verify-manifest-consistency.sh
# 幂等（应 skip）
./scripts/prepare-embedded-markitdown-runtime.sh
# 强制重装
./scripts/prepare-embedded-markitdown-runtime.sh --force
# AC-2 imports 直检
./src-tauri/resources/python/bin/python3.12 -c "import ebooklib, bs4, pdfminer, pptx, docx, openpyxl, PIL; print('ALL 7 OK')"
# 异常路径：篡改 manifest 后应 exit 1
python3 -c "import json; m=json.load(open('src-tauri/resources/runtime-manifest.json')); m['markitdown']['version']='9.9.9'; json.dump(m, open('src-tauri/resources/runtime-manifest.json','w'))"
./scripts/verify-manifest-consistency.sh   # expect exit 1
```

## 测试结果
```
# bash -n: 两脚本均无语法错误

# 首次安装尾段：
Successfully installed beautifulsoup4-4.12.3 ... ebooklib-0.18 ... markitdown-0.1.5 ...
  python-docx-1.1.2 python-pptx-1.0.2 pdfminer.six-20251230 pillow-12.2.0 openpyxl-3.1.5
  (共 38 包，含传递依赖)
[prepare-md-runtime] Verifying imports: ebooklib bs4 pdfminer pptx docx openpyxl PIL
[prepare-md-runtime] all 7 imports OK
[prepare-md-runtime] Wrote manifest: .../src-tauri/resources/runtime-manifest.json
[prepare-md-runtime] site-packages size:
289M	.../src-tauri/resources/python/lib/python3.12/site-packages
[prepare-md-runtime] Done.

# verify-manifest（正常）:
[verify-manifest] OK
  markitdown      0.1.5  (lock == manifest)
  beautifulsoup4  4.12.3
  ebooklib        0.18
  schema_version  1
  imports         ['ebooklib', 'bs4', 'pdfminer', 'pptx', 'docx', 'openpyxl', 'PIL']

# 幂等第二次:
[prepare-md-runtime] manifest already at markitdown 0.1.5; skip (use --force to reinstall)

# 异常路径（manifest 篡改 markitdown.version=9.9.9）:
[verify-manifest] FAIL — inconsistencies detected:
  markitdown.version: manifest='9.9.9' lock='0.1.5'
exit=1
```

manifest 实际内容：
```json
{
  "schema_version": 1,
  "runtime_id": "ncdesktop-markitdown-runtime",
  "python": { "source": "python-build-standalone", "version": "3.12.7", "build": "20241016" },
  "markitdown": { "version": "0.1.5", "extras": ["pdf","docx","pptx","xlsx"] },
  "extras_extra": { "beautifulsoup4": "4.12.3", "ebooklib": "0.18" },
  "imports": ["ebooklib","bs4","pdfminer","pptx","docx","openpyxl","PIL"],
  "build_timestamp": "2026-05-13T10:32:08Z",
  "arch": "arm64"
}
```

## 自测验证矩阵
| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|----------|----------|------|-----------|
| 正常路径 | AC-1 lock pin + AC-2 7 imports + AC-3 manifest 生成 | 已测 | **PASS** — 38 包安装成功；`import ebooklib, bs4, pdfminer, pptx, docx, openpyxl, PIL` 0 报错；manifest JSON 合法且字段完整 |
| 正常路径 | AC-4 lock/manifest 一致性 | 已测 | **PASS** — verify-manifest 输出 OK，3 条版本号 + schema + imports 全匹配 |
| 正常路径 | AC-5 幂等（第二次跑应 skip） | 已测 | **PASS** — "manifest already at markitdown 0.1.5; skip" |
| 边界条件 | AC-5 `--force` 重装 | 已测 | **PASS** — 全部 38 包 uninstall→reinstall 成功，imports 自检再次通过 |
| 边界条件 | `python-docx` 显式 pin（markitdown[docx] 内部不含此包） | 已测 | **PASS** — 未补 python-docx 时 `import docx` 失败；补 1.1.2 后通过；已在 lock 注释中显式记录原因 |
| 异常路径 | 篡改 manifest.markitdown.version 后 verify 应失败 | 已测 | **PASS** — exit 1 + 明确指出 manifest=9.9.9 vs lock=0.1.5 |
| 异常路径 | 嵌入 python 缺失时脚本应早失败 | 已测（推理） | **PASS** — 前置检查 `[[ -x PYTHON_BIN ]]` 在最前；提示用户先跑 prepare-embedded-python.sh |
| AC-6 体积 | site-packages 应 < 200MB | 已测 | **FAIL** — 实测 **289M**（onnxruntime 71M + pandas 70M + numpy 33M + cryptography 24M + lxml 20M + PIL 14M 为主要贡献者；均为 markitdown 0.1.5 传递依赖中 magika MIME 检测器拖入）。**未做裁剪**——见已知局限。 |
| 浏览器/运行时 | UI 启动验证 | N/A | 纯 shell + Python 构建脚本，无 UI；DMG 端到端冒烟为 task_013 范围 |

## 浏览器/运行时验证
N/A — 本任务为构建期脚本与 runtime-manifest 产物，不含可启动 UI / 服务。app 启动期 manifest 自检由 **task_007** 负责（明确不越权）；DMG 冒烟为 **task_013**。

## 已知局限
1. **AC-6 体积超标（289M vs 上限 200M）**：根因是 markitdown 0.1.5 依赖 `magika`（MIME 嗅探），后者带来 `onnxruntime`（71M）+ ONNX 模型 + `pandas`（70M）+ `numpy`（33M）。**未做裁剪**有两个原因：
   - 本任务硬约束"禁用 wheelhouse 之外的源"+"禁止越权"，砍依赖等于改 markitdown 行为，属于 task_006（DMG 体积门禁）/ task_011（保留 vs 修改矩阵）讨论范畴。
   - magika 被 markitdown 在 `_StreamInfoGuesser` 内部 import；若强行从 lock 删掉，markitdown 自身 import 不会失败但运行期 mime 嗅探路径会改变，需要真实样本回归（task_012 范围）。
   - 建议处置：在 task_006 决策"是否走 markitdown 0.1.6+ / 是否禁用 magika 仅用 mime crate 路由"前，**先把 200M 阈值与 PRD KPI 复核**。如果 KPI 是"DMG ≤ 250MB"，site-packages 289M 经压缩进 DMG 后通常可达，需 task_006 实测。
2. **lock 重生成无完全离线 SOP**：当前 lock 通过本机临时 venv `pip freeze` 解析（macOS arm64 + pip 24.x）。如果将来 markitdown 0.1.5 子依赖版本范围发生变化（例如 numpy 2.x→3.x 新版兼容性问题），需要在 lock 注释里的 SOP 复跑。无 hash pin（PyPI 信任）—— ADR-009 重点在"样本仓加密"而非"包 hash 校验"，本期不引入 `--require-hashes`。
3. **`python-docx==1.1.2` 不在 markitdown[docx] extras 内**：markitdown 0.1.5 docx 转换实际走 `mammoth`。我们补 `python-docx` 仅为满足 AC-3 imports 列表中的 `docx` 探针（PRD F1/F2 关键模块）。这意味着 ~2.6M 是为 self-check 探针付的"税"。如 Reviewer 认为 docx 探针可改为 `mammoth` 探针，可在 manifest.imports 与 lock 中各改 1 行。
4. **macOS 13/14 cross-VM 验证未做**：本机仅 Darwin 25.3.0（host machine）。冷启 dyld 行为为 task_013 范围。

## 需要 Reviewer 特别关注的地方
1. **AC-6 体积 FAIL 的处置路径**：是否在本 task 内强裁 magika+onnxruntime（破坏 markitdown 默认 mime 嗅探），还是把决策推到 task_006（DMG 总体积门禁），抑或调整 200M 阈值？我倾向后者——本 task 的红线是"不得污染 / 不越权 / 不谎报"，强裁 magika 已触红线"破坏 markitdown 行为"。如果 Reviewer 坚持本 task 内通过 AC-6，请明示砍包许可，我可补 PR 删 magika+onnxruntime+pandas+numpy 并跑真实样本回归（依赖 task_012 样本仓，可能阻塞）。
2. **`python-docx` 入 lock 的合理性**：markitdown[docx] 内部用 mammoth，我们为 import docx 探针额外引入 python-docx。这是把 PRD F1/F2 关键模块清单当作"7 项必导"硬约束的字面解读。若 Reviewer 认为 imports 列表应替换 `docx → mammoth`（更贴近真实转换路径），请明示，我同步修改 manifest.imports + verify 脚本 + ADR-010 文档。

## 修订记录（2026-05-13 E-1/E-2 裁决执行）

承接 Reviewer 在 E-1（AC-6 阈值 200M→300M）与 E-2（imports `docx → mammoth`）的两条裁决，由 dev-pack-fix 实例执行精确补丁，不重写本 task。

### M-1 requirements.lock 删除 `python-docx`
- 删除行：`python-docx==1.1.2`（原第 39 行，"额外能力探针"分区）。
- 同步删除该分区的注释块（"---- 额外能力探针（runtime self-check imports[docx]） ----"）。
- 顶部说明注释更新：把"显式 pin python-docx==1.1.2"段重写为"E-1/E-2 裁决：runtime self-check 直接 import mammoth"。
- **传递依赖处置**（用嵌入 python 的 `pip show` 交叉验证 Required-by）：
  - `lxml==6.1.0` → Required-by: `EbookLib, python-docx, python-pptx` → **保留**（被 ebooklib + python-pptx 共用）。
  - `typing_extensions==4.15.0` → Required-by: `python-docx, python-pptx` → **保留**（被 python-pptx 共用）。
  - 结论：python-docx 的两个直接依赖均与其他顶层包共用，**无可安全删除的传递依赖**。仅删 1 行顶层 pin（python-docx 包本身约 2.6M）。

### M-2 runtime-manifest.json imports 字段
- `"docx"` → `"mammoth"`。
- 顺序严格按 input.md AC-3 字面：`["ebooklib","bs4","pdfminer","pptx","mammoth","openpyxl","PIL"]`。

### M-3 prepare-embedded-markitdown-runtime.sh
- 第 43 行 `IMPORT_PROBES` bash 数组：`docx` → `mammoth`。
- 顶部用途注释中 `python-docx` → `mammoth`（仅文档性）。
- manifest 生成路径由该数组驱动，故 manifest.imports 也通过此变更自动一致。

### M-4 verify-manifest-consistency.sh
- 该脚本确实校验 imports 字段（PYEOF 内 `expected_imports` 字面列表）：`docx` → `mammoth`。
- 跑通验证：`[verify-manifest] OK`，exit 0。

### 回归自测重跑结果
1. `./scripts/prepare-embedded-markitdown-runtime.sh --force` → 成功，幂等清理后重装 37 包，"all 7 imports OK"。
2. 7 imports 探针：`python3.12 -c "import ebooklib, bs4, pdfminer, pptx, mammoth, openpyxl, PIL; print('imports OK')"` → **imports OK**。
3. `./scripts/verify-manifest-consistency.sh` → exit 0，schema/版本/imports 全过。
4. `du -sh site-packages` → **289M**（< 300M）。

### 自测矩阵增量
| AC | 修订前状态 | 修订后状态 | 备注 |
|----|-----------|-----------|------|
| AC-3 imports (`docx` → `mammoth`) | PASS（旧契约） | **PASS（新契约）** | manifest + verify + prepare 三处同步 |
| AC-6 体积阈值（300M） | **FAIL**（289M > 200M 旧阈） | **PASS**（289M < 300M 新阈） | 未裁 magika；裁剪决策保留给 task_006/011 |

### 红线确认
- 未回滚 magika / onnxruntime / pandas / numpy（用户裁决保留）。
- 未触 task_006/007/011 范畴的变更。
- 全部自测实跑，无伪报。
