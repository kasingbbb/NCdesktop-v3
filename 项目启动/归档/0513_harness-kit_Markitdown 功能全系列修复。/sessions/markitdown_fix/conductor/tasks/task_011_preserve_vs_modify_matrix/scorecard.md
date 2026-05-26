# Review Scorecard — task_011_preserve_vs_modify_matrix

## 审查思考过程

1. **Task 意图**：对 `markitdown.rs` 当前已稳定的"易被后续 task 误重构"行为做"保留 / 修改"二维矩阵显式声明，落地为：
   - 独立文档 `preserve_matrix.md`（决策表 + 联调声明 + Reviewer Checklist）
   - `markitdown.rs` 源码锚点注释（`// task_011 preserve:` / `// task_011 modify:`）
   - 单测补强保留行为（超时归类 / image 空回退 / 版本缓存）
   - **不动任何业务逻辑**（注释 + 测试段 only）

2. **AC 检查结果**：
   - AC-1 矩阵 ≥ 6 项 → 10 项 ✅，input.md 字面 6 项全员在表（#1 90s / #2 image fallback / #3 版本缓存 / #4 classify_output 替换 / #5 python_candidates / #6 SUPPORTED_MIME_TYPES）；
   - AC-2 注释引用：`grep -nE 'task_011 (preserve|modify):'` 命中 **10 行**（9 preserve + 1 modify），与 dev 自报一致；总行数 10 ≤ 30 预算 ✅；
   - AC-3 单测 4 个全部 `test result: ok` ✅；
   - AC-4 联调：image fallback 正向 + 非 image 反向双侧测试均 PASS，证明 `classify_output → EOutputEmpty → is_image gate → fallback` 链路不被污染 ✅；
   - AC-5 Reviewer Checklist：`preserve_matrix.md:50-61` 6 条 checklist 明确，含 grep gate / 字面常量 / `extractor_type` 字面 / 后台读线程结构 ✅。

3. **关键发现**：
   - 注释行数 10 行，恰好 "9 preserve + 1 modify" 的最小集，无冗余；
   - 矩阵从 6 项扩到 10 项（额外 #7 pipe drain / #8 audio debug_assert / #9 error_class 前缀 / #10 runtime_check 短路）都是真实在源码中存在且"易被顺手简化"的关键行为，扩展合理；
   - 业务逻辑零改动经 `grep -n "task_011" src/extraction/extractors/markitdown.rs` + 阅读 `markitdown.rs:200-300` 关键区段确认 —— `classify_output` 调用点 / image fallback 分支 / `python_candidates` / `probe_markitdown_version` 函数体字符无修改；
   - 4 测全部通过 `cargo test --lib` 实测 = **219 passed; 0 failed**（baseline 215 + 4 新测），数字与 dev 自报一致；
   - 版本缓存测改测"缓存状态机不变量"而非真子进程调用计数，dev 在 output.md 已知局限段诚实声明，且语义等价（gate 表达式与 markitdown.rs:213 真实使用一致），属合理工程取舍。

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 25% | 5 | AC-1~5 全过；矩阵 10 项含 input.md 6 项；注释 grep 10 命中；4 测全过。 |
| 安全性 | 25% | 5 | 纯注释 + 测试段，无 IPC / 路径 / 反序列化新面；fake python 用 `/usr/bin/true` 绝对路径，无 PATH 污染；mock 用临时文件路径（不真读不真写）。 |
| 代码质量 | 15% | 5 | 注释行均含"行为简述 + preserve_matrix.md #N"反向锚点；测试名 `task_011_*_<行为>` 命名清晰；版本缓存测明确分 (a)/(b)/(c) 三阶段断言不变量。 |
| 测试覆盖 | 15% | 5 | 超时 / image fallback 正向 / image fallback 反向 / 版本缓存 4 测全覆盖；AC-4 反向覆盖（`task_011_non_image_empty_output_does_not_fallback`）超出 input.md 字面要求。 |
| 架构一致性 | 10% | 5 | 未引入新 extractor / 新分类器 / 新依赖（拒绝 `serial_test` 改测状态机不变量）；ADR-007 `conversion_meta.failure_code` 未触及；Debate Layer 2 R-③ 由矩阵 #2 / #4 显式锁定。 |
| 可维护性 | 10% | 5 | preserve_matrix.md 作为后续 Reviewer Checklist 单一入口；源码锚点 + 矩阵编号双向引用（#1 ↔ 9+1 注释）；output.md 列出 5 条 Reviewer 关注点 + 3 条已知局限，文档自包含。 |

**综合分：5.0/5**（加权计算：0.25·5 + 0.25·5 + 0.15·5 + 0.15·5 + 0.10·5 + 0.10·5 = 5.00）

## 总体判断

- [x] **PASS**
- [ ] FIX
- [ ] BLOCKER

## 红线核查

| 红线 | 状态 |
|---|---|
| 修改 markitdown.rs 业务逻辑（extract / classify_output / image fallback / python_candidates / probe_markitdown_version 函数体） | **未触**（仅注释 + tests 段追加） |
| 修改 task_007/008/009/010 PASS 的其他 Rust 文件 | **未触**（grep `task_011` 仅命中 markitdown.rs） |
| 修改 db/migration.rs / db/conversion_meta.rs / db/asset.rs | **未触** |
| 修改 task_004~006 scripts/ | **未触** |
| 引入新 extractor / 分类器（H6） | **未引入** |
| 注释总行数 > 30 | **10 行**，远低于 30 阈值 |
| cargo test --lib 退步 | **219 passed**（baseline 215 + 4 新测），无退步 |

红线全过 **YES**。

## 4 关注点结论

1. **矩阵 10 项是否合理超出 6 项？**
   合理。额外 4 项均来自 dev 真实 grep（#7 pipe drain 后台读线程 / #8 audio debug_assert / #9 error_class 前缀 / #10 runtime_check 入口短路），都是 task_007/008/010 通过后稳定形态的关键支柱，且历史回归案例（macOS pipe buffer 死锁 / runtime self-check 失败仍起子进程 / scheduler 无法分类错误）都映射到这些项。扩展显著强化 Reviewer Checklist 覆盖。

2. **注释行数 ≤ 30 约束**
   实测 `grep -cE 'task_011 (preserve|modify):' = 10`，远低于 30 行预算。9 个 preserve + 1 个 modify 是该矩阵在源码侧的最小完备投影（10 项中 #4 走 modify，其余 9 项走 preserve；#5 因体现为 task_007 入口短路 + 候选顺序两段，注释挂在 #10 + 后置 `python_candidates` 段，不重复占行）。合规。

3. **超时测的 95s 实现方式**
   dev 选择直接调 `classify_output("", None, Duration::from_secs(95))` + 第二轮 `Some(137)` —— **不真起 95s sleep**，CI 时间预算保护到位。归类语义与 markitdown.rs:253-257 的 `io::ErrorKind::TimedOut → ETimeout90s` 映射共享同一 `classify_output` 函数，覆盖路径等价。output.md "已知局限 #2" 已诚实声明此取舍。合理。

4. **image fallback 测的 mock 方式**
   dev 用 `/usr/bin/true`（macOS / Linux 默认存在；带 `/bin/true` 防御性 fallback；两者都缺失则 `eprintln + return` 跳过 —— 不会假阳/假阴）。`/usr/bin/true` 忽略所有参数立即 `exit 0 + stdout=""`，恰好触发 `classify_output → EOutputEmpty → had_empty_success = true`，配合 `.png` 扩展名命中 `is_image && had_empty_success` 进入 fallback 分支。语义准确，在 CI Linux / macOS 环境下可靠（output.md "需要 Reviewer 特别关注 #2" 已声明 Windows CI 兼容性边界）。合理。

## cargo test 实测

```
test result: ok. 219 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.41s
```

- baseline（task_009/010 PASS 后）= 215
- task_011 新增 = 4（超时 / image fallback / 非 image 反例 / 版本缓存）
- 合计 215 + 4 = **219** ✓
- 与 dev 自报数字一致。

## 问题列表

### BLOCKER（必须修复，否则不可能 PASS）

**无**。

### MAJOR（强烈建议修复）

**无**。

### MINOR（可选 / 不影响 PASS）

1. **版本缓存测的 RwLock poison 行为未显式 cover**：若 `cached_version.read()` 返回 `Err(_)`（已 poison），dev 测的 gate 表达式 `is_none_or` 会落到 None 分支 → gate 返回 true。当前测不会触发 poison（无并发 panic），但未来若有 panic 注入测试，需额外 case。**评估**：不必修，超出 task_011 范围（属 task_008 / 未来并发硬化 task）。
2. **Reviewer Checklist 中 grep 命令未自带 `-c`**：`preserve_matrix.md:54` "grep 计数应 ≥ 4" 与实际 9 命中差距大，若后续 task 删 1-2 处 preserve 注释仍 ≥ 4 → checklist 误漏。**评估**：不必修，input.md AC-2 字面"每项保留行为加注释" + dev 注释 9 处覆盖 9 项保留行为，1:1 对应，后续 Reviewer 应核对每项 preserve_matrix.md #N 是否仍有对应锚点（grep `#N`），而非仅按计数。建议未来 Reviewer 用 `grep -oE 'preserve_matrix.md #[0-9]+' | sort -u | wc -l` 替代裸计数。

## 给 Dev 的修复指引

**无修复要求**。task_011 PASS，进入下一 task。

---

## scorecard 落盘

YES — `sessions/markitdown_fix/conductor/tasks/task_011_preserve_vs_modify_matrix/scorecard.md`。
