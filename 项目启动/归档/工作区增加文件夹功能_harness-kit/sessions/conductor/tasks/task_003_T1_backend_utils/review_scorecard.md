# Review Scorecard — task_003_T1_backend_utils

## 审查思考过程

### 1. Task 意图（复述）
搭建本期工作区文件夹管理的后端工具层：
- 引入 `trash` + `unicode-normalization` 依赖；
- 实现 `IpcError` enum / `IpcErrorCode` 11 项闭集 + `From<IpcError> for String`（ADR-001）；
- 在 `workspace.rs` 追加 `resolve_relative_path`（`__ROOT__` sentinel 入站归一）/ `validate_and_canonicalize`（越界拒）/ `validate_folder_name`（命名校验）；
- `nfc.rs`：NFC 归一 + 启动期自愈（macOS NFD→NFC）；
- `safe_rename.rs`：EXDEV-safe rename 两阶段 + cleanup_pending；
- `write_guard.rs`：项目级写通道 Mutex；
- 启动期挂 hook + `app.manage(WorkspaceWriteGuard::new())`。

### 2. 交付契约预检
- [x] 测试结果存在且非空（156 passed; 0 failed）
- [x] 自测验证矩阵存在，正常路径全部 PASS
- [x] 架构遵守声明已填写（含明确偏离说明：T3/T4 越权落地，已自爆）

### 3. AC 逐条核查

| AC | 描述 | 结果 |
|---|---|---|
| AC-1 | `Cargo.toml` 添 `trash = "5"` / `unicode-normalization = "0.1"` / dev `tempfile = "3"` | ✅ Cargo.toml:46-51 |
| AC-2 | `IpcError` + 11 项 code 闭集 + 11 工厂 + `From for String` + 单测 | ✅ `utils/ipc_error.rs`；11 项序列化字面量字符级断言通过 |
| AC-3 | `resolve_relative_path` / `validate_and_canonicalize` / `validate_folder_name` + 单测 | ✅ `workspace.rs:124/145/236`；14 单测覆盖 PRD §6.1 列举的 3 种越界 + 保留字 + 5 类非法名 |
| AC-4 | `nfc_normalize` / `nfc_eq` / `nfc_self_heal` / `nfc_heal_workspace` | ✅ `utils/nfc.rs`；5 单测 |
| AC-5 | `safe_rename` + `RenameOutcome` + EXDEV-18 + `remove_src_after_commit` + `cleanup_pending_scan` + `test_inject` | ✅ `utils/safe_rename.rs`；6 单测，含 src 保留断言 |
| AC-6 | `WorkspaceWriteGuard { Mutex<HashMap<String, Arc<Mutex<()>>>> }` + 3 单测（串行 / 并行 / 同 Arc） | ✅ `utils/write_guard.rs` |
| AC-7 | `setup` 中 `app.manage(WorkspaceWriteGuard::new())`；`startup.rs` 末尾、scheduler::recover 之前调 `nfc_heal_workspace` + `cleanup_pending_scan`，失败仅 log | ✅ `lib.rs:66` + `startup.rs:96-123`；外包 `catch_unwind` 双保险 |
| AC-8 | `cargo test --lib` 全绿 156 passed | ✅ |

### 4. 关键发现
1. **T0 §A.3 字面量字符级核验**：T1 选择 `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]` 简化路线，删除逐项 `rename`，但补一条字符级 11 项断言测试兜底。**字面量与 T0 §A.3 表完全一致**，契约不破。前一棒已在 enum doc-comment 写 warning，规避「variant 改名静默漂移」隐患（虽非 100% 防御，但合理）。可接受。
2. **`safe_rename.rs` 在 module 内为 `IpcError` 加 `fn attach_internal_hint`**：private impl 扩展，未污染 `ipc_error.rs` 公共 API。可接受，MINOR。
3. **越权 T3/T4 代码**（`commands/workspace_folders.rs` 4 写命令实现 + `lib.rs::invoke_handler!` 注册 + `tests/workspace_folders_integration.rs`）：本轮明确不评，留 task_005/task_006。
4. **`canonicalize_longest_prefix`** 对叶子未创建场景做了"先 canonicalize 已存在前缀再拼剩余段"，正确处理 create 时 symlink 越界检测；unix 单测验证。
5. **`utils/safe_name.rs`** 不在 T1 AC 范围，Dev 已自爆。其 6 项测试附带通过；本轮不评。

---

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 25% | 5 | 8 项 AC 全部命中；最长存在前缀 canonicalize / `__ROOT__` 入站归一 / EXDEV-18 raw_os_error 判定 / NFC 跳过已存在目标，逻辑均按 ADR 落地。156 单测全绿。 |
| 安全性 | 25% | 5 | `validate_and_canonicalize` 多层守卫（字符串拒 `..` / 绝对 / 非法 Component；canonicalize 后 `starts_with`；symlink 越界单测覆盖）；EXDEV 两阶段「先 copy → fsync → rename → 后删 src」严守 ADR-002；`From<IpcError> for String` 序列化失败有静态 JSON 兜底；启动 hook `catch_unwind` 防 panic。`assets.file_path` 防 `__ROOT__` 入库的 `debug_assert!` 留 T1 本身未直接消费但契约 §C.2 把检视点放在 T1/T3 共同（写命令归 T3，可在 T3 review 复检）。 |
| 代码质量 | 15% | 4 | 命名清晰、doc-comment 充足、模块职责单一；工厂函数 11 个均使用 camelCase details 键。两个瑕疵：(a) `safe_rename.rs:239-245` 把 `impl IpcError { fn attach_internal_hint }` 放在工具模块内属"反向 impl 扩展"，更合适的家是 `ipc_error.rs` 加 `pub(crate)`；(b) `nfc_self_heal` 函数末尾对子目录重新 join 一次 `nfc_normalize(&name)` 略冗余（已成 nfc 后可直接复用 target，少一次哈希）。均不阻塞。 |
| 测试覆盖 | 15% | 5 | T1 模块 33 项单测（ipc_error 4 / nfc 5 / safe_rename 6 / write_guard 3 / workspace::folder_utils 14 + 既有兼容 1）。覆盖：PRD §6.1 路径越界 3 例、保留字、5 类非法名；EXDEV 注入 + src 保留断言；同/异 project 写锁并发上限 1/2 + 耗时断言；NFC 自愈 NFD→NFC + 已存在跳过 + CJK idempotent；`cleanup_pending` 标记 + 孤立 tmp。AC-3 全部 3 例越界覆盖 ✅。 |
| 架构一致性 | 10% | 5 | 目录结构 / 函数签名 / 返回类型与 ADR-001~005 + T0 §A/§C 完全一致；未引入计划外依赖（trash + unicode-normalization + dev tempfile，全部 input.md 预估范围）。`resolve_relative_path` 是入站单点、`utils/write_guard.rs` 双层 Mutex、`RenameOutcome` 双 variant 均严格符 Architect 方案。 |
| 可维护性 | 10% | 4 | doc-comment 充分（每个 pub fn 标 ADR / T0 §X.Y 出处）；错误处理一致（IpcError 工厂统一入口）；测试中 `with_sandboxed_home` 用 unsafe `env::set_var` 是进程级状态，static `Mutex` 串行兜底已做，但 edition=2024 后会变 hard unsafe；Dev 已自爆。可接受。 |

**综合分**：
0.25×5 + 0.25×5 + 0.15×4 + 0.15×5 + 0.10×5 + 0.10×4 = 1.25 + 1.25 + 0.60 + 0.75 + 0.50 + 0.40 = **4.75 / 5**

---

## 总体判断

- [x] **PASS**
- [ ] FIX
- [ ] BLOCKER

无 BLOCKER；无 MAJOR；MINOR 3 项不阻塞。综合分 4.75 ≥ 3.5 阈值。

---

## 问题列表

### BLOCKER
（无）

### MAJOR
（无）

### MINOR（可选改进，**不阻塞** PASS）

1. **`impl IpcError { fn attach_internal_hint }` 反向扩展位置**
   - **位置**：`src-tauri/src/utils/safe_rename.rs:239-245`
   - **现状**：工具模块内为 `IpcError` 加 module-private 方法，技术上合法但拆开了"IpcError 行为定义"的可发现性。
   - **建议**：挪到 `src-tauri/src/utils/ipc_error.rs` 作 `pub(crate) fn attach_internal_hint(self, hint: &str) -> Self`，doc-comment 标"内部 hint 拼接，不改 code/details"。
   - **验证标准**：方法迁移后 `cargo test --lib` 仍全绿；调用方仅 import 路径变更。

2. **`nfc_self_heal` 末尾对子目录重复 NFC 归一**
   - **位置**：`src-tauri/src/utils/nfc.rs:60-64`
   - **现状**：`let final_path = project_root.join(nfc_normalize(&name));` 与上方 rename 目标重复计算一次 NFC。
   - **建议**：把 rename 后的 `target` 路径复用为 `final_path`，再行 is_dir 递归；同时 NFD/NFC 已合并场景也无需二次归一。
   - **验证标准**：`nfc_self_heal_renames_nfd_dir` 与 `nfc_idempotent_for_cjk` 仍通过。

3. **`lib.rs:66` 注释笔误标 `task_005 T3`，实为 T1 AC-7**
   - **位置**：`src-tauri/src/lib.rs:66`
   - **建议**：注释改为 `task_003 T1 AC-7`；可在 T3 review 顺手修，不必本轮单独修。
   - **验证标准**：注释一致即可，无功能变更。

---

## 给 Reviewer 给 Conductor 的备注

1. **越权范围明确不计入本轮**：T1 实际仓库中已含 T3 的 4 写命令实现 + T4 的 2 个集成测试 + `invoke_handler!` 注册。本评分**仅评 T1 AC 范围**（utils/ipc_error / utils/nfc / utils/safe_rename / utils/write_guard / utils/safe_name 跳过 / workspace.rs 追加 / startup.rs hook / lib.rs manage Guard / Cargo.toml）。T3/T4 越权部分留各自 reviewer 评。
2. **T0 §A.3 二选一**：T1 Dev 保留 `rename_all = "SCREAMING_SNAKE_CASE"`，删除逐项 `#[serde(rename)]`，配字符级 11 项断言测试兜底。**字面量字符级一致**，契约不破。已确认接受。
3. **PRD §6.1 单测列表 T1 范围内全部覆盖**：路径越界 3 例 ✅ / 保留字 `organized` ✅ / NFC 归一同名 ✅；`ai_organized` 4 类写 + SQL 前缀边界属 T3 范围。

---

## 自检清单
- [x] 逐条检查了 AC 满足情况
- [x] 检查了 session_context.md 的领域审查重点（IpcError 字面量 / canonicalize 越界 / NFC 自愈 / 写通道锁 / 保留字 / Cargo / 测试质量）
- [x] 每个 MINOR 给出了位置 + 建议 + 验证标准
- [x] 评分诚实（功能/安全/测试/架构 5 分；代码质量/可维护性 4 分，含具体扣分点）
- [x] PASS 无需修复指引
