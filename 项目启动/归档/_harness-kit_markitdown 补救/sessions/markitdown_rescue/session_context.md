# Session Context — markitdown_rescue

## 1. 项目信息
- **项目名称**：NCdesktop × MarkItDown 集成补救
- **一句话描述**：把当前"半实现+编译断裂"的 MarkItDown 集成补完整，落地宪章 v1.0 的 Step 1-6，并为 Step 7 知识下游对齐留接口。
- **项目类型**：Desktop App（Tauri + Rust + React/TypeScript）
- **复杂度等级**：M（改造现有代码、11 个 task、中等不确定性；跳过 Debate，直接 Architect → Dev 循环）

## 2. 技术上下文
- **主语言**：Rust（src-tauri）+ TypeScript（src）
- **框架/运行时**：Tauri 2.x、rusqlite、React、Zustand
- **数据库**：SQLite（迁移由 `src-tauri/src/db/migration.rs` 顺序执行）
- **关键外部依赖**：Microsoft `markitdown` 0.1.5（Python 3.10+，子进程调用）、`sha2`、`serde_json`、`uuid`
- **现有代码库**：改造现有代码（基线：`项目启动/NCdesktop/src-tauri/`；该目录在 git 中未跟踪，整套改动为 WIP）
- **目标部署环境**：本地桌面（macOS DMG，含嵌入式 venv）

## 3. 关键约束
- **安全性要求**：中 — markitdown 通过子进程调用，必须避免 shell 注入；只用 `Command::arg`/`args` 传参，不拼字符串。
- **性能要求**：中 — 转换在后台 pipeline 异步执行，但前端 Inspector 切换文件不能因为 IO 等待而卡顿。
- **用户体验要求**：高 — 必须三态可分（成功 / 已 fallback / 失败），不可"假成功 + 空 .md"。
- **可维护性要求**：高 — 标签传播、幂等物化、转换元数据三件事必须只有**一处实现**。
- **不可妥协的底线**：
  1. `cargo check` 必须先恢复绿，**未恢复前 main 分支不接受任何后续改动**。
  2. 同一 root asset 的 canonical markdown 衍生件**全系统唯一**。
  3. 衍生 .md **必须**自动继承原件标签，覆盖：物化创建时、AI 后补打标时、用户手动新增时。
  4. MarkItDown 失败**必须**有 fallback 链路，不允许直接把失败暴露给用户。
  5. 所有 markdown 衍生件的生成事实必须落到 `conversion_meta`，便于失败率统计与诊断。

## 4. 质量偏好

| 维度 | 权重 | 说明 |
|------|------|------|
| 功能正确性 | 30% | 三大底线（编译/幂等/标签）必须 PASS |
| 架构一致性 | 20% | 不允许在 dropzone/scheduler/inspector 三处重复实现标签传播逻辑 |
| 可维护性 | 15% | 模型与迁移一致；转换器抽象不被业务穿透 |
| 安全性 | 10% | 子进程参数化、错误信息脱敏（不向前端泄露 stderr 原文） |
| 测试覆盖 | 15% | 标签传播、幂等、fallback 三条路径必须有最小集成测试 |
| 代码质量 | 10% | Rust 侧避免 `unwrap()`，错误用 `?` + `map_err` |

## 5. 领域特定代码规范
- Rust：对外暴露的函数返回 `Result<T, String>` 或既有错误类型，**禁止 `.unwrap()`/`.expect()`** 在非 `main`/测试代码。
- SQL：参数化绑定（`params![...]`），禁止字符串拼接。
- 标签传播只允许通过 `db::tag::propagate_*` / `sync_*` 公共函数，不允许 inline INSERT。
- 数据库迁移**仅向后兼容**：新增列必须 `ALTER TABLE ... ADD COLUMN`，禁止 `DROP`/破坏式重建。
- 前端：所有 Tauri 命令调用经 `src/lib/tauri-commands.ts`，禁止页面组件直接 `invoke`。

## 6. 领域特定审查重点
- `scheduler.rs::write_derivative_md` 的"首次创建 / 已存在覆盖"两条分支的版本号推进是否对齐。
- `materialize_placeholder` 与真正成功路径**共享版本号空间**，需要验证 placeholder 之后真转换成功时不会回退版本号。
- `derivative_version` 写到 source 和 derivative 两侧（`scheduler.rs:691-692`），是否会让 derivative 在不同行号上分叉。
- MarkItDown 失败 → fallback → 都失败 → placeholder：四态切换是否有任何一条路径漏写 `conversion_meta`。
- 前端 Inspector 在 fallback 状态下展示"已自动回退"的措辞，不能让用户误判为失败。

## 7. 角色专业背景补充
- **Architect 应具备**：Tauri 命令注册机制、rusqlite 迁移模式、Python 子进程在 Rust 中的稳态包装。
- **Reviewer 应重点关注**：标签传播是否散落到多个调用点；fallback 链路的元数据完整性；前端三态文案是否一致。

## 8. 文件路径约定
- **基线源码**：`/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri/src/` 与 `/项目启动/NCdesktop/src/`
- **PRD/宪章**：`项目启动/NCdesktop/MarkItDown_集成开发宪章_v1.0.md` 与 `MarkItDown_文件格式转化迭代规划宪章_v1.0.md`
- **Session 记录**：`项目启动/_harness-kit_markitdown 补救/sessions/markitdown_rescue/`
- **进度文件**：`sessions/markitdown_rescue/conductor/progress.md`
- **架构方案**：`sessions/markitdown_rescue/conductor/tasks/task_001_architect/output.md`

## 9. 辩题概述
跳过（M 复杂度但本次仅做 Architect 再回放；如后续出现新争议，由 Conductor 触发 Layer 4 策略辩论）。
