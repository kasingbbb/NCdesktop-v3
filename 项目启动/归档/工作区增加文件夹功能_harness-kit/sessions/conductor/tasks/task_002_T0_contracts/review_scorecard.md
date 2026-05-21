# Review Scorecard — task_002_T0_contracts

## 审查思考过程

### 1. Task 意图
冻结本期 T1-T6 共用的契约基线 **`contracts.md`**：IpcError JSON shape（双向）/ 11 项错误码闭集 + details schema / 5 个新 Tauri 命令签名 + 前端 wrapper + DeleteReport / `__ROOT__` 编解码三处单点 / 错误码↔中文文案表 / 变更管控红线。该 task 是文档型，无产品代码改动；测试维度按"契约能否被 T1-T6 当唯一 spec 引用且无歧义"评估。

### 2. 审查前验证（handoff_contracts §3）
- [x] output.md 字段齐全：实现摘要 / 修改的文件 / 架构遵守声明 / 测试命令 / 测试结果 / 自测验证矩阵 / 已知局限 / 需要 Reviewer 特别关注的地方
- [x] 自测验证矩阵存在，正常路径 7/7 PASS，边界 3/3 PASS，异常 2/2 PASS
- [x] 架构遵守声明已填写，4 项 ✅ + 无偏离
- → **通过审查前验证**，进入实质性审查

### 3. AC 逐条检查

| AC | 内容 | 结论 | 证据 |
|---|---|---|---|
| AC-1 | 文档落盘 contracts.md + output.md | ✅ | `ls` 确认两文件存在；output.md 按 handoff_contracts §3 字段齐全 |
| AC-2 | IpcError shape 双向 + 11 项 code 闭集 + details schema + 禁 E_DEPTH_LIMIT/E_CYCLE | ✅ | §A.2 TS / §A.3 Rust（含 `From<IpcError> for String` 兜底 JSON）/ §A.3 字面量对照表 11 行 / §A.4 details schema 11 行含「必填」标注 / 顶部红线第 4 条 + §A.4「不变量」双重明示禁新增码 |
| AC-3 | 5 命令签名逐字 + 前端 wrapper + DeleteReport | ✅ | §B.1 5 个 Rust 签名字符级匹配 PRD §5.1（参数名、类型、返回类型逐字一致）；§B.2 前端 camelCase wrapper 5 个 + invoke payload key 列表；DeleteReport `{ trashed: u32 }` 在 §B.1 |
| AC-4 | `__ROOT__` 三处单点 + assets 写路径 `debug_assert!` | ✅ | §C.1 resolve_relative_path 代码片段 / §C.2 `debug_assert!(!path.contains("__ROOT__"))` 含 panic 文案 / §C.3 list 首行 + 4 命令根目录场景仍返 `__ROOT__` / §C.4 Reviewer checklist 4 项 |
| AC-5 | 文案表 11 项 + details 渲染规则 + 前端唯一来源 | ✅ | §D 11 行模板逐项给出；渲染规则 4 条；E_FOLDER_DIRTY 明示「必须用 details.now 渲染」；§D 单测约束声明 |
| AC-6 | 红线声明 | ✅ | 文件顶部「红线声明（必读）」4 条 + §E 变更管控四步流程 |

**AC 全数通过。**

### 4. 安全/领域审查重点逐项对照（session_context §6）

| 检查项 | 契约是否覆盖 | 备注 |
|---|---|---|
| 写命令 canonicalize + project_workspace_dir 内 | ✅ §A.4 #4 E_PATH_ESCAPE 触发来源明示 | 强约束传达到 T1 |
| ai_organized 前后端双拦 | ✅ §A.4 #5 E_PROTECTED_KIND 明示 "direct invoke 也走此分支"；action 闭集 5 项 | 强 |
| rename/move 同事务 SQL 前缀替换 | △ 契约文档不展开实现 | T0 范围之外，归 T3 |
| 删除走 trash，禁 remove_dir_all | ✅ §A.4 #9 E_TRASH_FAILED 触发来源明示 "trash::delete 成功+复检 still_exists" | 强 |
| 命名校验闭集 | ✅ §A.4 #1 E_NAME_INVALID reason 5 项闭集 / #3 reserved=`organized` | 强 |
| Win/Linux 删除返明确错 | ✅ §A.4 #8 E_PLATFORM_UNSUPPORTED `feature=trash, platform=windows\|linux\|unknown` | 强 |

### 5. 桥接摘要 10 条底线契约相关项

- #6 `__ROOT__` 永不入 DB / `debug_assert!`：§C 完整覆盖 ✅
- #9 命名校验后端权威（禁 `/ \ :` / `.` 开头 / 同级同名 / 保留字 organized）：§A.4 #1/#2/#3 完整覆盖 ✅
- #10 错误统一 IpcError JSON / 后端 message 仅日志：§A.1 + §A.2 message 注释「仅日志/上报」+ §D 渲染规则第 2 条 ✅

### 6. 关键发现
1. **契约质量高、可被 T1-T6 直接当 spec 引用**：5 命令签名、IpcError shape、11 项 code 字面量、details schema、文案表、`__ROOT__` 三处单点全部具体到字段名 / 字面量级别，无歧义。
2. **§A.3 Rust enum 有一处冗余写法**：`#[serde(rename_all = "SCREAMING_SNAKE_CASE")]` 与逐项 `#[serde(rename = "...")]` 同时出现；注释已说明意图，但 T1 实现时存在被照抄 → 冗余/混淆的小风险（MINOR）。
3. **§F 与既有代码对齐说明妥善**：明示「既有代码字段若与本契约不一致，以本契约为准；T1/T2/T3 实现时直接覆写既有 stub」——符合 PM 裁决「既有代码视作不存在」，不会反过来束缚 Dev。

---

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 25% | 5 | AC-1~AC-6 全数 ✅；契约覆盖度完整，字符级精度 |
| 安全性 | 25% | 5 | 桥接摘要安全相关底线（#6/#9/#10）全部以契约形式落地；`__ROOT__` 三处单点 + debug_assert! 明示；E_PATH_ESCAPE details 不暴露绝对路径防泄漏；ai_organized 双拦传达到位 |
| 代码质量 | 15% | 4 | 表格规范、字面量对照清晰、代码示例可读；§A.3 `rename_all` + 逐项 `rename` 冗余写法易引起 T1 照抄混淆 |
| 测试覆盖 | 20% | 5 | 文档型 task — 按"能否被下游当唯一 spec"评估：11 项 code 字面量对照表 + details schema 必填标注 + 文案表 + Reviewer checklist + T2 snapshot 单测约束，可被 T1-T6 直接引用且无歧义 |
| 架构一致性 | 10% | 5 | 逐字搬运 PRD §5.1 / §4.3；ADR-001/ADR-004/ADR-008 全部映射；既有 src/types/workspace.ts 与本契约一致 |
| 可维护性 | 5% | 5 | 顶部红线 + §E 变更管控四步流程清晰；§F 列出已存在文件与本契约的对齐关系，便于后续追踪 |

**加权综合分**：
- 0.25×5 + 0.25×5 + 0.15×4 + 0.20×5 + 0.10×5 + 0.05×5
- = 1.25 + 1.25 + 0.60 + 1.00 + 0.50 + 0.25
- = **4.85 / 5**

---

## 总体判断

- [x] **PASS**

无 BLOCKER、无 MAJOR；2 个 MINOR 不影响下游消费。综合分 4.85/5 远超 PASS 门槛 (3.5)。

---

## 问题列表

### BLOCKER（必须修复，否则不可能 PASS）
（无）

### MAJOR（强烈建议修复）
（无）

### MINOR（可选，不阻塞 PASS；建议在后续轮次顺带优化）

1. **§A.3 Rust enum `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]` 与逐项 `#[serde(rename = "...")]` 同时出现**
   - **代码位置**：contracts.md §A.3 第 51 行附近
   - **现象**：注释「形式说明；为避免 Rust 标识符限制，逐项 rename」承认两者并存意图，但 `rename_all` 在所有 variant 已显式 `rename` 的情况下完全冗余。T1 Dev 若照抄可能感到困惑或意外触发 lint。
   - **建议**：要么删除 `#[serde(rename_all = ...)]` 行（推荐），要么把它降级为代码块上方的散文说明「我们使用 SCREAMING_SNAKE_CASE 风格，逐项 rename 见下表」。
   - **是否阻塞**：否；T1 Reviewer 复核 enum 时也能拦下。

2. **§A.4 #6 `E_NOT_FOUND.details.identifier` 当 `target=folder` 且 relativePath 为根时的字面量未明示**
   - **代码位置**：contracts.md §A.4 第 112 行（E_NOT_FOUND 行）
   - **现象**：备注「folder = relativePath；asset = assetId」未说清根目录场景下 identifier 是空串 `""` 还是 `"__ROOT__"`。根据 §C 三处单点逻辑，**入站归一后** identifier 应取归一后空串，但出站给前端上报时是否仍用 `""` 表示根？
   - **建议**：在 #6 行 details schema 后追加一句「`identifier` 取入站归一后的 relativePath，根级 = `""`，不得用 `"__ROOT__"`」对齐 §A.4 #2 `parentRelativePath` 的口径。
   - **是否阻塞**：否；E_NOT_FOUND 文案不依赖 identifier 渲染（§D #6 「identifier 仅上报」），仅日志/上报字段歧义。

3. **§C.2 提到「既有 `delete_asset` / import 入口 T1/T3 检视时各加一处 `debug_assert!`」**
   - **代码位置**：contracts.md §C.2 + output.md「需要 Reviewer 特别关注的地方」#3
   - **现象**：PM 裁决"既有代码视作不存在"，但这里把"既有入口"作为 4 处之一计入。该写法不会束缚 Dev（仍是新加防御性 assert），但与 §F「以本契约为准，直接覆写既有 stub」口径上略有张力。
   - **建议**：把 §C.2「4 处入口」表述放宽为「所有 `assets` INSERT/UPDATE file_path 入口」，避免与既有代码做硬性绑定计数。
   - **是否阻塞**：否；T1/T3 Reviewer 用「所有写入口都加 assert」的语义检查即可，不依赖 4 这个数字。

---

## 给 Dev 的修复指引

**判定为 PASS**，无强制修复项。

如 PM/Conductor 决定顺手清理上述 MINOR，遵循以下约束：
- 仅在 `contracts.md` 内修改上述 3 处；不得连带改字面量 / details schema / 文案 / 命令签名。
- 修改后在文件顶部红线下方加一行「最后修订：YYYY-MM-DD（仅文字澄清，不动契约语义）」。
- 不需要重跑测试（无测试），人工核对修改未影响下游引用语义即可。

---

## Reviewer 自检
- [x] 逐条 AC 检查（6/6 通过）
- [x] 逐条对照 session_context §6 领域审查重点（6/6 覆盖到位）
- [x] 逐条对照桥接摘要 10 条底线契约相关项（#6/#9/#10 全覆盖）
- [x] 字面量对照表 11 行逐项核对 PRD §4.3 ✅
- [x] 5 命令签名逐字核对 PRD §5.1 ✅
- [x] 评分诚实：契约质量高 → 5 分维度多，§A.3 冗余写法 → 代码质量扣 1 分
- [x] MINOR 均给出位置 + 建议 + 是否阻塞
