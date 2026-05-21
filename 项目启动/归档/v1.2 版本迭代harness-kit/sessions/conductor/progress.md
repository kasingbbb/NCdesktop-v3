# Conductor Progress — NoteCapt v1.2

## 当前状态

STATE: ACCEPTANCE
当前 Task: 全部完成
更新时间: 2026-05-09

---

## 已完成 Tasks

| Task | 描述 | 状态 |
|------|------|------|
| task_001_iflytek_client | 讯飞 ASR Rust 客户端实现 | ✅ DONE |
| task_002_asr_swap | extractor 注册替换 | ✅ DONE（随 task_001 一并完成） |
| task_003_dropzone_close | 悬浮窗关闭 bug 修复 | ✅ DONE |

---

## 已知问题 / Blockers

- **API 端点路径待验证**：`UPLOAD_PATH = "/v2/private/lfasr/upload"` 和 `QUERY_PATH = "/v2/private/lfasr/getResult"` 为推测路径，首次运行需对照讯飞官方文档确认；若路径错误，日志会输出包含实际 HTTP 响应内容的错误信息，便于快速定位调整

---

## 关键决策记录

- 2026-05-09：task_001 和 task_003 并行完成；task_002 因工作量极小（仅改 mod.rs 一行注册）随 task_001 一并交付
- 2026-05-09：讯飞凭据采用编译期常量（v1.3 迁移至设置页）
- 2026-05-09：Extractor 内部用 `Handle::current().block_on()` 驱动 async HTTP，无需修改 scheduler 或添加 reqwest blocking feature
- 2026-05-09：悬浮窗关闭改用 `invoke('close_dropzone_window')` 而非 `win.close()`，利用已有 Rust 命令确保可靠性

---

## 状态转移日志

[2026-05-09] STATE: INIT → TASK_START | Task: task_001 + task_003 | 原因: PRD 已确认，session 结构已创建 | 风险: 低
[2026-05-09] STATE: TASK_START → ACCEPTANCE | Task: 全部 | 原因: cargo check 通过，7/7 单元测试 PASS，前端无编译错误 | 风险: 低（API 路径待真实调用验证）
