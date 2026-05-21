# Review Scorecard — task_002_T0_contracts

## 审查思考过程

1. **Task 意图**：T0 是纯文档契约冻结，产出 `contracts.md` 作为 T1-T6 唯一引用基线，包含 IpcError JSON shape、11 错误码闭集、5 Tauri 命令签名（逐字复制 PRD §5.1）、`__ROOT__` 编解码契约、中文文案表。

2. **交付契约前置验证**：
   - [x] 测试结果存在且非空（grep 总命中 49，每 code ≥4）
   - [x] 自测验证矩阵存在且正常路径全部 PASS（10 行覆盖 AC-1～AC-4 + 边界 + 异常）
   - [x] 架构遵守声明已填写（4 项 ✓，无偏离）

3. **AC 检查结果**：
   - AC-1（4 节齐全）：✅ (a) A.1-A.4 / (b) B.1-B.3 / (c) C.1-C.4 / (d) D.1-D.2
   - AC-2（每个 code 的 details schema 明确）：✅ A.4 表 11 行 × `details schema` + `必填字段` 双列
   - AC-3（T1/T2/T3/T4 消费方核对清单）：✅ 文末 4 个小节，每条带具体小节号引用
   - AC-4（grep 11 code 各 ≥1）：✅ 总 49，最低 4
   - 红线 1（5 命令签名逐字与 PRD §5.1 一致）：✅ 命令名、参数顺序、参数名（`project_id`/`relative_path`/`target_relative_path`/`new_name`/`asset_id`）、参数类型（`String`/`u32`/`bool`）、返回类型（`WorkspaceFolderEntry`/`DeleteReport`/`Asset`/`u32`）全部字符级一致
   - 红线 2（`__ROOT__` 永不入 DB + debug_assert!）：✅ C.2 显式约束
   - 红线 3（错误码闭集 11 项，无 `E_DEPTH_LIMIT`/`E_CYCLE`）：✅ A.4 闭集断言 + 文档开头红线显式禁止
   - 文案全中文、无 i18n 框架痕迹：✅ D.1/D.2 全中文常量

4. **关键发现**：
   - 主动新增 B.3「错误码 ↔ 命令矩阵」对 T3 单测覆盖直接有益，未引入新决策、未越界。
   - A.3 `IpcErrorCode` Rust 端用 `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]` 桥接 TS 字面量，是兑现 ADR-001 序列化协议的必要最小补强。

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 25% | 5 | AC-1～AC-4 全部满足；5 命令签名逐字与 PRD §5.1 一致；11 错误码闭集严格无增减；`__ROOT__` 三向契约（入站/DB/出站）完整覆盖；`E_FOLDER_DIRTY` 用 `details.now` 渲染契约明示 |
| 安全性 | 25% | 5 | `debug_assert!(!path.contains("__ROOT__"))` 写路径硬约束明确（C.2）；闭集禁止 MVP 外 code；序列化协议 fallback 到 `E_INTERNAL` 防 JSON.parse 失败；details schema 表格化避免下游误用字段名 |
| 代码质量 | 15% | 5 | 文档结构清晰（(a)/(b)/(c)/(d) 四节 + 消费方清单）；A.4 表格化呈现 schema 易对照；D.2 参考实现代码段可直接复用；命名/术语一致（NFC/EXDEV/sentinel 等术语对齐 ADR） |
| 测试覆盖 | 20% | 4 | grep 自检 49 命中 + per-code ≥4 覆盖 AC-4；自测矩阵 10 行覆盖正常+边界+异常；扣 0.5 因为本文档无法自动断言"逐字一致"（依赖 Reviewer 人工 diff），建议未来引入脚本 diff PRD §5.1 文本块 |
| 架构一致性 | 10% | 5 | ADR-001（IpcError 序列化）、ADR-004（`__ROOT__` 编解码）、ADR-005（前端 camelCase wrapper）、ADR-010（`E_FOLDER_DIRTY {old, now}` 用 `now` 重弹）均在文档中体现；B.1 `DeleteReport` 与 PRD §5.1 注释一致；未引入计划外类型 |
| 可维护性 | 5% | 5 | 消费方核对清单（T1/T2/T3/T4）每条带具体小节号引用，下游 Dev 反查路径清晰；红线声明在文档开头集中，避免散落；ADR 反向引用完整 |

**综合分**：5×0.25 + 5×0.25 + 5×0.15 + 4×0.20 + 5×0.10 + 5×0.05 = **4.80/5**

## 总体判断

- [x] **PASS**
- [ ] FIX
- [ ] BLOCKER

## 问题列表

### BLOCKER（必须修复，否则不可能 PASS）

无。

### MAJOR（强烈建议修复）

无。

### MINOR（可选）

1. **A.4 表第 5 行 `E_PROTECTED_KIND` 的 `kind` 当前仅枚举 `'ai_organized'`**：若未来出现其他 protected kind（如 `system_reserved`），需回 T0 扩展。已在文档红线声明「新增 code 必须回 T0」，但 `kind` 字面量集合扩展不在该约束内。建议在 A.4 加一行注释「`kind` 字面量集合扩展同样需回 T0」。可选，非阻塞。

2. **D.1 `E_NAME_INVALID` 模板把 `reason` 标"仅日志用"**：但 `reason` 可枚举值（`has_slash`/`leading_dot`/`blank`/`too_long`/`other`）已明确，若前端将来希望按 reason 分支提示更精准文案（如「名称不能以 . 开头」），需回 T0 修订。当前固定文案兜底字符串已足够 MVP。可选。

3. **C.2 `debug_assert!` vs `assert!` 的选择**：output.md 已将此点列为「需 Reviewer 关注」。当前选择 `debug_assert!` 与 ADR-004 原文一致，release 构建静默；考虑到底线 6「严禁 `__ROOT__` 入 DB」的强度，未来若发现生产数据出现 `__ROOT__` 泄漏，应回 T0 升级为 `assert!` 或返 `E_INTERNAL`。**当前接受 debug_assert! 不阻塞**，因为：(a) ADR-004 原文如此；(b) 写路径有限（仅 `db::asset` INSERT/UPDATE 入口），单元/集成测试覆盖即可保证 debug 构建命中；(c) 升级 `assert!` 会带 release 崩溃风险，需独立评估。

## 给 Dev 的修复指引

无（PASS，无须修复）。下游 T1/T2/T3/T4 可直接以 `contracts.md` 为唯一基线开工。

---

## Reviewer 备注（不影响判定，仅供 Conductor 参考）

- **B.3 错误码 ↔ 命令矩阵**为 T0 主动新增，input.md 未要求。审查认为该表对 T3 单测覆盖直接有益（每命令的可达 code 集合一目了然），且未引入新决策，**接受保留**。
- **A.3 Rust 枚举 `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]`**：ADR-001 未明写此细节，但为兑现「TS code 字面量集合 = Rust 枚举序列化字符串」的字符级一致性，这是必要的最小补强，**接受**。
- 既有 `WorkspaceFolderEntry`/`Asset` 类型未在 T0 重新定义符合 input.md「不动 NCdesktop 仓库」边界；output.md 已列为已知局限 1。T3 实施时若发现字段缺口须回 T0 增补，**接受**。
