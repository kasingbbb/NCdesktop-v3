# Task 输入 — task_004_dev_pr1_degraded_startup

## 目标
建立统一启动入口 `startup::bootstrap` → `AppMode::{Normal, Degraded(Reason), ReadOnly(Reason)}`，三档降级在首次窗口渲染前确定；前端读取后渲染横幅 / 屏蔽编辑入口。

## 前置条件
- 依赖 task：task_002 + task_003
- 必须先存在的文件/接口：`db/migration.rs::run_v10`、`db/repair.rs::run_post_migration_repair`

## 验收标准（AC）
1. 新建 `src-tauri/src/startup.rs`，导出 `bootstrap() -> AppMode`，调用顺序 `migrate → repair → derive_mode`
2. 三档判定：(a) 全部成功 = `Normal`；(b) repair Lenient 有失败 = `Degraded { reason: "partial_repair", failed_count }`；(c) migrate panic / DB 损坏 = `ReadOnly { reason }`
3. `tauri::Builder::setup` 中调用，注入为 `tauri::State<AppMode>`
4. 新增 command `get_app_mode() -> AppMode`，前端启动时读取
5. 前端：`uiStore` 增 `appMode` 字段；新建 `AppModeBanner` 组件（Degraded 黄条 / ReadOnly 红条）；ReadOnly 模式下导入 / 编辑 / 保存命令短路返回"只读模式"错误
6. ReadOnly 模式仍允许：列表查看、Finder 中显示、导出
7. `bootstrap` 单测：模拟 migrate 失败 / repair 部分失败 / DB 文件不可写

## 技术约束
- `AppMode` 用 serde tag-internal `enum`（前端解构友好）
- ReadOnly 模式由后端命令统一短路（前端禁用按钮 + 后端兜底）
- 不阻塞渲染：bootstrap 完成在 `tauri::Builder::setup` 内同步等待，但 repair 异步部分继续

## 参考文件
- task_001 output.md ADR-006
- `src-tauri/src/lib.rs`（注册位置）
- `src/stores/uiStore.ts`（appMode 字段）

## 预估影响范围
- 新建：`src-tauri/src/startup.rs`（~150 行）、`src/components/features/AppModeBanner.tsx`（~80 行）
- 修改：`src-tauri/src/lib.rs`、`src/stores/uiStore.ts`、根 Layout 引入 banner
- 测试：`src-tauri/tests/startup.rs`（~70 行）

## Reviewer 重点关注
- ReadOnly 短路覆盖面（写命令清单是否齐全）
- bootstrap 失败的最坏情况（DB 文件锁）
- 前端 banner 的 i18n / 用户操作引导
