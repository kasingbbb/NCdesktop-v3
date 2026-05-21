# PRD — 悬浮窗导入工作区文档统一为 MD（workspace_unified_md v1）

> 项目：NCdesktop
> 复杂度：M
> 状态：Debate 已闭合，可移交 Conductor / Architect
> 起草日期：2026-05-13
> 辩论日志：`sessions/workspace_unified_md/debate/session_001/debate_log.md`

---

## 1. 项目概述

### 1.1 用户原话（PM 输入）
> 现在通过"悬浮窗"导入多个文件时，工作区会同时呈现两个文件：一个是重命名后的文件，另一个是转化后的文件。用户最终希望的体验是：拖拽 5 个不同格式的文件到悬浮窗，工作区只看到 5 条 MD 资产，且 rename / 打标签等操作完全关联到 MD，而非作为两个独立条目分开呈现。

### 1.2 一句话定义
**让工作区每一行 = 一个"以 MD 为主呈现的逻辑资产"**，源文件作为该资产的物料保留，所有元数据操作以 `asset_id` 为唯一标识，outbound 拖拽 payload 恒为 MD。

### 1.3 本期不解决
1-to-N 拆篇、rendition / source 多版本、fsnotify 长驻监听、10 万级规模性能承诺（标 ≤ 1 万）、协作 / 同步、非 MD outbound（如多模态直传图片）。

---

## 2. 用户定义与核心场景

### 2.1 用户画像
单用户、本地桌面（macOS）、用 NCdesktop 作为"AI 工作流入口"——把音频/PDF/图片/Office 等异构资料整理为 MD，再多选拖入 ChatGPT / Claude / 其他 AI 客户端。

### 2.2 核心场景
1. **批量导入**：悬浮窗拖入 5 个不同格式文件 → 应用自动转化 → 工作区出现 5 条 MD 资产。
2. **整理**：选中某条资产 → rename / 打标签，操作对 MD 与源同时生效（用户感知为对"一份东西"操作）。
3. **拖出消费**：多选 done 态资产 → 拖入 ChatGPT 客户端，落地为 MD 文件且文件名为 display_name。
4. **失败/离线降级**：网络断 / 服务故障时仍能导入，资产以 "离线待转化" / "失败可重试" 呈现，可 rename / 打标签。
5. **回看源**：用户点"查看原文件"打开原音频/PDF 核对。

---

## 3. 功能需求（带优先级）

### 3.1 P0（MVP 必交付）

| # | 模块 | 功能 | 主要落点 |
|---|---|---|---|
| M0 | 原子导入事务 | `import_files` 命令：先 `INSERT` root asset(`status=pending`) → 后 `enqueue` conversion；导入返回时用户已能看到 N 条占位 | `commands/dropzone.rs`、`commands/extraction.rs`、`db/asset.rs` |
| M1 | 折叠列表 | `list_root_assets()` 唯一查询入口：`WHERE source_asset_id IS NULL`；附带 `rendition_path` / `source_path` / 派生 `state` | `db/asset.rs`、`commands/asset.rs` |
| M2 | 四态聚合 | 由 `pipeline_tasks.status` + `conversion_meta` + 文件存在性派生 `{done, converting, failed, offline}` | `db/asset.rs::compute_asset_state` |
| M3 | 命令 asset_id 化 | `rename` / `tag` / `delete` / `outbound_payload` 命令仅接受 `asset_id`，后端解算到 root + rendition 双写 | `commands/asset.rs`、`commands/conversion.rs` |
| M4 | outbound MD payload | `prepare_outbound_payload(asset_ids[])`：done 态返回 hardlink 路径（跨卷 fallback copy），同时写 NSFilenamesPboardType + NSStringPboardType；非 done 态 `Err(state)` | `commands/dropzone.rs`、前端 `AssetListView.tsx` |
| M5 | 失败重试入口 | failed 态行显示「重试」按钮，调用 scheduler.enqueue；依赖 V7 `idx_pipeline_tasks_active_unique` 保证幂等 | `commands/extraction.rs`、`AssetListView.tsx` |
| M6 | 删除级联 | 删除 asset → 删 root + derivative 两行 + 两个磁盘文件 + 关联 `conversion_meta`（FK CASCADE 已就位） | `db/asset.rs::delete` |
| M7 | 启动期 source 扫描 | 应用启动时单次遍历 root assets，stat source 文件；不存在则内存态 `source-missing`，UI 置灰"查看原文件"按钮 | `lib.rs` setup hook |

### 3.2 P1（次版）

| # | 模块 | 功能 |
|---|---|---|
| M8 | converting 态取消转化（`pipeline_tasks.status → cancelled`） |
| M9 | 离线检测 + 网络恢复自动入队 |
| M10 | 资产详情页转化日志面板（基于 `conversion_meta.list_by_source`） |
| M11 | 多选混合态拖拽：含一条非 done 即整体禁用 + toast |
| M12 | 批量导入进度聚合 UI |
| M13 | `source-missing` 持久化（新增列或迁移到 P1 才动 schema） |

### 3.3 P2 / Out

拆篇（1-to-N）、rendition 多版本、source 多版本、fsnotify 长驻、10 万级性能、协作/同步、非 MD outbound（多模态直传）。

---

## 4. 非功能需求

### 4.1 性能
- 悬浮窗导入 N 个文件，**3 秒内**工作区可见 N 条占位条目（M0 保证）。
- 工作区规模 **≤ 1 万 asset** 是承诺范围；超出范围视为已知局限。
- outbound 拖拽 hardlink 同卷应在 200ms 内完成；跨卷 copy 视 rendition 大小而定（典型 < 1MB）。

### 4.2 可用性 / 降级
- 任意态资产均支持 rename、tag、delete。
- 非 done 态禁用 outbound 拖拽；多选混合态整体禁用 + toast。
- markitdown / 讯飞 ASR 失败 → `failed` 态，提供重试入口。

### 4.3 数据一致性
- DB display_name 是元数据真相源，磁盘文件名由 asset_id 派生 (`{asset_id}.{ext}`)。
- outbound 落盘文件名 = sanitize(display_name)；同名冲突追加 `_<asset_id 前 8 位>`。
- 删除 asset 必须保证：root row / derivative row / 两个磁盘文件 / conversion_meta 历史 全部清除，受管目录无孤儿。

### 4.4 文件名 sanitize 规则
- `/`、`\` → `_`
- 控制字符（U+0000–U+001F、U+007F）→ 删除
- emoji / CJK → 保留（macOS APFS 原生支持 UTF-8）
- Windows 保留字（CON/PRN/...）、尾随 `.` 或空格 → 追加 `_`
- UTF-8 长度截断到 200 字节并对齐字符边界
- 若超长被截断，追加 `_<asset_id 前 8 位>` 保唯一

### 4.5 可观测性
- 所有 conversion 调度走 `extraction/scheduler.rs`，状态变更写 `conversion_meta`。
- M5 / M9 重试和入队事件可在日志中追溯。

---

## 5. 技术约束（来自 session_context.md）

- **语言/框架**：Rust（Tauri 2 src-tauri）+ React 18 + Vite。
- **数据库**：SQLite，**本期零新增 migration**（依赖 V1–V8 已落地 schema：`assets.source_asset_id`、`conversion_meta`、`pipeline_tasks` 含 `idx_pipeline_tasks_active_unique`、`extracted_content`）。
- **不在 commands/ 中拼 SQL**：所有 DB 操作走 `db/` 模块。
- **不在 command 中起裸 `tokio::spawn`**：异步通过 `extraction/scheduler.rs` 统一调度。
- **类型源**：前端工作区列表 DTO 形状统一定义在前端 `types.ts`，组件不自行重塑结构。
- **用户可见状态文案统一中文**。

---

## 6. 分期计划

| 阶段 | 内容 | 目标周期（参考，不硬约束） |
|---|---|---|
| Phase 1 — MVP | M0–M7（P0 全部） + 集成测试覆盖 S1/S2/S3/S4/S5/S7/S8 | 首版交付 |
| Phase 2 — Polish | M8–M13（P1）+ S6 完整可度量化 | 次版 |
| Phase 3 — 扩展 | P2 项视需求与埋点结果再启动 | 远期 |

---

## 7. 关键技术决策（Architect 必读）

| ID | 决策 | 选择 | 理由 |
|---|---|---|---|
| A | 资产模型 | **复用 `assets.source_asset_id` 两行表示**，不新建 rendition 表 | V5 已落地、`find_markdown_derivative` 已可用、零 migration |
| B | 状态来源 | 由 `pipeline_tasks.status` + `conversion_meta` + 文件存在性派生 | 单一事实源，避免双写 |
| C | Migration | **零新增**；`source-missing` 留在内存态 / P1 再持久化 | 降低本期风险 |
| D | outbound 实现 | hardlink 优先；跨卷 fallback **copy**（不用 symlink）；NSFilenamesPboardType + NSStringPboardType 双 representation | 兼容 ChatGPT/Claude 桌面端差异 |
| E | 离线检测 | 被动（调用失败分类）+ 启动期一次探测；**不**用 fsnotify | 与 Layer 1 决议一致 |
| F | 查询单一入口 | `db/asset.rs::list_root_assets()` / `list_assets_filtered()`；**禁止**其他 caller 直接拼 SQL | Rust 类型系统兜底，编译期防遗漏 |

---

## 8. 成功标准（验收）

### Happy-path
- **S1 唯一性**：导入 5 混合格式文件，工作区精确显示 5 条资产，扩展名展示 `.md`。
- **S2 元数据一致**：rename 后 (a) 列表显示，(b) outbound 落盘文件名（经 sanitize），(c) DB display_name 三处一致。
- **S3 三态可见**：done / converting / failed 在 UI 自动化中可断言 `data-state` 属性。
- **S4 失败降级**：markitdown 失败下资产仍存在且可 rename / tag。
- **S5 源不丢↔删除**：删除 asset 后，受管目录中对应 source 与 rendition 均消失，无孤儿。

### Sad-path
- **S6 离线批量**：断网下导入 N 个文件，**3 秒内** N 条 offline 占位条目可见；可 rename / tag；不可拖出。
- **S7 失败重试无重复**：连击 5 次重试，工作区行数恒等于导入数；`conversion_meta` 历史 ≥ 5 条，活动 `pipeline_tasks` 至多 1 条。
- **S8 source 失联**：source 文件被 Finder 删除后，rendition 仍可拖出；"查看原文件"按钮置灰并提示。

---

## 9. 已知局限 / 风险登记

| 风险 | 类型 | 状态 | 处理 |
|---|---|---|---|
| rendition 文件被用户在 Finder 中改名 | 数据一致性 | 接受 | 写入已知局限；下次 outbound 可能失败 |
| `source-missing` 仅内存态 | 持久化 | P1 | 重启后需重新扫描 |
| 跨卷 outbound 落到 copy 时占空间 | 性能 | 接受 | rendition 通常 < 1MB |
| display_name 含极端 Unicode → sanitize 后视觉差异 | UX | 接受 | sanitize 规则文档化 |
| ChatGPT/Claude 桌面端对 .md 识别差异 | 兼容性 | 双 representation 缓解；需 Phase 1 末做一次手动 spike | — |
| 工作区规模超 1 万 asset 性能未承诺 | 性能 | 已知局限 | 在 PRD 显式声明 |

---

## 10. Debate 未达成共识的争议

无强争议遗留。所有 ⏸️ 项都已显式划入 P1 / P2 / out-of-scope。

下列点 **Architect 设计时必须做出明确选择**（不在 PRD 内强行选定）：

1. **outbound hardlink 缓存目录策略**：使用 `~/Library/Caches/NCdesktop/outbound/{asset_id}/` 还是 per-session 临时目录？需考虑系统清理 Caches 时的恢复路径。
2. **状态聚合 SQL 查询性能**：派生 `state` 字段是在每次 list 查询中实时 join 计算，还是用触发器维护一个 `assets.cached_state`？后者破坏"零 migration"约束，建议默认前者，性能压测后再决议。
3. **导入失败的事务回滚边界**：M0 中"insert asset → enqueue conversion"两阶段，若 enqueue 失败是否回滚 asset 行？建议保留 asset(`status=failed`) 给用户重试机会，但 Architect 需在设计中显式说明。

---

## Conductor 桥接摘要

### 核心功能清单（带优先级）

| 功能 | 优先级 | 核心用户场景 | 来自 Debate 的关键约束 |
|---|---|---|---|
| M0 原子导入事务 | P0 | 拖入 5 文件，3s 内 5 条占位可见 | "insert asset(pending) → enqueue" 顺序不可逆 |
| M1 折叠列表（`list_root_assets`） | P0 | 工作区行数 = 逻辑资产数 | 必须是唯一查询入口 |
| M2 四态聚合 | P0 | done/converting/failed/offline 四态可见 | 状态由 `pipeline_tasks` + `conversion_meta` 派生，不落库 |
| M3 命令 asset_id 化 | P0 | rename/tag/delete/outbound 都以 asset_id 为唯一目标 | 后端解算双写 root+rendition |
| M4 outbound MD payload | P0 | 多选拖入 ChatGPT，落地 MD | hardlink+copy fallback；双 representation；sanitize 规则固化 |
| M5 失败重试入口 | P0 | 失败资产 1 键重试 | 依赖 V7 active unique idx 幂等 |
| M6 删除级联 | P0 | 删除资产无孤儿 | root+derivative+磁盘+meta 全清 |
| M7 启动期 source 扫描 | P0 | source 失联 UI 提示 | 不用 fsnotify |
| M8–M13 | P1 | 取消/离线自愈/日志/多选 toast/批量进度/source-missing 持久化 | — |

### 不可妥协的技术底线

1. **零新增 migration**：本期严守 V1–V8 schema；任何 schema 变更须升级版本号并被 Conductor 显式批准。
2. **`db/asset.rs::list_root_assets` 是工作区列表的唯一查询入口**；其他 caller 不得直接拼 `SELECT ... FROM assets`。
3. **命令以 `asset_id` 为唯一目标**：rename / tag / delete / outbound 命令禁止接受文件路径。
4. **磁盘文件名由 asset_id 派生**：用户可见名只活在 DB `display_name`。
5. **非 done 态禁用 outbound 拖拽**：单选 / 多选混合态一致行为。
6. **删除资产不留孤儿**：root + rendition + 两个磁盘文件 + 关联 conversion_meta 全部清除。
7. **不引入 fsnotify 长驻监听**。

### 已识别的高风险项

| 风险 | 来源 | 当前状态 | 缓解策略 |
|---|---|---|---|
| 查询遗漏导致双条目复发 | Layer 4 Reviewer 挑战 #1 | 已解决 | 决策 F：`list_root_assets` 单一 API |
| outbound 跨卷 EXDEV | Layer 4 Reviewer 挑战 #2(a) | 已解决 | copy fallback |
| display_name 含特殊字符 sanitize 后破坏一致性 | Layer 4 Reviewer 挑战 #2(b) | 已解决 | sanitize 规则固化（§4.4） |
| ChatGPT/Claude 桌面端 .md 识别差异 | Layer 4 Reviewer 挑战 #2(c) | 缓解 + 待 spike | 双 representation；Phase 1 末手动 spike |
| converting → done 落盘命名竞态 | Layer 1 Reviewer 挑战 B | 已解决 | 磁盘名 `{asset_id}.md` 永远不变，display_name 仅 DB |
| 10 万级规模性能 | Layer 4 挑战 D | 搁置 | PRD 显式声明 ≤ 1 万 |
| rendition 带外修改 | Layer 1 Q4 | 接受 | out-of-scope，写入已知局限 |
| `source-missing` 仅内存态 | Layer 4 Reviewer #3 | 搁置 | P1 持久化 |

### MVP 边界声明

- **做什么**：M0 原子导入事务、M1 折叠列表、M2 四态聚合、M3 命令 asset_id 化、M4 outbound MD payload（含 sanitize 与双 representation）、M5 失败重试、M6 删除级联、M7 启动期 source 扫描。
- **不做什么**：
  - 拆篇（1-to-N）— 触发 schema 演进、与产品价值无直接挂钩；
  - rendition / source 多版本 — 同上；
  - fsnotify 长驻 — ROI 低、平台陷阱多；
  - 10 万级规模性能承诺 — 超出本期用户场景；
  - 批量进度聚合 UI、`source-missing` 持久化 — 体验优化，不阻塞核心契约；
  - 协作 / 同步、非 MD outbound — 远期 P2。

### Debate 中未达成共识的争议

无强争议遗留。需 Architect 设计时决议的具体落点见 §10：
1. hardlink 缓存目录策略；
2. `state` 字段实时派生 vs 触发器维护；
3. M0 两阶段事务中 enqueue 失败的回滚边界。
