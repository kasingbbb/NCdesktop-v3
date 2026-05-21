# Task 交付 — task_002_dev_m0_atomic_import

## 实现摘要

按 ADR-006 把 `import_drop_paths` 的「INSERT asset(pending) → enqueue conversion」两阶段事务边界明确化。核心改动：

1. **新增最小 trait `EnqueueScheduler`**（仅在 `commands/dropzone.rs` 内）+ 生产实现 `AppHandleEnqueue<'a>`（透传到 `PipelineScheduler::enqueue`）。Trait 注入仅在本文件内做，不扩散到 `scheduler.rs`。
2. **抽出纯函数 `pub fn import_files_core<S: EnqueueScheduler>(conn_mutex, scheduler, project_id, paths) -> Result<ImportCoreOutput, String>`**：同步函数（无 await，天然不存在跨 await 持 MutexGuard 的问题）；每条 path 顺序执行「copy → insert(短锁块) → scheduler.enqueue（锁已释放）」三步；enqueue 失败 → 仅 `log::warn` + `failures_to_enqueue.push(asset_id)`，**不删 asset 行 / 不删源文件**（ADR-006）；asset 仍计入 `created`，让 UI 可见为 offline 占位。AI 旁路相关 (asset, classify_input) 通过新出参 `ImportCoreOutput::ai_pending_jobs` 回吐给命令薄包装层，避免核心层裸 `tokio::spawn`。
3. **`import_drop_paths` 退化为薄包装**：仅做 AppHandle / State 解构 + `ensure_import_project_id` + 调用 `import_files_core` + 在命令层 spawn AI 旁路 + 调用 `PipelineScheduler::start`（幂等）+ emit `notecapt/import-drop-finished` 事件。即使 `failures_to_enqueue` 全员失败，事件仍 emit。
4. **`ImportDropSummary::failures_to_enqueue`**（已存在字段）在所有出口被正确填充。
5. **新增 `#[cfg(test)] mod tests`** 覆盖 happy / failure 两条核心路径，使用 in-memory-style 临时 SQLite 文件（跑 V1–V8 migration）+ 一次性 UUID project_id（隔离 workspace 目录）+ 两类假 scheduler（`OkScheduler` 实测写一行 pipeline_tasks，`FailingScheduler` 永远 Err）。

不动 `db/asset.rs`、`extraction/scheduler.rs`、`mod.rs` 重导出，未引入新依赖。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---------|----------|------|
| `NCdesktop/src-tauri/src/commands/dropzone.rs` | 修改 | 新增 `EnqueueScheduler` trait / `AppHandleEnqueue` 适配器 / `ImportCoreOutput` 结构 / 纯函数 `import_files_core`；`import_drop_paths` 改为薄包装；新增 `#[cfg(test)] mod tests`（两个单测） |

## 对 Architect 方案的遵守声明

- [x] 目录结构与 Architect 方案一致（改动仅在 `commands/dropzone.rs`，未新建文件）
- [x] API 路径/命名与 Architect 方案一致（`import_drop_paths` 命令签名未变）
- [x] 数据模型与 Architect 方案一致（仅使用 V1–V8 已有列；零新增 migration）
- [x] 未引入计划外的新依赖
- 偏离说明：input.md 给出的核心函数签名是 `(conn, scheduler, project_id, paths)`；为保证 ADR-006 中「enqueue 期间不持锁」与「Mutex 借用所有权对单测可拼装」两点，实际签名采用 `&StdMutex<Connection>` 而非裸 `&Connection`（这是同一只 Mutex 的借用，语义与 input 一致；纯函数仍不持 AppHandle，单测构造容易），并增加一个进程内出参 `ImportCoreOutput::ai_pending_jobs`（不序列化），用于把 AI 旁路 spawn 留在命令薄包装而非核心层。

## 测试命令

```bash
cd "/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri"
cargo test -p notecapt --lib commands::dropzone
```

> 注：input.md 写的是 `-p app_lib`，但本仓库 `Cargo.toml` 中 `[package].name = "notecapt"`、`[lib].name = "app_lib"`（`app_lib` 是 lib 名而非 package ID）。实际可用的命令为 `-p notecapt`；输出二进制目录仍为 `target/debug/deps/app_lib-*`。已就此核对：`cargo test -p app_lib …` 报 `package ID specification 'app_lib' did not match any packages`。

## 测试结果

```
warning: `notecapt` (lib test) generated 5 warnings (run `cargo fix --lib -p notecapt --tests` to apply 4 suggestions)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.41s
     Running unittests src/lib.rs (target/debug/deps/app_lib-8c2b6ae4be6c948e)

running 2 tests
test commands::dropzone::tests::enqueue_failure_keeps_asset ... ok
test commands::dropzone::tests::happy_path_inserts_root_and_enqueues ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 80 filtered out; finished in 0.04s
```

（5 条 warning 均为 task_002 改动之外的既存噪声：`PathBuf` 未使用、`llm/chat.rs` 三个 `_*` 参数与 `AnthropicContent` 死字段——在 cargo check 中即已存在，未在本 task 修复以遵守「只做你的 task，不多做」。）

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|----------|----------|------|-----------|
| ✅ 正常路径 | 2 个文件 → 2 条 root asset(`source_asset_id IS NULL`) + 2 条 `pipeline_tasks(status='queued')`，`failures_to_enqueue` 为空 | 已测 | PASS（`happy_path_inserts_root_and_enqueues`） |
| ❌ 异常路径 | enqueue 永远 Err → 2 条 root asset 仍在 DB；2 个工作区副本文件仍在盘；`failures_to_enqueue.len()==2`；`pipeline_tasks` 空表 | 已测 | PASS（`enqueue_failure_keeps_asset`） |
| ⚠️ 边界 | `paths.is_empty()` → 命令层短路返回 `ImportDropSummary::default()`（带空 `failures_to_enqueue`） | 未单测 | 由 `Default` 派生天然成立；命令薄包装中显式判断 |
| ⚠️ 边界 | 单 path 复制失败（fs::copy Err） → 仅计入 `failures`，不进 `created` | 未单测 | 与原行为一致；未在 task_002 范围内变更该分支 |
| ⚠️ 边界 | 单 path DB insert 失败（如 UUID 碰撞）→ 计入 `failures`，物理文件保留为孤儿，不调用 enqueue | 未单测 | 与原行为一致；未在 task_002 范围内变更该分支 |
| ⚠️ 并发 | 多个 import_drop_paths 同时进行 | 未单测 | 锁粒度未变（每次仅持 Mutex 做单条 insert / settings 查询），与原实现同语义 |

## 已知局限

1. **`failures_to_enqueue` UI 提示**：本 task 仅保证后端字段被正确填充；前端如何渲染（toast / 行内徽章）属于 task_008 范畴。
2. **ADR-006 路径下的 offline 状态派生**：核心函数让 asset 行存在但无 pipeline_task；状态派生函数 `compute_asset_state` 把"无 pipeline_task + 无 rendition + 无 conversion_meta"判为 `offline`——此判定属 task_003 范畴，本 task 不实现。
3. **测试落到真实 `~/Downloads/NoteCaptWorkPlace/<uuid>/`**：使用 UUID 子目录隔离 + 测试尾部 best-effort `remove_dir_all` 清理；若进程异常退出可能残留子目录，下次 `tempdir` 不受影响（uuid 唯一）。未引入更深的 workspace mock 是为了保持 `import_files_core` 的真实路径与生产路径完全一致。
4. **未给"复制成功 + insert 成功 + enqueue 成功 + AI 旁路缺席"组合做单测**：该路径在 happy path 内已隐含覆盖（`LLMClient::is_available_in_conn` 在没有 settings 的全新 DB 上返回 false）。

## 需要 Reviewer 特别关注的地方

1. **死锁防护**：`import_files_core` 在调用 `scheduler.enqueue` 之前**必须**已释放 DB MutexGuard（`AppHandleEnqueue` 的生产实现会在 `PipelineScheduler::enqueue` 内部再去 `app.state::<Database>().conn.lock()`）。代码中通过显式 `{ let conn = …; … }` 短锁块 + drop 时机保证；请确认 Reviewer 视角下没有遗漏的"insert 后仍持锁直到 enqueue"路径。
2. **trait 借用形态**：`AppHandleEnqueue<'a> { app: &'a AppHandle }`、`OkScheduler<'a> { conn_mutex: &'a StdMutex<…> }`——生产路径与测试路径都不需要 `'static`。如对 trait 暴露范围有顾虑，可在后续 task 把 trait 私有化为 `pub(crate)`。
3. **`failures_to_enqueue` 的语义边界**：本 task 把"enqueue Err"视作唯一来源；其他失败（copy / insert）依然走 `failures`。这是为了让 UI 能区分「offline 占位」与「彻底没进库」。
4. **测试隔离**：依赖 `dirs_next::download_dir()` 返回 Some（macOS / Linux 桌面一般成立）；CI 上若无桌面环境会失败。如需 headless CI，应在 task_009 阶段引入 workspace 路径注入。
5. **`-p app_lib` vs `-p notecapt`**：测试命令上述差异。建议 Conductor 在后续 task 的 input.md 中统一为 `-p notecapt`。
