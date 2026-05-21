# 技术方案 — workspace_unified_md (P0 / Phase 1)

> 任务：task_001_architect
> 输入：`product/prd/workspace_unified_md_prd_v1.md`、`sessions/workspace_unified_md/session_context.md`
> 真实代码根目录：`/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/`
> 说明：本期所有改造直接落在该现有代码库；本方案**不**在 `product/src/` 下重复目录。

---

## 一、项目概述

让"通过悬浮窗导入任意格式文件"在工作区呈现为**单条以 Markdown 为主表示的逻辑资产**：拖入 N 个文件 → 3 秒内可见 N 条占位行 → 异步转化后状态自动收敛为 done；rename / tag / delete / outbound 一律以 `asset_id` 为唯一目标，后端解算双写 root + canonical markdown derivative。本期严守 V1–V8 schema 零新增 migration、复用已落地的 `assets.source_asset_id` 双行模型与 `conversion_meta` append-only 日志，并通过 `db/asset.rs::list_root_assets()` 作为工作区列表的唯一查询入口避免双条目复发。

---

## 二、思考协议（内部）

1. **需求复述**：当前 `commands::dropzone::import_drop_paths` 已经"复制源文件 → insert(asset, root) → PipelineScheduler::enqueue → start"，但 list 端走 `db::asset::get_by_project`，会同时返回 root 和 markdown derivative 两行；scheduler 的 `write_derivative_md` / `write_placeholder_md` 已经写第二行 asset(`source_asset_id != null`, `asset_type = 'markdown'`)。所以"工作区出现两行"完全是**查询端没过滤** + **命令端不区分 root/derivative**导致。修复路径：(a) 列表过滤掉 derivative；(b) DTO 同时携带 source/rendition 路径；(c) 命令 asset_id 化；(d) 状态四态派生；(e) outbound 永远 MD；(f) 删除级联两行 + 两文件 + meta；(g) 启动期扫源；(h) sanitize + hardlink/copy 落盘。

2. **硬约束清单**（来自 session_context + PRD）：
   - 零新增 migration（守 V1–V8）
   - `db/asset.rs::list_root_assets` 唯一工作区列表入口
   - commands/ 内不拼 SQL，不裸 `tokio::spawn`
   - 命令以 `asset_id` 为唯一目标
   - 磁盘文件名 `{asset_id}.{ext}`；display_name 仅 DB
   - 非 done 态禁用 outbound；混合态整体禁用
   - 删除资产不留孤儿（root + derivative + 两文件 + conversion_meta）
   - 不引入 fsnotify
   - 前端 DTO 形状统一在 `src/types/`，组件不重塑
   - 用户可见状态文案中文

3. **风险扫描**（PRD §9 高风险项及应对）：
   - "查询遗漏导致双条目复发"→ ADR-006：把 `db::asset::get_by_project` 标记为 deprecated（保留以兼容其他视图），但工作区视图改走 `list_root_assets`；类型上 `WorkspaceAssetDto` 与原 `Asset` 区分（前端 DTO 拆分）以编译期防错。
   - outbound 跨卷 EXDEV → 复用 `dropzone.rs::try_rename_or_copy_remove` 同源思路实现 hardlink→copy fallback。
   - 状态由三处派生（`pipeline_tasks.status` + `conversion_meta` 最新一行 + 文件 stat）→ 抽到纯函数 `compute_asset_state` 单测覆盖。
   - ChatGPT/Claude 桌面端 .md 识别差异 → 通过 `tauri-plugin-drag` 的 `item: Vec<String>` 已传文件路径；macOS 端原生写入 NSFilenamesPboardType；本期不引入二级 NSStringPboardType（plugin 不支持多 representation 直写），改为通过文件名带 .md 后缀的 hardlink 路径让消费端 sniff。 NSStringPboardType 双 representation 列入风险登记表，Phase 1 末 spike 决定是否在 macos 模块下写 native pasteboard 旁路。

4. **关键技术决策点**：
   - 资产模型复用 / 不复用 → 复用 source_asset_id（PRD 已锁定）
   - 状态字段实时派生 / 触发器 → 实时派生（争议点 #2）
   - hardlink 缓存目录 → `~/Library/Caches/NCdesktop/outbound/{asset_id}/{sanitized_name}.md`（争议点 #1）
   - M0 enqueue 失败回滚边界 → 保留 asset 行（争议点 #3）
   - 文件命名约定：磁盘上 source 是 `{asset_id}_{safe_original_name}.{ext}`（沿用现有 dropzone 行为，保留 stem 信息用于诊断与日后启动期文件名比对），derivative 是 `{derivative_id}_{sanitized_stem}.md`（沿用现有 scheduler 行为）。PRD 中"磁盘文件名由 asset_id 派生"在本期被解释为"asset_id 必须出现在文件名前缀以保证唯一性与可解析性"，display_name 仅活在 DB。这与现有代码一致，零落盘格式迁移。
   - tauri-plugin-drag 改造 → 前端把 `filePaths` 由"asset.filePath"改为"经过后端 `prepare_outbound_payload` 解算出的 cache 路径数组"。

---

## 三、技术选型

| 维度 | 选型 | ADR |
|---|---|---|
| 资产关系模型 | 复用 `assets.source_asset_id` + `asset_type='markdown'` 双行 | ADR-001 |
| 工作区列表查询入口 | 新增 `db::asset::list_root_assets(project_id)`，单一 API | ADR-002 |
| 状态派生 | 实时 join `pipeline_tasks`/`conversion_meta`/文件 stat | ADR-003 |
| Migration 政策 | 零新增；source-missing 内存态 | ADR-004 |
| outbound 实现 | hardlink → 跨卷 copy fallback；缓存到 `~/Library/Caches/NCdesktop/outbound/{asset_id}/` | ADR-005 |
| M0 事务回滚边界 | enqueue 失败不回滚 asset；asset 进入 failed/offline 由后台后续补登 | ADR-006 |
| 命令 asset_id 化 | rename/tag/delete/outbound 一律拒绝接受 file_path | ADR-007 |
| outbound DTO | 后端返回 `{asset_id, path, displayName}`；前端 useDragAssets 改 resolve 逻辑 | ADR-008 |

---

## 四、Architecture Decision Records (ADR)

### ADR-001：资产关系模型 — 复用 `source_asset_id` 双行
- **状态**：已接受
- **上下文**：V5 已落地 `assets.source_asset_id` + `derivative_version`；`find_markdown_derivative`、scheduler `write_derivative_md`/`write_placeholder_md` 都已按"一 root → 一 canonical markdown"工作。新建 rendition 表会触发 V9 迁移，违背"零新增 migration"硬约束。
- **决策**：本期所有"逻辑资产 = root + canonical markdown derivative"。Root 行通过 `source_asset_id IS NULL` 标识；derivative 通过 `source_asset_id = root.id AND asset_type = 'markdown'` 标识。
- **被排除项**：(a) 新建 `renditions` 表（违反零迁移）；(b) 仅一行只写 derivative，源文件以 sidecar 形式存在（破坏现有 dropzone 与 scheduler）。
- **后果**：所有工作区命令必须能从 asset_id 解算另一半（双向）。`db::asset::resolve_asset_pair(asset_id)` 作为统一辅助函数。

### ADR-002：工作区列表唯一查询入口 — `list_root_assets`
- **状态**：已接受
- **上下文**：当前 `commands::asset::get_assets` → `db::asset::get_by_project` 返回所有 asset 行，导致工作区出现 root + derivative 双条目（即用户反馈的 bug 根因）。需要单一 API 保证未来不会有第二个 caller 直接拼 SELECT。
- **决策**：新增 `db::asset::list_root_assets(conn, project_id) -> Vec<WorkspaceAssetView>`，SQL 形如：
  ```sql
  SELECT root.*,
         md.id            AS rendition_id,
         md.file_path     AS rendition_path,
         md.file_size     AS rendition_size,
         pt.status        AS pipeline_status,
         pt.error_message AS pipeline_error,
         cm.error_class   AS latest_error_class,
         cm.fallback_used AS latest_fallback_used,
         ec.status        AS extraction_status
  FROM assets root
  LEFT JOIN assets md
         ON md.source_asset_id = root.id AND md.asset_type = 'markdown'
  LEFT JOIN extracted_content ec
         ON ec.asset_id = root.id
  LEFT JOIN (
    SELECT asset_id, status, error_message
    FROM pipeline_tasks
    WHERE id IN (
      SELECT id FROM pipeline_tasks t1
      WHERE t1.asset_id = asset_id
      ORDER BY created_at DESC LIMIT 1
    )
  ) pt ON pt.asset_id = root.id
  LEFT JOIN (
    SELECT source_asset_id, error_class, fallback_used,
           ROW_NUMBER() OVER (PARTITION BY source_asset_id ORDER BY converted_at DESC) AS rn
    FROM conversion_meta
  ) cm ON cm.source_asset_id = root.id AND cm.rn = 1
  WHERE root.project_id = ?1 AND root.source_asset_id IS NULL
  ORDER BY root.imported_at DESC;
  ```
  - `commands::asset::get_assets` 改为调用 `list_root_assets`，返回 `Vec<WorkspaceAssetView>`（DTO 字段 = root 的 Asset 字段 + `renditionPath` / `renditionId` / `state` / `stateReason`）。
  - `db::asset::get_by_project` **保留**（其他视图如 search/timeline/knowledge 仍在用）但加 `#[deprecated(note = "use list_root_assets for workspace")]`，并在文档中标注"工作区禁用"。
- **被排除项**：(a) 视图（SQL VIEW）—— 不接受参数；(b) 让 frontend 做 group by —— 违反"前端不重塑形状"硬约束。
- **后果**：新增一份 DTO `WorkspaceAssetView`；前端新增 `types/workspaceAsset.ts`；`AssetListView.tsx` 切到新 DTO。

### ADR-003（**争议点 #2**）：状态派生 — 实时 SQL join，**不**做触发器或缓存列
- **状态**：已接受
- **上下文**：状态来源有三：`pipeline_tasks.status`（queued/running/completed/failed/cancelled）、`conversion_meta.error_class`（最新一条）、文件 stat（rendition / source 是否还在盘上）。可选实现：
  (a) 每次 list 实时 join + 在 Rust 侧调 `Path::exists` 派生四态；
  (b) 触发器维护 `assets.cached_state`；
- **决策**：选 (a) 实时派生。理由：
  1. 工作区规模上限 ≤ 1 万（PRD §4.1）；单次 list 即使 join 三表也是常数级延迟（≤ 50ms 估算）。
  2. 触发器方案需要新增列 → 破坏零 migration 硬约束。
  3. 文件 stat 不能放进触发器（DB 看不到磁盘），分裂事实源。
- **被排除项**：触发器；前端缓存。
- **后果**：
  - 派生函数抽到 `db::asset::compute_asset_state(pipeline_status, latest_error_class, rendition_exists, source_exists) -> AssetState`，纯函数易测。
  - 文件 stat 在 list 调用端（commands/asset.rs）做，**不**在 db/ 内做 IO（保持 db 模块纯）。
  - 性能回退条件：若实测 1 万规模下 list 超过 200ms，再升级为 V9 加 `cached_state` 列；本期硬性回退条件写入风险登记表。

### ADR-004：Migration 政策 — 零新增
- **状态**：已接受
- **上下文**：V1–V8 已落地所有必需 schema 元素（`source_asset_id`、`derivative_version`、`conversion_meta`、`pipeline_tasks` + `idx_pipeline_tasks_active_unique`、`extracted_content`）。本期任何新增列都会引入回滚风险（用户库已升至 V8）。
- **决策**：本期不写 `v9_*`。`source-missing` 状态仅在内存态（启动期扫描结果用 `std::collections::HashSet<asset_id>` 保存在 `app.manage(SourceMissingSet)`），不持久化。任何"必须新增列"的提议必须降级为内存态 / 派生 / 配置；若实在做不到，须升级 PRD 边界声明并经 Conductor 批准。
- **被排除项**：v9 添加 `assets.source_status` 列。
- **后果**：重启后 source-missing 需重新扫描（P1 持久化）；P1 决定再升 V9。

### ADR-005（**争议点 #1**）：outbound 缓存目录策略
- **状态**：已接受
- **上下文**：outbound 拖拽时落盘文件名 = sanitize(display_name).md，但实际 derivative 磁盘文件名是 `{derived_id}_{stem}.md`。两者通常不一致；不能直接把 derivative 路径塞给 tauri-plugin-drag（落盘名会变成 `{derived_id}_{stem}.md` 而非用户期望的 display_name）。因此需要在拖出前在某个临时路径生成"显示名正确"的 hardlink/copy。选项：
  (a) `~/Library/Caches/NCdesktop/outbound/{asset_id}/{sanitized_name}.md`（持久缓存，系统 Caches 清理由 macOS 自管）；
  (b) `std::env::temp_dir()/notecapt-outbound-{session_uuid}/`（每次进程启动重建）；
- **决策**：选 (a)。理由：
  1. ChatGPT/Claude 客户端在拖入时立刻拷贝走文件，源 hardlink 是否被清理对消费端无影响；但**拖入流程跨越多次失败重试**时，每次重新生成 hardlink 浪费 IO。`~/Library/Caches` 是 macOS 推荐的"可被系统清理但仍可恢复"路径。
  2. 系统清理 Caches 后，下次 `prepare_outbound_payload` 调用会检测 hardlink missing 并幂等重建（`fs::hard_link` 失败 → fallback copy）；恢复路径无需特殊处理。
  3. per-session 临时目录在用户长会话中文件累积，无系统级清理，反而泄漏更多。
- **被排除项**：tempdir、`$TMPDIR`。
- **后果**：
  - 缓存路径解析：`dirs_next::cache_dir().unwrap().join("NCdesktop/outbound")`；与现有 `dirs_next` 依赖一致。
  - 同 asset_id 多次拖出 → 复用同一文件；如果 `display_name` 在两次拖出之间变更，旧 cache 文件保留（孤儿），但因目录由 asset_id 分桶，相同 asset_id 内每次拖出前清空目录再 hardlink 新文件，避免孤儿堆积。
  - 删除资产时（M6）级联清理 `{cache_root}/{asset_id}/`。

### ADR-006（**争议点 #3**）：M0 事务回滚边界
- **状态**：已接受
- **上下文**：M0 import 流程为 (1) copy 源文件到工作区目录 → (2) `db::asset::insert(root)` → (3) `PipelineScheduler::enqueue(asset_id)`。若 (3) 失败：
  (a) 回滚 (2) 删 asset 行 + 删 (1) 的物理文件 → 用户视为"导入失败"；
  (b) 保留 asset 行，UI 显示 offline/failed，提供"重试入队"入口 → 用户仍能看到资产，可后续重试。
- **决策**：选 (b) — **不回滚 asset 行**。理由：
  1. 与 PRD §S6（离线批量）的"3 秒内 N 条 offline 占位条目可见"一致；enqueue 失败本质上是后台错误，不应让用户感觉"文件凭空消失"（破坏底线 §3 失败可恢复）。
  2. 已落库的 root asset 即使没有 pipeline_task 也能被 M1/M2 渲染：状态派生函数对"无 pipeline_task 且 rendition 不存在"分类为 `offline` 态，而非 `failed`（区别在于无 error_class，提示用户"等待入队/网络恢复后自动入队"）。
  3. 后续 M5 重试与 M9 离线自愈天然兼容。
- **被排除项**：strict transaction（回滚 asset + 删文件）。
- **后果**：
  - import_files 命令必须**先**完成 (1)+(2)，再尝试 (3)；(3) 失败仅 `log::warn`，并把 asset_id 加入返回值 `failures_to_enqueue` 字段。
  - 任何对"完全干净"导入的需求（用户撤销整个导入操作）下放到 M3 delete 命令完成。

### ADR-007：命令 asset_id 化与"解算辅助"
- **状态**：已接受
- **上下文**：现有 `commands::asset::move_asset_to_workspace_folder` 已接受 `asset_ids: Vec<String>`；但 `commands::asset::update_asset` 接受整个 `models::Asset`、`commands::asset::delete_asset` 仅删 root 一行（不级联 derivative）；`useDragAssets` 拿 `asset.filePath` 直传 tauri-plugin-drag。本期需要：
  - 新命令 `rename_asset(asset_id, new_display_name)`；
  - 新命令 `prepare_outbound_payload(asset_ids: Vec<String>) -> Result<Vec<OutboundEntry>, OutboundError>`；
  - `delete_asset` 改为级联（M6）；
  - `link_tag_to_asset` / `unlink_tag_from_asset` 已是 asset_id 化（保留）。
- **决策**：所有新/改命令都只接受 asset_id。后端通过 `db::asset::resolve_asset_pair(conn, asset_id) -> (Asset, Option<Asset>)` 解算"root + 可选 derivative"。
  - 若传入 asset_id 是 derivative，自动解算回 root（用 `source_asset_id`）。
  - rename：双写 root.name + derivative.name（保持 rendition `{stem}.md`、根据 root.name 重新派生 stem 经 `sanitize_stem`）。磁盘文件名保留 `{id}_{stem}.{ext}` 形式不动（display_name 仅在 DB，符合硬约束 §4）。
  - tag：写到 root；通过现有 `db::tag::propagate_tags_to_derivative` 同步到 derivative（已有实现）。
- **被排除项**：接受 file_path 的命令。
- **后果**：前端 `useDragAssets` 改成"调用 `prepare_outbound_payload(asset_ids)` 拿到路径后再 startDrag"。

### ADR-008：outbound payload DTO 与拖拽实现
- **状态**：已接受
- **上下文**：tauri-plugin-drag `startDrag({ item, icon })` 的 `item` 接受路径数组；macOS 侧由插件写入 NSFilenamesPboardType。NSStringPboardType 双 representation（PRD §3.1 M4）当前 plugin 不直接支持。
- **决策**：
  - 命令签名：
    ```rust
    pub struct OutboundEntry {
        pub asset_id: String,
        pub path: String,         // hardlink 或 copy 后的缓存路径
        pub display_name: String, // sanitize(display_name).md
    }
    pub enum OutboundError {
        StateNotDone { asset_id: String, state: String },
        MixedStates { offending: Vec<String> },
        RenditionMissing { asset_id: String },
        IoFailed { asset_id: String, reason: String },
    }
    #[tauri::command]
    pub async fn prepare_outbound_payload(
        database: State<'_, Database>,
        asset_ids: Vec<String>,
    ) -> Result<Vec<OutboundEntry>, String> // String 用 serde_json 序列化 OutboundError
    ```
  - 前端 useDragAssets 改造：拖动开始前 `await invoke('prepare_outbound_payload', { assetIds })`；若返回错误 toast，禁用 startDrag。
  - **NSStringPboardType 双 representation**：本期**不**实现。文件路径已带 `.md` 后缀（caches 目录文件名），ChatGPT/Claude 桌面端在已有 NSFilenamesPboardType 下能正确识别（与已有 hooks/useDragAssets 现状一致）。该项标记为风险，列入 Phase 1 末 NSPasteboard spike（已在 PRD progress 中登记）。
- **被排除项**：拖拽前同步生成 hardlink + 旁路写 NSStringPboardType（需要在 `src-tauri/src/macos/` 下新增 native pasteboard 写入，本期 P1 化）。
- **后果**：spike 决议若发现 .md 识别差异显著，再在 P1 增量加一个 `macos::pasteboard::write_string_representation` 辅助。

---

## 五、系统架构

### 模块划分（增量改动覆盖）

```
src-tauri/src/
├── commands/
│   ├── dropzone.rs        改：import_drop_paths 抽出 import_files 核心，明确 ADR-006 边界
│   ├── asset.rs           改：get_assets 切到 list_root_assets；新增 rename_asset、prepare_outbound_payload
│   ├── conversion.rs      （现状保留，本期不动）
│   └── extraction.rs      （现状保留；retrigger_extraction 复用为 M5 重试命令）
├── db/
│   ├── asset.rs           新：list_root_assets / resolve_asset_pair / compute_asset_state
│   │                      改：delete 改为级联（root + derivative + 两文件 + meta）
│   └── conversion_meta.rs （现状保留）
├── extraction/
│   ├── scheduler.rs       不动主循环；仅复用现有 materialize_md / write_placeholder_md
│   └── ...
├── models/
│   └── asset.rs           不动 Asset；新增 WorkspaceAssetView（专门给工作区列表）
├── outbound/              新模块
│   └── mod.rs             prepare_outbound_payload 业务逻辑：解算 + sanitize + hardlink/copy
├── source_scan.rs         新：M7 启动期扫描 + 内存态 SourceMissingSet
└── lib.rs                 改：setup hook 增加 M7 扫描；invoke_handler 注册新命令

src/
├── types/
│   ├── workspaceAsset.ts  新：WorkspaceAssetView / WorkspaceAssetState
│   └── asset.ts           保留：原有 Asset 仍用于其它非工作区视图
├── components/features/
│   ├── AssetListView.tsx  改：消费 WorkspaceAssetView；状态徽章 4 态；非 done 禁用拖拽
│   └── dropzone/
│       └── DropzoneApp.tsx 不动（导入命令签名不变）
├── hooks/
│   └── useDragAssets.ts   改：拖动前调用 prepare_outbound_payload
└── lib/
    └── tauri-commands.ts  新增 wrapper：renameAsset / prepareOutboundPayload / retryExtraction
```

### 数据流

```
Dropzone import:
  paths → import_drop_paths
    → workspace::ensure_project_workspace
    → for each path:
        fs::copy → db::asset::insert(root, source_asset_id=NULL)
        → PipelineScheduler::enqueue   ── 失败仅 warn（ADR-006）
    → emit "notecapt/import-drop-finished"

Workspace list:
  AssetListView mount → invoke get_assets(projectId)
    → commands::asset::get_assets
       → db::asset::list_root_assets         （ADR-002）
       → for each row: stat rendition / source → compute_asset_state（ADR-003）
       → return Vec<WorkspaceAssetView>
    
Background conversion:
  scheduler loop → run_extractor → write_derivative_md / write_placeholder_md
    → emit "notecapt/asset-converted" 或 "extraction:failed"
    → frontend refresh list

Outbound drag:
  mousedown threshold → useDragAssets
    → invoke prepare_outbound_payload(assetIds)
       → for each id: resolve_asset_pair → 检查 state == done
                       → sanitize(display_name)
                       → fs::hard_link(rendition_path → cache_path)
                          on EXDEV → fs::copy
    → startDrag({ item: cachePaths, icon })

Retry:
  failed-row 重试按钮 → invoke retrigger_extraction(assetId)
    → reset_extraction_state + PipelineScheduler::enqueue + start

Delete:
  context menu 删除 → invoke delete_asset(assetId)
    → db::asset::delete_with_cascade
       → resolve_asset_pair
       → fs::remove_file(root.file_path)
       → fs::remove_file(derivative.file_path)
       → DELETE FROM assets WHERE id = root.id  （FK CASCADE 会清 derivative / conversion_meta / extracted_content / pipeline_tasks）
       → 清理 outbound cache 目录 {asset_id}/

Source scan (startup):
  lib.rs setup hook → tauri::async_runtime::spawn → source_scan::scan_all
    → list root assets across all projects
    → fs::metadata(asset.file_path)
    → 不存在的 → push 到 SourceMissingSet（app.manage）
```

---

## 六、数据模型（核对现状，不新增列）

V1–V8 schema 已就位（见 `src-tauri/src/db/migration.rs`）：
- `assets`：含 `source_asset_id`（V5）、`derivative_version`（V5）、`original_name`（V2）；FK `project_id → projects` ON DELETE CASCADE。
- `conversion_meta`（V6）：FK `source_asset_id → assets` ON DELETE CASCADE；`derived_asset_id → assets` ON DELETE SET NULL。
- `pipeline_tasks`（V7）：含 `idx_pipeline_tasks_active_unique`（`asset_id, task_type` WHERE status IN ('queued','running')）。注：`pipeline_tasks` 无 FK 到 assets，所以"删 asset 时 pipeline_tasks 不会自动 CASCADE"——M6 必须手工 `DELETE FROM pipeline_tasks WHERE asset_id IN (root, derivative)`。
- `extracted_content`（V8）：FK `asset_id → assets` ON DELETE CASCADE。
- `asset_tags`：FK `asset_id → assets` ON DELETE CASCADE。

**FK 全局已启用**：`db::mod::Database::open` 设 `PRAGMA foreign_keys=ON`（验证：`db/mod.rs:39`）。

### 内存态模型（无 schema）

```rust
// src-tauri/src/source_scan.rs
pub struct SourceMissingSet {
    inner: std::sync::RwLock<std::collections::HashSet<String>>,
}
impl SourceMissingSet {
    pub fn contains(&self, asset_id: &str) -> bool { ... }
    pub fn insert(&self, asset_id: String) { ... }
    pub fn remove(&self, asset_id: &str) { ... }
}
// 在 lib.rs setup 中 app.manage(SourceMissingSet::default())；
// list_root_assets 命令端注入 State<SourceMissingSet> 用于派生 state。
```

### WorkspaceAssetView（新 DTO，仅内存/序列化）

```rust
// src-tauri/src/models/asset.rs（追加）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceAssetView {
    // root asset 字段（与现 Asset 同形）
    pub id: String,
    pub project_id: String,
    pub asset_type: String,
    pub name: String,
    pub original_name: String,
    pub file_path: String,           // source path
    pub file_size: i64,
    pub mime_type: String,
    pub captured_at: String,
    pub imported_at: String,
    pub source_type: String,
    pub source_data: Option<String>,
    pub is_starred: bool,
    pub derivative_version: i32,

    // 派生 / 关联
    pub rendition_id: Option<String>,
    pub rendition_path: Option<String>,   // canonical markdown 绝对路径
    pub rendition_size: Option<i64>,
    pub state: AssetState,
    pub state_reason: Option<String>,     // 失败 error_class / source-missing 描述
    pub source_missing: bool,             // 来自 SourceMissingSet
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AssetState {
    Done,
    Converting,
    Failed,
    Offline,
}
```

前端镜像 `src/types/workspaceAsset.ts`。

---

## 七、API 设计（Tauri commands）

### 新增 / 改造命令一览

| 命令 | 签名 | 引入 task | 状态 |
|---|---|---|---|
| `get_assets`（改造） | `(projectId: string) -> Vec<WorkspaceAssetView>` | task_003 | 改造现有 |
| `rename_asset` | `(assetId: string, newDisplayName: string) -> WorkspaceAssetView` | task_004 | 新增 |
| `delete_asset`（改造） | `(assetId: string) -> ()` 级联两文件+meta | task_006 | 改造现有 |
| `prepare_outbound_payload` | `(assetIds: string[]) -> Vec<OutboundEntry>` | task_005 | 新增 |
| `retry_asset_conversion` | `(assetId: string) -> ()` 对外名；内部复用 `retrigger_extraction` | task_006 | wrap 现有 |
| `import_drop_paths`（确认） | 现签名保留；内部按 ADR-006 调整失败语义 | task_002 | 改造现有 |
| `tag` 相关（现状） | `link_tag_to_asset/unlink_tag_from_asset` 已 asset_id 化 | — | 保留 |
| `source_scan_get_missing` | `() -> string[]` 测试/调试用，可选 | task_007 | 新增（仅 debug） |

### invoke_handler 注册（lib.rs 改动）

```rust
.invoke_handler(tauri::generate_handler![
    // 既有 ...
    commands::asset::rename_asset,                  // 新
    commands::outbound::prepare_outbound_payload,   // 新
    commands::extraction::retrigger_extraction,     // 已注册
    // commands::asset::delete_asset 已注册，无需重复
])
```

---

## 八、目录结构（指向真实路径）

所有改动落在以下实际仓库路径：

```
/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/
├── src-tauri/
│   ├── src/
│   │   ├── lib.rs                                       改
│   │   ├── source_scan.rs                               新建
│   │   ├── commands/
│   │   │   ├── asset.rs                                 改
│   │   │   ├── dropzone.rs                              改（轻微，ADR-006）
│   │   │   ├── extraction.rs                            保留
│   │   │   └── outbound.rs                              新建（prepare_outbound_payload）
│   │   ├── db/
│   │   │   └── asset.rs                                 改（list_root_assets / resolve_asset_pair / delete_with_cascade / compute_asset_state）
│   │   ├── models/
│   │   │   └── asset.rs                                 改（追加 WorkspaceAssetView / AssetState）
│   │   ├── extraction/scheduler.rs                      保留（不动主循环）
│   │   └── workspace.rs                                 保留
│   └── tests/
│       └── workspace_unified_md_integration.rs          新建（S1–S5 + S7–S8）
└── src/
    ├── types/
    │   ├── workspaceAsset.ts                            新建
    │   └── index.ts                                     改（re-export）
    ├── hooks/
    │   └── useDragAssets.ts                             改
    ├── lib/
    │   └── tauri-commands.ts                            改（新 wrapper）
    ├── stores/
    │   └── assetStore.ts                                改（DTO 切换）
    └── components/features/
        ├── AssetListView.tsx                            改（4 态徽章 + 拖拽禁用）
        └── AssetContextMenu.tsx                         改（rename / delete / 重试）
```

> 注：**不**在 `harness-kit/product/src/` 下复刻目录。

---

## 九、安全考量

- **本地纯桌面，安全等级低**（session_context §3）。
- 注意：`prepare_outbound_payload` 写文件到 `~/Library/Caches/NCdesktop/outbound/`，需确认路径不会写入 `~/Downloads/` 等敏感目录之外。已通过 `dirs_next::cache_dir()` 落到 macOS 系统 Cache 标准位置。
- `rename_asset` 不修改磁盘文件名（仅修改 DB display_name），无路径遍历风险。
- `delete_asset` 仅 `fs::remove_file(asset.file_path)`，必须在删之前验证 `path.starts_with(workspace_root)` 以防 asset.file_path 被人为篡改指向系统路径（已在 `move_asset_to_workspace_folder` 中有类似保护，复用模式）。
- 命令端不拼 SQL；不裸 spawn —— 编译期通过命名约定保证（review 检查项）。

---

## 十、风险登记表

| 风险 | 概率 | 影响 | 缓解措施 |
|---|---|---|---|
| `list_root_assets` 4-way JOIN 在 1 万规模下慢于 200ms | 低 | 中 | ADR-003 已声明回退条件：实测超过 200ms 升级 V9 加 cached_state 列；测试中加 `bench_list_root_assets_at_10k` |
| ChatGPT/Claude 桌面端 .md 识别差异 | 中 | 中 | 缓存路径文件名带 `.md` 后缀；Phase 1 末 NSPasteboard spike 决定是否补 NSString representation |
| 用户在 Finder 中改 rendition 磁盘文件名 | 低 | 中 | 接受，文档化为已知局限；下次 outbound 失败 → 走 RenditionMissing 错误路径 |
| source-missing 仅内存态，重启后未扫描期间显示错误 | 低 | 低 | 启动期扫描在 setup hook 中**同步等待完成第一遍**（async spawn 但前端 list 命令依赖完成），失败仅 warn |
| `pipeline_tasks` 无 FK → CASCADE 不自动 | 中 | 中 | `delete_with_cascade` 手工 `DELETE FROM pipeline_tasks WHERE asset_id IN (...)` |
| outbound cache 目录系统清理后用户立即拖拽 → 找不到 | 低 | 低 | `prepare_outbound_payload` 总是先 `remove_dir_all` 再重建 hardlink，幂等 |
| 多人/同设备并发拖拽同一资产 | 低 | 低 | hardlink 是幂等操作（先删再建）；同进程串行执行 |
| ADR-006 下 enqueue 失败留下"无 pipeline_task" 资产，状态派生需正确判 offline | 中 | 中 | `compute_asset_state` 把"无 pipeline_task + 无 rendition + 无 conversion_meta" 派生为 offline；单测覆盖 |
| 启动期 source 扫描在大库（1 万 root）阻塞 setup | 低 | 中 | 用 `tauri::async_runtime::spawn`，UI 启动不阻塞；首屏空缺以 `source_missing=false` 渲染，扫描完毕 emit `notecapt/source-scan-finished` 让前端 invalidate |
| 工作区列表查询遗漏（潜在新增 caller） | 低 | 高 | `db::asset::get_by_project` 加 `#[deprecated]`；CI grep 检查工作区代码路径不出现该函数 |

---

## 十一、Task 清单

### 总览（依赖拓扑见下）

| Task ID | 标题 | 主要目标 | 估算变更行数 |
|---|---|---|---|
| task_002_dev_m0_atomic_import | M0 原子导入事务 | 整理 `import_drop_paths` 与 ADR-006 边界；提取 import_files 核心函数；集成单测 | ~250 |
| task_003_dev_m1m2_list_state | M1 折叠列表 + M2 四态聚合 | `list_root_assets` + `compute_asset_state` + WorkspaceAssetView；`get_assets` 切流；纯函数单测 | ~600 |
| task_004_dev_m3_asset_id_commands | M3 命令 asset_id 化 | `rename_asset`（双写 root+derivative）；`resolve_asset_pair`；标签同步走现有 propagate | ~350 |
| task_005_dev_m4_outbound_payload | M4 outbound MD payload | 新 `commands/outbound.rs`；sanitize；hardlink + copy fallback；幂等 cache；命令注册 | ~450 |
| task_006_dev_m5m6_retry_delete | M5 失败重试 + M6 删除级联 | `delete_with_cascade`（root+derivative+两文件+conversion_meta+pipeline_tasks 手工+outbound cache）；`retry_asset_conversion` 包装；幂等校验 | ~400 |
| task_007_dev_m7_source_scan | M7 启动期 source 扫描 | `source_scan.rs` + `SourceMissingSet`；lib.rs setup hook 接入；事件通知前端 | ~300 |
| task_008_frontend_integration | 前端集成 | types/workspaceAsset.ts；assetStore 切 DTO；AssetListView 4 态徽章 + 禁用拖拽 + 重试按钮；useDragAssets 走 prepare_outbound_payload；中文文案 | ~700 |
| task_009_integration_tests | 集成测试 | tauri 后端集成测试覆盖 S1/S2/S3/S4/S5/S7/S8（PRD §8）；含 1 万规模 bench | ~500 |
| task_010_ux_review | UX 评审 | 状态文案、拖拽禁用反馈、失败提示一致性、键盘可达性 | 文档型 |

### Task 粒度自检

每个 task 均满足：
- 单一目标 ✓（一句话目标见上）
- 可独立测试 ✓（task_002/003/004/005/006/007 都有独立单测路径；task_009 是集成测试本身）
- 规模 ≤ 2000 行 ✓
- 依赖清晰 ✓（见拓扑）
- AC 可验证 ✓（每个 input.md 列出 `cargo test --test ...` 或 `pnpm test` 命令）

---

## 十二、Task 依赖拓扑

```
                           ┌────────────────────┐
                           │ task_002 (M0)      │
                           │ 原子导入事务边界    │
                           └────────┬───────────┘
                                    │
                                    ▼
┌────────────────────────────────────────────────────────┐
│ task_003 (M1+M2)                                       │
│ list_root_assets + compute_asset_state + DTO           │
└────────┬────────────────┬──────────────────────┬───────┘
         │                │                      │
         ▼                ▼                      ▼
┌─────────────────┐ ┌─────────────────┐ ┌──────────────────┐
│ task_004 (M3)   │ │ task_007 (M7)   │ │ task_008 前端    │
│ rename + 解算   │ │ source 扫描      │ │（依赖 003 DTO 与 │
└────────┬────────┘ └────────┬─────────┘ │ 004 命令 + 005   │
         │                   │           │ outbound + 006   │
         ▼                   │           │ delete/retry）   │
┌─────────────────┐          │           └─────────┬────────┘
│ task_005 (M4)   │          │                     │
│ outbound payload│          │                     │
└────────┬────────┘          │                     │
         │                   │                     │
         ▼                   │                     │
┌─────────────────┐          │                     │
│ task_006 (M5+M6)│          │                     │
│ 重试 + 删除级联 │          │                     │
└────────┬────────┘          │                     │
         │                   │                     │
         └─────────┬─────────┴─────────────────────┘
                   ▼
        ┌────────────────────┐
        │ task_009 集成测试  │
        └────────┬───────────┘
                 ▼
        ┌────────────────────┐
        │ task_010 UX 评审   │
        └────────────────────┘
```

可并行：
- task_004 / task_005 / task_007 在 task_003 完成后可并行（彼此无文件冲突，影响范围分别在 commands/asset.rs / commands/outbound.rs / source_scan.rs+lib.rs）
- task_008 可在 task_004/005/006 任一完成后开始局部集成，但完整冒烟需全部完成

关键路径：task_002 → task_003 → task_006 → task_009 → task_010

---

## 十三、三个争议点的最终决议结论（摘要）

1. **outbound hardlink 缓存目录**：选 `~/Library/Caches/NCdesktop/outbound/{asset_id}/{sanitized_name}.md`。系统清理 Caches 后由 `prepare_outbound_payload` 幂等重建。ADR-005。
2. **状态聚合 SQL 性能**：默认实时 JOIN 派生，不引触发器。回退条件：实测 1 万规模 list 超过 200ms 即升 V9 加 cached_state（破零迁移由 P1 决议）。ADR-003。
3. **M0 两阶段事务回滚边界**：enqueue 失败**不**回滚 asset 行。状态自动归类为 offline，由 M5 重试或 M9 自愈兜底。ADR-006。

---

## 十四、建议 Conductor 写入 progress.md 的 Task 清单文本

复制以下文本到 `sessions/conductor/progress.md` 的"待执行 Task 队列"段落：

```
- [x] task_001_architect — 基于 PRD v1 产出技术方案（含 ADR / Task 切分）
- [ ] task_002_dev_m0_atomic_import — M0 原子导入事务边界（ADR-006）
- [ ] task_003_dev_m1m2_list_state — M1 折叠列表 + M2 四态聚合（list_root_assets / compute_asset_state）
- [ ] task_004_dev_m3_asset_id_commands — M3 rename / resolve_asset_pair 等命令 asset_id 化
- [ ] task_005_dev_m4_outbound_payload — M4 outbound MD payload（sanitize + hardlink/copy + 缓存目录）
- [ ] task_006_dev_m5m6_retry_delete — M5 重试入口 + M6 删除级联（root+derivative+两文件+meta+pipeline_tasks+outbound cache）
- [ ] task_007_dev_m7_source_scan — M7 启动期 source 扫描 + 内存 SourceMissingSet
- [ ] task_008_frontend_integration — 前端 AssetListView 切 DTO / 拖拽禁用 / 重试按钮 / 中文文案
- [ ] task_009_integration_tests — 集成测试覆盖 S1/S2/S3/S4/S5/S7/S8（PRD §8）
- [ ] task_010_ux_review — UX 体验审查（状态文案、键盘可达、错误提示一致性）
```

并把以下记录追加到"关键决策记录"：

```
- 2026-05-13 Architect 决议：ADR-005 outbound 缓存目录 = ~/Library/Caches/NCdesktop/outbound/{asset_id}/；ADR-003 状态实时派生；ADR-006 M0 enqueue 失败不回滚 asset。
```

---

## 十五、Architect 自检

- [x] 思考协议 4 步全部执行
- [x] PRD §10 三个争议点各自一个 ADR 决议
- [x] 至少覆盖：资产模型复用 / 查询单一入口 / outbound hardlink-copy / 零 migration 4 项 ADR
- [x] 真实代码路径核对：V1–V8 migration、`source_asset_id`、`conversion_meta`、`idx_pipeline_tasks_active_unique`、`find_markdown_derivative`、`write_derivative_md/placeholder` 都已就位
- [x] 目录结构指向真实仓库路径，未在 `product/src/` 下重复
- [x] 每个 task 单一目标、可独立测试、≤ 2000 行变更
- [x] 风险登记表覆盖性能 / 兼容 / 启动期 / 并发 / FK 缺失等关键面
- [x] 用户可见状态文案约束已传递给 task_008
