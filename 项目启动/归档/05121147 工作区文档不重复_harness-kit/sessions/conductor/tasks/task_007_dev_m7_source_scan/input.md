# Task 输入 — task_007_dev_m7_source_scan

## 目标
应用启动期一次性遍历所有 root assets，stat 各自 `file_path`；不存在的 → 加入内存态 `SourceMissingSet`，并 emit `notecapt/source-scan-finished` 让前端 invalidate list。

## 前置条件
- 依赖 task：task_003（list_root_assets / WorkspaceAssetView 已能消费 `source_missing` 字段）
- 必须先存在的文件/接口：
  - `db::asset::list_root_assets`（task_003）
  - `tauri::async_runtime::spawn`
  - `app.manage` 机制（与现有 `Database` / `PipelineScheduler` 注册同源）

## 验收标准（AC）
1. **AC-1**：新建 `src-tauri/src/source_scan.rs`，导出：
   ```rust
   pub struct SourceMissingSet {
       inner: std::sync::RwLock<std::collections::HashSet<String>>,
   }
   impl SourceMissingSet {
       pub fn new() -> Self;
       pub fn contains(&self, asset_id: &str) -> bool;
       pub fn insert(&self, asset_id: String);
       pub fn remove(&self, asset_id: &str);
       pub fn snapshot(&self) -> Vec<String>;
   }
   pub fn scan_all_projects(app: &AppHandle) -> Result<usize, String>;
   ```
2. **AC-2**：`scan_all_projects` 流程：
   - 取 `Database` state，遍历所有 library / project；对每个 project 调 `db::asset::list_root_assets`（**仅 root**，不扫 derivative）；
   - `Path::new(&asset.file_path).exists() == false` → `SourceMissingSet::insert(asset.id)`
   - emit `notecapt/source-scan-finished` 携带 `{ scanned: usize, missing: usize }`
   - 返回 missing 数
3. **AC-3**：在 `lib.rs::setup` 中：
   - `app.manage(source_scan::SourceMissingSet::new());`
   - `tauri::async_runtime::spawn(async move { let _ = source_scan::scan_all_projects(&app_handle); });` （**不**阻塞 setup）
4. **AC-4**：`commands::asset::get_assets`（由 task_003 已切到 list_root_assets）注入 `State<SourceMissingSet>`，在派生 state 时设置 `source_missing` 字段。
5. **AC-5**：单测 `source_scan::tests::detects_missing_file`：
   - 构造内存 DB（用 tempfile 临时目录）+ 2 个 root（一个文件真实存在，一个被 unlink）
   - 调用 `scan_all_projects` 等价的纯函数版（`scan_with_conn(conn, missing_set, project_id)`）
   - 断言 missing_set 包含被 unlink 的 asset.id，不含真实存在的
6. **AC-6**：（可选 debug 命令）`source_scan_get_missing() -> Vec<String>` 仅 `cfg(debug_assertions)` 注册，便于集成测试断言。
7. **AC-7**：`cargo test -p app_lib --lib source_scan` 通过。

## 技术约束
- 不引入 fsnotify（PRD 硬约束）。
- 不阻塞 setup hook（用 async_runtime::spawn）。
- 不在 commands/ 中拼 SQL：扫描走 `db::asset::list_root_assets`。
- SourceMissingSet 用 `RwLock<HashSet>` 而非 Mutex（读多写少）。
- 失败仅 warn（启动期不应让应用崩溃）。

## 参考文件
- `src-tauri/src/lib.rs::run`（setup hook 位置；参考现有 PipelineScheduler manage / recovery 模式）
- `src-tauri/src/db/asset.rs::list_root_assets`（task_003 输出）
- `src-tauri/src/db/library.rs::get_all` / `db::project::get_by_library`（遍历 project）
- `task_001_architect/output.md` §ADR-004 / §六 内存态模型

## 预估影响范围
- 新建文件：`src-tauri/src/source_scan.rs`
- 修改文件：
  - `src-tauri/src/lib.rs`（pub mod source_scan / setup hook / app.manage）
  - `src-tauri/src/commands/asset.rs`（get_assets 注入 SourceMissingSet）
- 估算变更：~300 行（含 ~120 行测试）
