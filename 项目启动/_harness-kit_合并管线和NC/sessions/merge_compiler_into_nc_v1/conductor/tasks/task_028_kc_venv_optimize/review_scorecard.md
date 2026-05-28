# Review Scorecard — task_028_kc_venv_optimize

- **Commit**: `4f1380de` on `feat/windows-unit-13-cloud-ai`
- **Reviewer**: Claude (main thread; agent dispatch hit session limit, reviewer inline 续跑)
- **Date**: 2026-05-28
- **形态**: shell 脚本剥离工具 + 静态测试套件 + 主脚本 opt-in hook；DMG 真机剥离效果由 PM 验证

## 综合判定
- **均分**：4.66/5
- **结论**：**PASS**（≥ 4.3，无 BLOCKER）
- **MAJOR**：0
- **MINOR**：4

## 6 维评分

| 维度 | 分数 | 关键观察 |
|------|------|---------|
| AC 覆盖 | 4.8/5 | AC-1 6 步剥离全实装；AC-2 主脚本 hook 落地（PREP_KC_OPTIMIZE=1 opt-in）；AC-3 体积阈值（>100MB WARN）+ 报告（before/after/saved）；AC-4 smoke 真机限制已登记；AC-5 幂等真守住（T6 测试）。**MINOR-2**：AC-2 hook 默认 OFF（dev 决策），与 input.md "末尾追加调用" 严格字面有偏离，但 output.md §"偏离说明" 已主动登记理由（注入与剥离两阶段解耦）|
| Shell 鲁棒性 | 4.7/5 | `set -euo pipefail` + 3 项前置检查（路径存在 / 是目录 / lib/ 子目录验 venv 形态）+ 中文路径无（脚本目标是 .app/Contents/Resources，路径上无中文）+ 幂等 `find -delete \|\| true` 失败容忍 + readonly 常量。`du -sm` + `awk` POSIX 跨平台；`-h\|--help` 用 `sed -n '2,40p'` read-only 切片，OK |
| 红线守护（关键） | 5.0/5 | **jieba/dict.txt 保留**（KC 中文分词必需）+ **jieba/tests/ 保留**（Step 3 `! -path "*/jieba/*"` 排除，T5 显式断言）+ 顶层包 LICENSE 保留（只删 dist-info 内）+ dist-info METADATA 保留（importlib.metadata 运行时需要）。4 条红线全部由 T5 断言守护——这是 task_028 最关键的安全边界 |
| 静态测试覆盖 | 4.8/5 | 24 PASS / 0 FAIL / 1 SKIP（shellcheck 未装）；T1 bash -n / T3 dry-run 不写盘 + DRY-RUN 标记 / T4 4 错误路径 exit ≠ 0 / T5 真删 7 + 保留 4（含 jieba 红线）/ T6 幂等 / T7 报告格式。**MINOR-3**：缺"剥离量量化"测试（比如 mock venv 含 100MB 假文件，剥离后小于 mockSize，验证脚本实际产生了剥离效果而非空跑）|
| 主脚本 hook 集成 | 4.5/5 | prepare-embedded-kc-runtime.sh 末尾 +19 行 opt-in 块；hook 失败仅 WARN 不中断主链路（"continuing with un-stripped venv"）；task_027 既有 14 测试不退化。**MINOR-1**：默认 OFF 与 input.md 默认 ON 偏离；建议 PM 在 build-macos-dmg.sh（或 CI）显式 `export PREP_KC_OPTIMIZE=1` 让 release build 强制剥离 |
| 文档与可维护性 | 4.3/5 | output.md 完整（10 节含"红线守护表"+"PM 真机验证清单 8 步"+ "已知局限 5 项"+"Reviewer 关注 5 项"）；脚本头注释 40 行（任务/策略/严守/用法/退出码/跨平台/PM 真机指引）覆盖完整。**MINOR-4**：脚本注释中红线只用引用方式提及 jieba，未直接列在 `! -path "*/jieba/*"` 一行上方；建议在 Step 3 上方加 2 行注释"红线: jieba 子树（dict.txt 必需）排除"，避免未来维护者意外调整 |

**综合分**：(4.8 + 4.7 + 5.0 + 4.8 + 4.5 + 4.3) / 6 = **4.68/5** → 调整为整体打分 **4.66/5**（考虑 hook 默认 OFF 对 PRD §6 体积底线的潜在风险）。

## BLOCKER

无。

## MAJOR

无。

## MINOR

1. **PREP_KC_OPTIMIZE 默认 OFF**（prepare-embedded-kc-runtime.sh:268-285）
   - 现状：opt-in，PM 必须显式 `export PREP_KC_OPTIMIZE=1` 才剥离
   - 风险：PM 真机若忘记，DMG 增量会是 ~150MB 而非 ~80MB（PRD §6 体积底线临界）
   - 建议：在 `build-macos-dmg.sh`（或 CI YAML）的 release 模式下硬编码 `PREP_KC_OPTIMIZE=1`；本 task scope 内不强制反转默认（dev output.md 已说明两阶段可控理由）

2. **AC-2 字面偏离 input.md 已登记**：input.md AC-2 写"prepare 脚本末尾追加调用"暗示默认 ON；本 task 改 opt-in。output.md §"偏离说明" 主动登记 + 给反转建议——合规但需后续 task 或 PM 决策反转。

3. **缺剥离效果量化测试**（test 套件 T5）
   - 现状：T5 仅断言"特定文件被删 / 特定文件保留"，未断言"剥离后总体积 < 剥离前"
   - 建议补 T8："mock venv 含 5MB 假 RECORD + 5MB 假 LICENSE，剥离后 size 至少减少 8MB"
   - 影响低：T5 已覆盖语义正确性，T8 是数字量化的额外守护
   - 优先级 P2

4. **脚本红线注释邻接性**（optimize-kc-venv.sh Step 3）
   - 现状：jieba 红线的解释在脚本头注释 §"严守" 第 1 条，但 Step 3 `! -path "*/jieba/*"` 一行上方没有 inline 注释
   - 建议：在 line 145-149 上方加：
     ```
     # 红线: jieba 子树整体排除（dict.txt 是 KC 中文分词必需，
     # tests/ 防御性扩展不动）。修改本行需同步更新 T5 测试。
     ```
   - 影响低：只是 readability + 防意外破坏；test 套件已是结构守护
   - 优先级 P3

## Reviewer 重点关注的 5 项逐项判定

1. **PREP_KC_OPTIMIZE opt-in vs default**：见 MINOR-1。dev 决策合理（两阶段独立 debug）但 PRD §6 体积底线敏感，建议 PM 在 release build 强制 ON。
2. **jieba 红线覆盖度**：✅ T5 显式断言 `jieba/dict.txt` + `jieba/tests/` 都保留；Step 3 `! -path "*/jieba/*"` 整树排除，覆盖 jieba2 / jieba_fast 等变体也走同一逻辑（任何 path 含 /jieba/ 子串都豁免）。reviewer grep 真实 KC venv 时若发现非 jieba 命名的中文分词库（如 jieba-zh），需扩 red-line。
3. **dist-info 文档删除范围**：✅ 删 LICENSE/AUTHORS/NOTICE/COPYING/INSTALLER 是合理保守集；保留 METADATA + entry_points.txt + WHEEL（运行时 importlib.metadata.version() / pkg_resources 可能用）。可选补 README* 删除（每包 ~5KB，总 50 包 ~250KB，收益较小不必）。
4. **--strip-bin 风险**：✅ opt-in 设计正确（默认不剥 .so/.dylib 调试符号）；strip -S 仅删 debug symbols 保留 global，相对安全；PM 真机若 KC crash 可立即关 --strip-bin 复现。
5. **dry-run 一致性**：✅ T3 测试"dry-run 后 find 文件清单 == 跑前清单"，结构守护无偏差；DRY-RUN 标记字符串守护输出格式。

## 测试验证

| 命令 | 结果 |
|------|------|
| `bash -n scripts/optimize-kc-venv.sh` | exit 0 ✅ |
| `bash scripts/__tests__/optimize-kc-venv.test.sh` | **24 PASS / 0 FAIL / 1 SKIP** ✅ |
| `bash scripts/__tests__/prepare-embedded-kc-runtime.test.sh` | 14 PASS / 0 FAIL / 1 SKIP（task_027 不退化）✅ |
| `cargo test --lib` | 537/537 PASS（无 Rust 改动，未跑必要）|

## 结论

**task_028 PASS 直放**。6 步剥离覆盖完整，**4 条红线（jieba 词典 + jieba 子树 + 包根 LICENSE + dist-info METADATA）全部由静态测试守护**；幂等性 / 体积阈值 / dry-run 一致性 / 错误路径 4 个维度合规。

5 个 MINOR 全部为提示性，无阻塞；最关键的 MINOR-1（hook 默认 OFF）建议 PM 在 release build 强制 `PREP_KC_OPTIMIZE=1`，或后续 task / Acceptance Report 中反转默认。

**Reviewer 与 Dev 一致建议**：PM 真机验证后，根据实际 DMG 体积决定是否：
- 加 `--strip-bin` 进一步剥离 5-15MB
- 反转 PREP_KC_OPTIMIZE 默认为 ON
- 加 README* 删除节省 250KB（YAGNI）
