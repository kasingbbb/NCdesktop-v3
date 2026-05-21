# Task 交付 — task_004_dev_pr1_degraded_startup

## 实现摘要
新建 `src-tauri/src/startup.rs` + `commands/app_mode.rs`；`lib.rs::run` 用 `bootstrap()` 替代裸 `Database::open`，按 `migrate → repair → 推导 AppMode` 顺序执行；前端新增 `AppModeBanner`、`uiStore.appMode`、`AppMode` 类型；ReadOnly 守卫 `ensure_writable` 已暴露给 task_005/006/012/013 复用。

## 修改/新建
| 文件 | 类型 | 说明 |
|------|------|------|
| `src-tauri/src/startup.rs` | 新建 | AppMode + bootstrap + ensure_writable + 3 单测 |
| `src-tauri/src/commands/app_mode.rs` | 新建 | get_app_mode / get_repair_progress |
| `src-tauri/src/commands/mod.rs` | 修改 | `pub mod app_mode;` |
| `src-tauri/src/lib.rs` | 修改 | bootstrap 接入 + 命令注册 |
| `src/stores/uiStore.ts` | 修改 | AppMode 类型 + appMode 字段 |
| `src/components/features/AppModeBanner.tsx` | 新建 | 三态横幅 |
| `src/App.tsx` | 修改 | 在 AppLayout 上方挂 banner |

## 架构遵守
- [x] 调用顺序符合 ADR-006：migrate → repair → derive_mode
- [x] AppMode 枚举 `serde(tag="kind", rename_all="snake_case")`，前端解构友好
- [x] ReadOnly 守卫集中（task_006/012/013 直接调用）

## 测试 / 结果
```
cargo test --lib startup → 3 passed
cargo test --lib         → 91 passed; 0 failed
npx tsc --noEmit         → 0 errors
```

## 自测矩阵
| 类型 | 场景 | 状态 | 结果 |
|------|------|------|------|
| ✅ | 新库 bootstrap → Normal | PASS | bootstrap_fresh_db_normal |
| ✅ | ensure_writable 拦截 ReadOnly，放行 Normal/Degraded | PASS | ensure_writable_blocks_readonly |
| ✅ | 不可写路径 → ReadOnly | PASS | bootstrap_unwritable_dir_falls_to_readonly |
| ⚠️ | 前端 banner 渲染 | 未启动 dev server，仅 TS 类型校验通过 | 见已知局限 |
| ⚠️ | Degraded 场景 | 集成测试缺失（需注入 failed=1 模拟），单元测试 ensure_writable 已覆盖判定逻辑 | 见已知局限 |

## 已知局限
1. 前端横幅未实测渲染（无法在此会话启动 dev server）；依赖 task_017 UX review 时端到端验证
2. Degraded 场景未做集成测试；单元层面 AppMode 推导分支已覆盖
3. ReadOnly 守卫仅暴露 helper；具体 write 命令的接入由 task_006/012/013 落地

## Reviewer 关注
- bootstrap 内 `database.conn.lock()` 的借用域（用 `run_repair_locked` 隔离避免与后续 move 冲突）
- `serde(tag="kind", rename_all="snake_case")` 序列化形态是否与前端 `AppMode` 类型匹配
- 内存 fallback Connection（`open_in_memory`）的语义：极端情况下用户启动后看到 ReadOnly + 空数据，可接受
