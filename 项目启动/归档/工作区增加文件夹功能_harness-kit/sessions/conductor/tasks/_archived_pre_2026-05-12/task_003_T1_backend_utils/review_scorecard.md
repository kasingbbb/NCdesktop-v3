# Review Scorecard — task_003_T1_backend_utils

## 审查思考过程

1. **Task 意图**：为工作区文件夹管理后端搭建工具层 — `IpcError` 11 项闭集、NFC 归一与启动期自愈、EXDEV-safe rename、项目级写通道串行锁、路径/命名校验工具；不注册任何 `#[tauri::command]`（命令归 T3）。
2. **AC 检查结果**：
   - AC-1 IpcError serde + 11 code + Into<String> ✅（4 测试 PASS，逐项 rename 锁定字面量与 contracts.md §A.2 一致）
   - AC-2 nfc_normalize + nfc_heal_workspace ✅（5 测试 PASS，含 NFD→NFC 真实 rename）
   - AC-3 safe_rename 同卷 + EXDEV copy-first + fsync ✅（6 测试 PASS，mock EXDEV 路径 + raw errno 18）
   - AC-4 WriteGuard 同/异 project 并发 ✅（3 测试 PASS，含时间断言 + Arc::ptr_eq）
   - AC-5 validate_and_canonicalize 4 子条 ✅（含 symlink 越界真实 fixture）
   - AC-6 validate_folder_name 5 类非法 ✅（含 NFD 通过 + 长度 + 保留字）
   - AC-7 Cargo.toml +2 deps，cargo build 成功 ✅（仅 trash="5"、unicode-normalization="0.1"，未顺手加）
   - AC-8 bootstrap 挂 nfc_heal + cleanup_pending_scan + catch_unwind ✅
3. **审查前验证**：测试结果非空（148 PASS / 0 FAIL）、自测矩阵全 PASS、架构遵守声明 8 项全勾 ✅
4. **关键发现**：
   - `IpcErrorCode` 11 项 `#[serde(rename = "E_...")]` 字面量逐项与 contracts.md §A.2 字符级一致；`details:None` 时 skip 序列化，符合 TS 类型 `details?` 可选语义。
   - `safe_rename` 顺序严格为 `copy → fsync → rename(tmp→final) → 返回 pending`，**绝不在函数内删 src**；EXDEV 检测使用 `raw_os_error()==18` + `kind()` 字符串兜底，避开 `ErrorKind::CrossesDevices` 稳定性差异。
   - `WorkspaceWriteGuard::lock_for` 返 `Arc<Mutex<()>>` 而非 owned guard：取舍合理（避免 `OwnedMutexGuard` 的 unsafe），调用方两行写法已在模块文档中明示。
   - `validate_and_canonicalize` 用「最长存在前缀 canonicalize + 拼剩余段」处理叶子未创建场景，对 symlink 越界仍能正确拒绝（已构造真实 symlink fixture 验证）。
   - `nfc_self_heal` 目标已存在时仅 `log::warn` 跳过，从不覆写。
   - `nfc_heal_workspace` / `cleanup_pending_scan` 均被 `catch_unwind(AssertUnwindSafe(...))` 包裹，bootstrap 不可阻塞。
   - 未注册任何 `#[tauri::command]`，未引入计划外依赖。

## 评分

| 维度 | 权重 | 分数 | 说明 |
|------|------|------|------|
| 功能正确性 | 25% | 5 | 8 个 AC 全部 ✅；28 个新增单测全 PASS；边界（NFC NFD/CJK、symlink、errno=18、长度 300、空白）均覆盖 |
| 安全性 | 25% | 5 | 路径越界三件套（`..` / 绝对 / symlink）严格拒；NFC 入站归一防鬼影；`__ROOT__` sentinel 单点消费；命名校验在 NFC 后；启动 hook 非阻塞 |
| 代码质量 | 15% | 4 | 注释清晰、模块边界明确；唯一瑕疵：`impl IpcError::attach_internal_hint` 写在 `safe_rename.rs` 内（Dev 自承），应归 `ipc_error.rs` 单点 impl |
| 测试覆盖 | 20% | 5 | 28 个测试覆盖正常/边界/异常三类；EXDEV 用静态 mutex 注入 mock；并发用时间断言；symlink 用真实 fixture |
| 架构一致性 | 10% | 5 | 严格遵守 ADR-001/002/003/004/005/008；闭集 11 项；未注册命令；未引入计划外依赖 |
| 可维护性 | 5% | 4 | `unsafe set_var("HOME")` 测试 sandbox 用全局锁串行化合理；`cleanup_pending` 24h 阈值未实现但启动期 hook 上下文安全（已自承） |

**综合分：4.85/5**（加权：0.25×5 + 0.25×5 + 0.15×4 + 0.20×5 + 0.10×5 + 0.05×4 = 4.85）

## 总体判断

- [x] **PASS**

## 问题列表

### BLOCKER
无。

### MAJOR
无。

### MINOR（可选，不影响 PASS；建议在后续 task 顺手处理）

1. **`impl IpcError::attach_internal_hint` 放错位置**：定义在 `utils/safe_rename.rs:239-245` 而非 `ipc_error.rs`。Rust 语法合法，但破坏「IpcError 所有方法单点」可读性。
   - 建议：移到 `ipc_error.rs` 作为 `pub(crate) fn`。
2. **保留字大小写策略未契约化**：`validate_folder_name("Organized")` 当前通过。Dev 已写测试锁定，且在 output.md 自承询问。
   - 建议：在 contracts.md §A.4 或后续 T3 task input 明确"大小写敏感/不敏感"，目前实现可接受。
3. **`cleanup_pending_scan` 24h 阈值未实现**（ADR-002 提到）：当前见到 `.cleanup_pending` / `.cross_device.tmp` 即清。
   - 风险评估：仅在 bootstrap 时点调用，T3 写命令在 startup 之后才会运行，**当前调用上下文安全**；若未来在 runtime 也调用本函数则需补 mtime 检查。
4. **`canonicalize_longest_prefix` 重复创建 workspace root**：`validate_and_canonicalize` 内 `create_dir_all(&root)` 是 side effect，在纯校验函数中不理想。
   - 建议：让 caller（T3 命令）确保 root 存在，或拆出 `ensure_workspace_root()` 单独调用。当前实现幂等无害。
5. **`nfc_self_heal` 递归路径计算冗余**：第 61 行 `nfc_normalize(&name)` 再算一次（已在第 40 行算过）。
   - 建议：复用上面的 `nfc` 变量。

## 给 Dev 的修复指引

无需修复。**直接 PASS，进入 task_004**。MINOR 可在 T3 实现时顺手清理，不强制本 task 返工。

## 审查者总结

T1 工具层交付质量优秀：契约对齐、测试矩阵完整、安全底线（NFC + 路径越界 + EXDEV）全覆盖、启动期非阻塞、未越权注册命令。Dev 在 output.md 中主动标注的 5 个"已知局限"展现了良好的工程自省。综合分 4.85/5，判定 **PASS**。
