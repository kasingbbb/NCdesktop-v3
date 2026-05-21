# Debate Log — workspace_folder_mgmt / session_001

> Host 主持的 4 层 Debate 完整记录。Proposer 与 Reviewer 由 subagent 扮演。
> 项目上下文：`sessions/workspace_folder_mgmt/session_context.md`

---

## Layer 1 — 问题定义

### Round 1.1 Proposer（初始立场）
- **核心问题**：把"悬浮窗导入"从只读 chip 升级为可写 Finder 列表，闭合"建/改/删/移"人工整理回路；与 ai_organized 正交。
- **边界**：范围内 = F1-F4 + 仅 root + 替换 chip；范围外 = 嵌套、多选、ai_organized 写、Win/Linux 删除、撤销栈。
- **替换 vs 并存** Trade-off：选定替换（单一真相源、chip 撑不起 inline 编辑/右键/drop）。
- **场景**：导入 30 剪藏 → 建"参考资料" → 拖 5 截图 → 删旧"草稿"。
- **指标**：M1 完成率 ≥ 95%；M2 外跳 Finder ↓ 70%；M3 脏数据 = 0；M4 拒绝率 = 100%。
- **脆弱性**：嵌套需求；EXDEV；SQL 前缀冲突；dragover 节流。

### Round 1.2 Reviewer（L3 + L2 挑战）
- **L3**：`__ROOT__` 存储契约未定义。DB `assets.file_path` 对根散文件该写什么？前后端各处特判位置？rename 前缀替换是否需特殊处理？M3 因此不可度量。
- **L3**：F4 对称性缺口——drop target 含 `__ROOT__` 但未声明"从 root 拖回 `__ROOT__`"是否合法。若否则打脸 M2。
- **L2**：M1-M3 不可埋点（无埋点系统、无基线、无运行时自检），应改为工程可验收断言。

### Round 1.3 Proposer（回应）
- **`__ROOT__` Canonical 契约**：仅 UI/IPC sentinel，**严禁入 DB**；`assets.file_path` = 根目录文件为裸名、子目录为 `参考资料/a.png`；后端单点 `resolve_relative_path()`：`__ROOT__→""`；rename 前缀用 `LIKE 'old/%' ESCAPE '\'`；写路径加 `debug_assert!(!path.contains("__ROOT__"))`。
- **F4 双向**：支持 `__ROOT__` 双向 drop；根目录同名冲突返回明确错误。
- **M1-M4 重写**：M1=P0 验收清单 1-6 全过；M2=`test_round_trip_root_to_folder_to_root`；M3=ai_organized 四类写均 Err + 前端 drop 拦截单测；M4=`cargo test`+`pnpm test` 全绿 + rename 集成测试 DB 行数=物理子树。

### Host 共识 1
（见 debate_conclusions.md）

---

## Layer 2 — 理想态

### Round 2.1 Proposer
- **inline 编辑 3 态**：Enter/blur 提交（同步乐观）；Esc 取消；冲突显示行内红字 + 抖动。
- **拖拽 4 阶段**：enter→inset 2px 描边；over→cursor copy/not-allowed；leave→清；drop→闪烁后清。
- **右键菜单 3 形态**：root 全菜单；ai_organized 灰显 + tooltip；`__ROOT__` 隐藏 rename/delete（不灰显）。
- **工具栏联动**：`+` 永激活；rename/delete 仅 root 激活。
- **错误模型初版**：结构化 enum + 边界转 `[CODE] message` 字符串前缀（自承脆弱）。
- **状态管理**：不新建 store；uiStore 加 `editingFolderPath` / `dropTargetPath`。

### Round 2.2 Reviewer
- **L3 错误模型骑墙**：字符串前缀是伪结构化；要么前端只认 code 自出文案，要么用 `{code, message, details}` 对象。选一个。
- **L2 inline blur 竞态**：编辑 A 时点 B → blur 提交→click 切 B → toast 落到 B 上下文，回滚不知闪谁。
- **L2 F1 幽灵行失败**：未定义。
- **L2 拖到编辑行**：3 选 1 未定。
- **L1 删除 N 口径**：必须后端实时 count，前端 hint。

### Round 2.3 Proposer（回应）
- **错误模型 → 结构化 JSON `IpcError {code, message, details?}`**；前端文案表唯一来源；message 仅日志。
- **inline blur → 同步乐观 + selection 冻结**；失败回滚到原节点 A，selection 自动定位回 A。
- **F1 幽灵行**：失败保留编辑态 + 红框；Esc 直接丢弃；切走需二次确认 modal。
- **拖到编辑行 → 拒绝 drop**（禁止图标 + toast）。
- **删除 N**：新增 IPC `count_folder_assets`，loading 态展示。

### Host 共识 2（含 MVP 边界裁回）
（见 debate_conclusions.md；Host 裁回了 Proposer 越界提到的递归 count、E_DEPTH_LIMIT、E_CYCLE、i18n）

---

## Layer 3 — 差距分析

### Round 3.1 Proposer
- **改动清单 8 文件**；**风险登记 R1-R9**：EXDEV / 越界 / SQL ESCAPE / TOCTOU / 并发 / trash 静默失败 / Tauri DnD / NFC / 深色模式（POST-MVP）。

### Round 3.2 Reviewer
- **L3 R1 fs 事务原子性是话术**：SQLite 事务无法回滚 FS；要求两阶段顺序明文。
- **L3 R4 TOCTOU**：confirm→trash 之间并发写入会被一起 trash 留 DB 孤儿；二选一（事务内重扫 vs 仅删空）。
- **L2 R5 锁范围漏 move_asset**。
- **L2 R8 NFC 不对称**：扫描时 NFD 字节与 DB NFC 字节不等会 ENOENT。
- **L2 R10 ⌘⌫ 绕过 disabled**：必须 handler 入口判定。
- **L2 R9 深色模式不能 POST-MVP**：drop 不可见 = 功能不可达。

### Round 3.3 Proposer（回应）
- **R1 copy-first 两阶段**：copy_dir→fsync→rename(tmp→final) → BEGIN/UPDATE/COMMIT → COMMIT 后 remove 源（失败仅记 cleanup_pending 日志）。最坏=多占磁盘，绝不丢数据。
- **R4 选 B：仅删空**（被 Host 后续否决）。
- **R5 锁覆盖 5 命令**。
- **R8 NFC 自愈**：readdir 真实字节 → nfc 查 DB → miss 入库存 N；hit B≠N → 一次性 rename(B→N) + nfc_healed 日志。
- **R10 handler 入口统一判定**。
- **R9 drop 高亮 `var(--accent-emphasis)` 进 MVP**。

### Host 共识 3（裁决：R4 否决"仅删空"）
（见 debate_conclusions.md）

---

## Layer 4 — 策略

### Round 4.1 Proposer
- 4 等级 scope；**6 task** 拆分；裁剪原则；回溯映射；交付门槛 checklist。
- 但把 ⌘⌫、工具栏 重命名/删除 按钮、深色精修都降到 P1。

### Round 4.2 Reviewer
- **L3 擅自缩小用户已写死的 P0 验收面**：⌘⌫ 是 spec §F3 入口，工具栏三按钮是 spec §工具栏 明文要求；不能降 P1。
- **L2 T0 契约对齐缺失**；`__ROOT__` 编解码 + NFC 自愈无 task 落点。
- **L2 T5 单 task 包了 4 个独立交互态**，应拆 T5a 骨架 / T5b 编辑状态机。
- **L1 Conductor 桥接摘要未列入交付物**。

### Host 共识 4（直接裁定，无需 Proposer 再回）
- 工具栏三按钮 + ⌘⌫ 全部恢复 P0；
- task 拆为 8 个，新增 T0 契约冻结、T5 拆 a/b；
- NFC 自愈 hook 挂 T1，`__ROOT__` 编解码挂 T1；
- Conductor 桥接摘要写入 PRD §末。

---

## 论证追踪表（全程汇总）

| 论点 | 提出方 | 层级 | 状态 | 备注 |
|---|---|---|---|---|
| 替换 chip 为 Finder 列表 | Proposer | L1 | ✅ | trade-off 充分 |
| `__ROOT__` 仅 UI/IPC sentinel | Reviewer→Proposer | L1 | ✅ | 契约级修正 |
| F4 `__ROOT__` 双向 drop | Reviewer→Proposer | L1 | ✅ | — |
| M1-M4 改可验收断言 | Reviewer→Proposer | L1 | ✅ | 弃伪 SLO |
| 嵌套子文件夹 | Proposer | L1 | ⏸️ | P2 |
| 结构化 IpcError JSON | Reviewer→Proposer | L2 | ✅ | — |
| 同步乐观 + selection 冻结 | Reviewer→Proposer | L2 | ✅ | — |
| F1 幽灵行：保留+切走二次确认 | Reviewer→Proposer | L2 | ✅ | — |
| Drop 到编辑行=禁止 | Reviewer→Proposer | L2 | ✅ | — |
| `count_folder_assets` IPC | Reviewer→Proposer | L2 | ✅ | — |
| 4 命令最终签名+错误枚举 | Proposer | L2 | ✅ | — |
| 不新建 store + uiStore 加字段 | Proposer | L2 | ✅ | — |
| 递归 count / E_DEPTH_LIMIT / i18n | Proposer | L2 | ❌ | Host 裁回（越界） |
| R1 copy-first 两阶段 | Reviewer→Proposer | L3 | ✅ | — |
| R4 仅删空 | Proposer | L3 | ❌ | Host 否决（违 spec） |
| R4 非空可删+事务 recount+写锁 | Host | L3 | ✅ | Host 裁决 |
| R5 锁范围 5 命令 | Reviewer→Proposer | L3 | ✅ | — |
| R8 NFC 自愈 | Reviewer→Proposer | L3 | ✅ | — |
| R9 drop 高亮入 MVP | Reviewer→Proposer | L3 | ✅ | — |
| R10 handler 入口判定 | Reviewer→Proposer | L3 | ✅ | — |
| 工具栏三按钮+⌘⌫ 入 P0 | Reviewer→Host | L4 | ✅ | 否决 Proposer 降级 |
| T0 契约冻结 | Reviewer→Host | L4 | ✅ | 新增 task |
| T5 拆 a/b | Reviewer→Host | L4 | ✅ | — |
| Conductor 桥接摘要 | Reviewer→Host | L4 | ✅ | 入 PRD 末 |
| 跨进程并发 | Proposer 自承 | L4 | ⏸️ | 单进程多窗口 OK，跨进程 P2 |
