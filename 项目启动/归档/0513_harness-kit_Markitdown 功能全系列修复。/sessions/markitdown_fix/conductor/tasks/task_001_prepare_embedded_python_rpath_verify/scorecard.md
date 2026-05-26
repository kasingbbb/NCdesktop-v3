# Review Scorecard — task_001_prepare_embedded_python_rpath_verify

## 审查思考过程

1. **Task 意图**：改造 `scripts/prepare-embedded-python.sh` 下载固定 release 的 python-build-standalone 3.12.7 (arm64) 到 `src-tauri/resources/python/`，并新建独立的 `scripts/verify-rpath.sh` 用 `otool -L` 验证所有依赖路径"可重定位 / 系统库"白名单。本 task 是 DMG 自包含分发链起点。

2. **AC 检查结果**
   - AC-1（硬编码 URL + SHA256）：PASS — `prepare-embedded-python.sh:22-28` 三个 `readonly` 常量，无字符串拼接；首轮 SHA mismatch 真实触发证明校验生效。
   - AC-2（目录结构 bin/lib/include 齐全）：PASS — `prepare-embedded-python.sh:108-119` 三个 `[[ -x ]]` / `[[ -d ]]` 断言。
   - AC-3（rpath 白名单）：PASS-WITH-NOTE — Dev 主动声明扩展为 5 项（见偏离 1 判定）。
   - AC-4（幂等 skip）：PASS — `prepare-embedded-python.sh:67-71` 检查 `bin/python3.12` 可执行；实测输出 "python already present, skipping"。
   - AC-5（`set -euo pipefail` + 双引号 + `--force`）：PASS — 两脚本第 15/19 行均有 `set -euo pipefail`；grep 未发现裸变量引用；`--force` 分支 `prepare-embedded-python.sh:73-76` 完整。
   - AC-6（干净 shell 输出 3.12.7）：PASS — 实测 `env -i PATH=/usr/bin:/bin` 输出 `3.12.7 (main, Oct 16 2024 ...)`。

3. **关键发现**
   - 两处主动声明的偏离均为合理工程修正（详见偏离判定）。
   - 未越权写入 `runtime-manifest.json`（grep 仅出现在注释中，无写文件操作）。
   - 未调用 brew / 系统 python3（grep 仅出现在 ADR 引用注释中）。
   - 首轮 SHA mismatch 真实触发，证明 Dev 没有空跑校验，是诚实标注的正面案例。

## 偏离判定

### 偏离 1：`verify-rpath.sh` 白名单扩展 `@loader_path/`、`@rpath/` — **PASS**
理由：这两种形式与 `@executable_path/` 同属 Mach-O 相对引用族（不含开发机绝对路径），保持 ADR-001/ADR-003 "可重定位、无路径泄露"原则不变；脚本注释 (verify-rpath.sh:9) 与 `is_allowed_path` (verify-rpath.sh:37-47) 仍明确拒绝 `/opt/homebrew/*`、`/usr/local/*`、`/Users/*` 等绝对路径。严格执行字面 AC-3 会让 PBS 产物 100% 失败，属必要工程修正。
**建议 Architect 在 ADR-001 同步补强 AC-3 的字面表述**，把 `@loader_path/` 与 `@rpath/` 纳入官方白名单。

### 偏离 2：`libpython3.12.dylib` install_name 占位符用 basename 匹配跳过 — **PASS（附加 MINOR 建议）**
理由：阅读 `verify-rpath.sh:60-74`，跳过逻辑有四重收紧：(a) 仅 `*.dylib` 文件触发；(b) `self_skipped` flag 保证一个 dylib 至多跳过 1 行；(c) 必须 `basename(dep) == basename(file)` 才跳过；(d) 跳过的是 install_name 行（otool -L 第二行），不影响后续真正的 dependency 行。

理论失误场景：某个 dylib A 凑巧依赖另一个同名但路径为 `/opt/homebrew/lib/A.dylib` 的库时会被吞掉——但 (1) 这要求 PBS 自带 dylib 拓扑里有同名同 basename 但不同绝对路径的真依赖，本批样本不存在；(2) 即使发生，跳过的也只会是第一行（install_name 默认就是第一条），真依赖通常排在 install_name 之后。**风险等级 MINOR**，可接受。

## 评分

| 维度 | 权重 | 分数 | 说明 |
|------|------|------|------|
| 功能正确性 | 25% | 5 | 6 条 AC 全部满足（含主动声明的偏离合理化）；首轮 SHA mismatch 真实触发校验，证明非空跑。 |
| 安全性 | 25% | 5 | 硬编码 URL + SHA256；下载到 `mktemp -d` 临时目录，校验失败立即退出，污染不进目标目录；trap 清理；平台守卫；无 brew / 无系统 python3。 |
| 代码质量 | 15% | 5 | 注释充分（ADR 引用 + 设计动因），常量集中顶部，变量全双引号，路径变量统一 `readonly`，错误信息一致。 |
| 测试覆盖 | 15% | 4 | 正常路径 + AC-1~AC-6 + SHA mismatch + 未知参数全部实测 PASS；`--force` 与非 arm64 守卫仅 `bash -n` + code review（标 PENDING-USER-MACHINE 是合理的，避免重复下载 70MB 与缺机器）。诚实标注。 |
| 架构一致性 | 10% | 5 | 严格遵守 ADR-001（PBS install_only）/ ADR-003（未运行 `venv --copies`、未触碰内部目录）；未越权写 `runtime-manifest.json`（task_002 之事）。 |
| 可维护性 | 10% | 5 | 注释含 ADR 编号锚点、占位符跳过的设计动因写在代码上方；脚本可独立调用，错误码区分清楚（exit 1 vs exit 2）。 |

**综合分：4.85/5**（加权计算：0.25×5 + 0.25×5 + 0.15×5 + 0.15×4 + 0.10×5 + 0.10×5 = 4.85）

## 总体判断

- [x] **PASS**

## 问题列表

### BLOCKER
无。

### MAJOR
无。

### MINOR（可选 / 后续 task 处理）

1. **basename 跳过逻辑可加防御性 sanity**
   - 代码位置：`verify-rpath.sh:71`
   - 建议：增加额外断言——被跳过的 dep 路径必须以 `/install/`、`@rpath/`、`@loader_path/` 或 `@executable_path/` 开头之一；若为 `/opt/`、`/Users/`、`/private/`、`/tmp/`、`/usr/local/` 等开发机路径，仍然报 VIOLATION 而非跳过。这能彻底消除"同名真依赖被吞"的理论盲点。
   - 非阻塞：本批 PBS 样本未触发，不进入 FIX 强制项。

2. **Architect 文档同步**（不是 Dev 责任）
   - 建议 Architect 在 ADR-001 / 后续 task input.md 中把 `@loader_path/` 与 `@rpath/` 写入 AC-3 白名单字面表述，避免下个 Reviewer 再次确认偏离。

3. **`--force` / 非 arm64 守卫实测**
   - 可在 task_006（build-macos-dmg 集成）的 CI 流水线中顺带覆盖；本 task 不强求。

## 修复指引

无（PASS，无 FIX 项）。
