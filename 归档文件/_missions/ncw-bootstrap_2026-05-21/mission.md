# Mission: ncw-bootstrap_2026-05-21

## 核心诉求

把 NoteCapt Windows 版（仓库 kasingbbb/notecapt-windows，HEAD 40926a7e）从"代码已迁移"推到：
- 开发环境（macOS host）能跑通核心功能（至少能 `cargo check` / 起 `vite dev` / 主界面渲染 / 关键 IPC 不 panic）
- Windows 实机 build 通过 + 主要功能不报错（用户在 Windows 主机上跑实测后反馈）

## 暗偏好（chris 从 mission 描述里萃取）

1. **不污染 macOS 源**：`项目启动/NCdesktop/` 整棵树只读
2. **隔离工作**：所有产物落在 `_missions/ncw-bootstrap_2026-05-21/` + `/tmp/ncw-test/notecapt-windows/`
3. **正规 PR**：不 force push notecapt-windows 的 main，所有修复走 hotfix branch + PR
4. **决策点交回用户**：API key、是否引入新依赖、是否跳过某功能等

## 红线

- 不改 `项目启动/NCdesktop/` 任何文件
- 不 force push notecapt-windows main
- 不 spawn agent 在用户拍板 plan 之前

## 自动推进许可

- chris 自主：take stock、写 plan、跑 cargo check、跑 vitest、读代码、对比 diff
- 需要主对话/用户：改代码、开 PR、合 PR、申请 API key、跑 Windows 实测

## 时间盒

- Take stock：~30 min（已完成）
- Plan 评审：~10 min（待用户拍板）
- Phase 1 修复（macOS host buildable）：~1-2 h
- Phase 2 开发环境验证（vite dev + 关键 IPC）：~1-2 h
- Phase 3 用户 Windows 实测：异步、不计时
