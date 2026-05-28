# Review Scorecard — task_027_dmg_packaging_kc

**Commit**: `542ef912` on `feat/windows-unit-13-cloud-ai`
**Reviewer 时点**: 2026-05-28
**形态**: Dev 实装完成；DMG 真机打包 + .app 启动验证 → PM dry-run（无真机情境下静态评审）

---

## 审查思考过程

### 1. Task 意图复述
实装 F22 DMG 打包脚本扩展：写 `prepare-embedded-kc-runtime.sh` 注入 kc-venv + KC 源码（仅 `compiler/` + `run_api.py`）到 `.app/Contents/Resources/kc/`，扩展 `runtime-manifest.json` 追加 `kc` 字段（runtime_id / version / commit_sha / python_version / venv_size_bytes / build_timestamp）。`kc-requirements.txt` 提供 KC 子进程 venv 的 pinned deps，红线禁 gradio/pandas/numpy/huggingface_hub/torch/transformers/jupyter（DMG 体积约束）。本 task 不串接 `build-macos-dmg.sh`（KC_REPO_PATH 是外部参数）、不签名、不剥离（task_028 处理）。

### 2. AC 检查结果

| AC | 描述 | 满足？ | 备注 |
|----|------|-------|------|
| AC-1 | `prepare-embedded-kc-runtime.sh` 6 步骤实装（stage venv / pip / 白名单 cp / 清 __pycache__ / 注入 .app / manifest） | ✅ | 6 步全有；额外补 `--dry-run` flag 用于 CI 静态测试（合理扩展） |
| AC-2 | `scripts/kc-requirements.txt` 精简版 + 红线 | ✅ | 15 行 active deps（含 httpx + pyyaml，多于 input.md 13 项；合理）；红线在注释中显式列出，T5 测试 grep 守护 |
| AC-3 | `build-macos-dmg.sh` 串接 KC 注入 | ❌ **不做** | output.md §1 显式声明：KC_REPO_PATH 是外部参数，不便硬编码；PM 真机手动追加（task_028 正式落入）。Dev 决策合理且**已在 output.md §3.3 写出 PM 手动追加位置**——审视清晰。 |
| AC-4 | manifest 含 `kc.{version, commit_sha, python_version}` | ✅+ | 3 字段全有；额外补 `runtime_id` / `venv_size_bytes` / `build_timestamp`，前者用于追溯 + 后者用于 task_028 体积守护。**保留 markitdown/python/imports 既有字段**（task_009 KC lifecycle 既有读取无 break）。 |
| AC-5 | dry-run 测试在 staging 模式下能成功跑 | ✅ | T3: mock APP + mock KC repo 下 `--dry-run` exit 0 且**未写盘**（reviewer 复跑确认） |
| AC-6 | 体积验证 `du -sh .../kc/` ~150-180MB | ⏸ PM 真机 | 脚本末尾 `du -sh "${KC_TARGET}"` 输出；reviewer 无真机 venv 不能直接验证 |

### 3. 关键发现（reviewer 复跑验证）

1. **reviewer 复跑全部静态测试**：14 PASS / 0 FAIL / 1 SKIP（shellcheck 未装），与 output.md §5 一致。`bash -n` 主脚本 + 测试脚本均 exit 0。
2. **manifest 兼容性 OK**：`src-tauri/src/extraction/runtime_check.rs:33` 的 `RuntimeManifest` struct **未用** `#[serde(deny_unknown_fields)]`，新增 `kc` 字段不会破坏既有 serde 反序列化（task_002 markitdown manifest 路径无退化）。
3. **NC kc client 路径对齐**：`src-tauri/src/kc/process.rs:852-913` 查找 `kc/venv/bin/python` 和 `kc/src/run_api.py`；脚本 Step 5 注入到 `${KC_TARGET}/venv` + `${KC_TARGET}/src/{compiler, run_api.py}`，**完全对齐 task_009 既有 KC lifecycle 期望的目录结构**。
4. **红线包真覆盖**：`grep -E "^(gradio|pandas|numpy|huggingface_hub|torch|transformers|jupyter)" kc-requirements.txt` 0 hit（仅注释中作为提醒）；T5 测试用正则 `^pkg[[:space:]]*[=><~]` 守护（先 strip 注释行再 grep），逻辑正确。
5. **跨平台兼容做到位**：脚本**不用任何 `sed -i`**（避免 BSD vs GNU 分歧），`find -exec rm -rf {} +` 与 `du -sk` + `awk` 是 POSIX 通用语法。`cp -R` 保留 venv 符号链接（venv/bin/python -> python3）正确。
6. **manifest 写入用 env 透传不用 inline 注入**：`PREP_KC_*` 环境变量透传到 python3 heredoc，**避免 shell 引号嵌入 JSON 注入风险**——比 input.md AC-1 示例代码（直接 `'$KC_REPO_PATH'` 内嵌）**更安全**。
7. **manifest schema_version 兜底**：`manifest.setdefault("schema_version", 1)` 仅在 manifest 不存在时补；存在则保留——这意味着即使 markitdown 还未跑、kc 单独跑也能产出**合法 v1 manifest**（虽然缺 python/markitdown 字段，task_009 KC lifecycle 不依赖这些字段）。

---

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| AC 覆盖 | 20% | **5** | 6 步骤全实装 + manifest 6 字段（高于 AC-4 的 3 字段）；AC-3 明示不做并给 PM 手动追加指引；AC-5 dry-run 真实可跑（reviewer 验证）；AC-6 体积 du 输出已埋点。**仅 AC-3 自动串接未做，但 output.md §3.3 已给 PM 明确 patch 位置**。 |
| Shell 鲁棒性 | 20% | **5** | `set -euo pipefail`（line 33）+ 5 项前置检查（line 90-112，含 .app/KC repo/whitelist source/python binary/requirements 均有 fail 退出）+ 中文路径全双引号 + 幂等 `rm -rf "${KC_TARGET}"` 重建（line 172）+ stage 失败保留诊断成功路径才 cleanup（line 138-143）+ readonly 常量防漂移。**唯一可挑剔**：`-h\|--help` 用 `sed -n '2,30p'` 自切头部注释，是技巧但与"不用 sed -i"声明无冲突——sed 用作 read-only 没问题。 |
| 静态测试覆盖 | 15% | **5** | T1-T5 共 14 测试 / 0 FAIL / 1 SKIP；T4 含 5 错误路径（no-args / missing .app / missing KC repo / missing compiler/ / missing python）；T5 含 7 红线 grep miss + 5 核心依赖 grep hit（含反向校验：核心包缺失也会 FAIL）。T3 验证 `--dry-run` **真的没写盘**（断言 `! -e ${KC_TARGET}`）—— dry-run 不留副作用是 CI 守护 essential。 |
| manifest 设计 | 15% | **5** | 6 字段（runtime_id / version / commit_sha / python_version / venv_size_bytes / build_timestamp）够 task_009 KC lifecycle 启动时查路径用；**保留 markitdown/python/imports 既有字段**（task_002 不退化）；schema_version 兜底；python heredoc 用 env 透传安全；commit_sha 容错（非 git 标 `unknown`，符合"真机 KC 可能是 release tarball"现实）；pyproject.toml version 用 regex 抓不引 tomli/tomllib（python 3.10 兼容）。**`venv_size_bytes` 是 Dev 额外补的体积守护字段**，task_028 verify-dmg-contents 会消费。 |
| kc-requirements.txt | 15% | **5** | 15 个 active deps 全部 pinned with upper-bound（除 `tiktoken` 无界——可接受，因 tiktoken 无 SemVer 严格约定）；红线 7 项在注释中显式标 +体积成本；安装策略说明（含传递依赖 vs `--no-deps + lock` 与 markitdown 风格区别）；重新生成方法 6 行 shell 注释；Python 3.11 pin 理由（langchain/onnxruntime ABI）明示。**唯一小瑕疵**：openai 锁 `<2` 但 openai SDK 2.x 已发布——可能 6 个月内 pip 解析触底，但 Dev 注释中无相关 TODO 提醒。 |
| PM 真机验证清单 | 15% | **4** | 5 大步骤 + 启动验证 + 失败回滚结构清晰；命令均可直接复制；体积/manifest 验证用 `du -sh` + `cat` 都易看输出。**扣 1 分原因（MINOR-1）**：缺少"真机启动 KC 后调 `curl http://localhost:<port>/health` 验证返回含 `ai_enabled` 字段"（与 task_020b 配套）。output.md §3.2 仅写"`ps aux \| grep run_api`"+ "无 ImportError"——但 task_020b 关键验收点是 KC `/health` 返回 `ai_enabled` 字段透传，task_027 真机验证最自然带一笔。 |

**综合分**：(5×0.20 + 5×0.20 + 5×0.15 + 5×0.15 + 5×0.15 + 4×0.15) = 1.00 + 1.00 + 0.75 + 0.75 + 0.75 + 0.60 = **4.85 / 5**

---

## 总体判断

- [x] **PASS**（综合分 4.85 ≥ 4.3，无 BLOCKER，0 MAJOR，3 MINOR）

理由：6 步骤全实装；shell 鲁棒性满分（set -euo pipefail + 5 前置检查 + 双引号 + 幂等 + stage diagnostics）；静态测试 14/14 + 1 SKIP；红线包真覆盖（reviewer grep 复核 0 hit）；manifest 设计**保留前序字段且额外补体积/timestamp 守护**；PM 清单 5 步可执行。AC-3 build-macos-dmg.sh 自动串接未做，但 Dev 在 output.md §3.3 给 PM 明确 patch 位置，且推到 task_028 落入是合理决策——不阻塞 PM 真机验证。

---

## 问题列表

### BLOCKER（必须修复）
无。

### MAJOR（强烈建议修复 / 后续 task 处理）
无。

### MINOR（可选 / 后续 task 捎带）

1. **PM 真机清单缺 KC `/health` 调用验证**
   - **位置**：output.md §3.2「启动验证（手动）」
   - **现状**：仅写 `ps aux | grep run_api` + 无 ImportError；未写 `curl http://localhost:<port>/health` 验证返回含 `ai_enabled: true|false` 字段
   - **影响**：task_020b 关键验收点是 KC `/health` 返回 `ai_enabled` 字段透传到 NC `KcHealthStatusDto`；task_027 真机 DMG 验证是**首次端到端跑 KC venv**，若不调 /health 看 ai_enabled，意味着 task_020b 的 DMG 内验证被推到更晚的 task。建议在 §3.2 补一行：
     ```bash
     curl http://localhost:$(grep KC_PORT ~/Library/Logs/NoteCapt/*.log | tail -1)/api/v1/health
     # 期望: {"status":"ok","ai_enabled":true|false,...}
     ```
   - **优先级**：可在 PM 真机执行前补 output.md，或推到 task_028 一并提示；不阻塞本 task PASS。

2. **shellcheck SKIP 未在 output.md 登记如何启用**
   - **位置**：output.md §5「shellcheck SKIP (未装)」
   - **现状**：仅说"PM 真机若装 shellcheck，运行测试套件 T2 自动跑"——但未给安装命令（`brew install shellcheck`）
   - **影响**：PM 若没用过 shellcheck，看到 SKIP 不知如何启用；CI 守护层缺一条。建议 output.md §5 改成「shellcheck SKIP（未装；`brew install shellcheck` 启用 T2）」一行
   - **优先级**：5 字注释 fix；可推到 task_028 一并。

3. **`openai>=1.30.0,<2` 上界 6 个月内可能触底**
   - **位置**：scripts/kc-requirements.txt:44
   - **现状**：openai SDK 2.x 已发布（2025 年）；锁 `<2` 短期 OK，但 6 个月内 pip 解析可能拒装新 wheel
   - **影响**：构建 venv 时 pip 会 fallback 到旧 openai 1.x；KC 不依赖 2.x 新 API 则 OK，但若 KC 升级到 langchain-openai 0.3+ 强依赖 openai 2.x → 红线交叉
   - **修复方向**：在文件头部"重新生成 / 升级方法"中加一条 TODO「KC 升级到 langchain-openai 0.3+ 时，同步放开 openai 上界到 `<3`」
   - **优先级**：可推到 task_028 / 后续 KC 子任务捎带；不阻塞本 task PASS。

---

## Reviewer 重点核查 — 8 项

| Reviewer 关注项 | 结论 |
|---|---|
| 红线包真覆盖（grep gradio/pandas/numpy/hf_hub/torch/transformers/jupyter） | ✅ **0 hit**（仅注释中提及；T5 测试用 `^pkg[[:space:]]*[=><~]` 正则守护，先 strip 注释行再 grep） |
| manifest 兼容性（保留 markitdown/python/imports 字段） | ✅ `manifest.setdefault("schema_version", 1)` + 仅 `m["kc"] = {...}`，**不 clobber** 既有字段；reviewer 已读 `runtime_check.rs:33` 确认 struct 未用 `deny_unknown_fields` |
| macOS/Linux 双兼容（是否用 `sed -i ''` 特例） | ✅ **0 处** `sed -i`；`find -exec rm -rf {} +` 与 `du -sk \| awk` POSIX 通用；`-h\|--help` 中 `sed -n '2,30p'` 是 read-only 切片不影响跨平台 |
| 静态测试 14 PASS / 1 SKIP 合理性 | ✅ reviewer 复跑确认 14 PASS / 1 SKIP（shellcheck 未装）；T1-T5 覆盖度足；T3 `--dry-run` 真没写盘 |
| PM 真机清单 5 步可执行性 | ✅ 命令可复制；体积/manifest 验证用 `du -sh` + `cat` 易看；3.4 失败回滚 1 行 `rm -rf` 清；**MINOR-1 缺 /health 调用** |
| KC 客户端路径匹配（NC kc/process.rs 期望 vs 脚本注入） | ✅ NC 查 `kc/venv/bin/python` + `kc/src/run_api.py`；脚本注入 `${KC_TARGET}/venv` + `${KC_TARGET}/src/run_api.py`——**完全对齐** |
| python heredoc 注入安全 | ✅ `PREP_KC_*` env 透传不 inline shell 引号嵌入；比 input.md 示例代码更安全 |
| 幂等性 | ✅ `rm -rf "${KC_TARGET}"` 后 `mkdir -p` 重建；同 commit 重跑等价；stage 失败保留 tmp 诊断、成功才 cleanup |

---

## 给 Dev / PM 的指引

**本 task 判 PASS（4.85 / 5），无需立即修复**。

### 不阻塞 PASS 的后续技术债（推到 task_028 或单独捎带）

1. **MINOR-1**：PM 真机验证清单 §3.2 启动验证补一行 `curl /api/v1/health` 期望返回 `ai_enabled` 字段（与 task_020b 配套验证）
2. **MINOR-2**：output.md §5 shellcheck SKIP 行补 `brew install shellcheck` 启用提示
3. **MINOR-3**：kc-requirements.txt 文件头部"重新生成方法"加 TODO「KC 升级 langchain-openai 0.3+ 时同步放开 openai `<3`」

### PM 真机验证关键检查项（确认 task_027 端到端 PASS 的必要证据）

- [ ] `bash scripts/__tests__/prepare-embedded-kc-runtime.test.sh` 输出 `PASS: 14+ FAIL: 0`
- [ ] `du -sh .../NoteCapt.app/Contents/Resources/kc/` 输出 ~150-180MB（剥离前）
- [ ] `cat .../runtime-manifest.json` 含 `kc.{runtime_id, version, commit_sha, python_version, venv_size_bytes, build_timestamp}` 且 `markitdown / python / imports` 字段未丢
- [ ] `open NoteCapt.app` 后 `ps aux | grep run_api` 见 KC 子进程；`~/Library/Logs/NoteCapt/` 无 ImportError
- [ ] **(MINOR-1 推荐)** `curl http://localhost:<KC_PORT>/api/v1/health` 返回 `ai_enabled: true|false` 字段
