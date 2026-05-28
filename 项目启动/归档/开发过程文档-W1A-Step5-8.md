# NCdesktop 开发过程文档 — W1-A 并行线（Step 5-8）

> 记录人：AI Agent（Claude）
> 执行日期：2026-03-25
> 适配宪章：《（cursor）NCdesktop 20 步开发框架宪章》
> 阶段：W1 窗口并行线 A（Agent α，Rust 数据 + 引擎）
> 状态：✅ 全部完成

---

## 执行摘要

W1-A 是「数据地基」阶段，负责从类型定义到数据库到状态管理到 IPC 通信的完整数据链路贯通。

| Step | 名称 | 状态 | 产出文件数 |
|------|------|------|-----------|
| 5 | TypeScript 类型系统 | ✅ 完成 | 10 文件 |
| 6 | SQLite 数据库层（Rust 端） | ✅ 完成 | 16 文件 |
| 7 | Zustand 状态管理层 | ✅ 完成 | 10 文件 |
| 8 | Tauri IPC 通信桥 | ✅ 完成 | 1 文件（前端封装）+ Step 6 中已含 Rust 命令 |

---

## Step 5：TypeScript 类型系统

### 执行内容

基于《NCdesktop 软件 PRD》第三章数据模型，定义全部核心实体的 TypeScript 类型。

### 产出文件

```
src/types/
├── index.ts          # 统一导出
├── library.ts        # Library
├── project.ts        # Project, ProjectSource, ProjectMetadata
├── timeline.ts       # Timeline, AudioTrack, Transcription, TranscriptionSegment, Keyframe, Marker
├── asset.ts          # Asset, AssetType, AssetSource, AIAnalysis
├── common.ts         # Tag, Note
├── export.ts         # ExportConfig, ExportFormat
├── settings.ts       # AppSettings, LLMTarget
├── sync.ts           # TFCardManifest, SessionData, SyncProgress 等
├── llm.ts            # ChatMessage, LLMConfig, LLMRequestLog
└── ui.ts             # LayoutMode, PlaybackState, TimelineViewport, SearchResult, ModalType, Notification, DropzoneState 等
```

### 类型覆盖率

| 领域 | 类型/接口数 |
|------|-----------|
| 知识库 & 项目 | 4 |
| 时间轴 & 音频 | 8 |
| 素材 & AI 分析 | 5 |
| 标签 & 笔记 | 2 |
| 导出 | 2 |
| 设置 | 2 |
| TF 卡同步 | 6 |
| LLM | 4 |
| UI 状态 | 11 |
| **合计** | **44** |

### 验收结果

| 检查项 | 结果 |
|--------|------|
| `tsc --noEmit` 零错误 | ✅ |
| 所有 PRD 实体已覆盖 | ✅ |
| 使用 `interface` 定义对象、`type` 定义联合类型 | ✅ |
| 不含 `any` | ✅ |

---

## Step 6：SQLite 数据库层（Rust 端）

### 执行内容

1. **添加 Rust 依赖**：`rusqlite`（bundled）、`uuid`（v4）、`chrono`
2. **数据模型**：创建对应 TypeScript 类型的 Rust 结构体（`serde` + `camelCase` 序列化）
3. **数据库初始化**：`PRAGMA WAL + foreign_keys`、`user_version` 版本管理
4. **V1 迁移**：创建 12 张表 + 10 个索引 + 3 个 FTS5 虚拟表 + 6 个 FTS 触发器
5. **CRUD 模块**：按实体分文件实现完整增删改查
6. **FTS5 搜索**：素材名/路径搜索 + 笔记内容搜索 + 聚合搜索

### 数据库表结构

| 表名 | 行为 | FTS |
|------|------|-----|
| libraries | CRUD | - |
| projects | CRUD + 级联删除 | - |
| assets | CRUD + 星标切换 | ✅ fts_assets |
| ai_analyses | Upsert | - |
| tags | CRUD + get_or_create | - |
| asset_tags | 关联 + 计数刷新 | - |
| project_tags | 关联 + 计数刷新 | - |
| timelines | CRUD | - |
| audio_tracks | CRUD | - |
| transcriptions | Upsert | - |
| keyframes | CRUD | - |
| markers | CRUD | - |
| notes | CRUD | ✅ fts_notes |
| settings | KV get/set | - |

### 产出文件

```
src-tauri/src/
├── models/
│   ├── mod.rs          # 模块导出
│   ├── library.rs      # Library
│   ├── project.rs      # Project
│   ├── asset.rs        # Asset, AIAnalysisRow
│   ├── timeline.rs     # Timeline, AudioTrack, Transcription, Keyframe, Marker
│   ├── common.rs       # Tag, Note, AssetTag, ProjectTag
│   └── settings.rs     # SettingRow
├── db/
│   ├── mod.rs          # Database 结构 + 初始化
│   ├── migration.rs    # V1 迁移（DDL）
│   ├── library.rs      # Library CRUD
│   ├── project.rs      # Project CRUD
│   ├── asset.rs        # Asset CRUD + AI 分析
│   ├── timeline.rs     # Timeline/AudioTrack/Keyframe/Marker CRUD
│   ├── tag.rs          # Tag CRUD + 关联
│   ├── note.rs         # Note CRUD
│   ├── search.rs       # FTS5 搜索
│   └── settings.rs     # Settings KV
└── commands/
    ├── mod.rs
    ├── library.rs      # 4 个 IPC 命令
    ├── project.rs      # 5 个 IPC 命令
    ├── asset.rs        # 7 个 IPC 命令
    ├── timeline.rs     # 10 个 IPC 命令
    ├── tag.rs          # 5 个 IPC 命令
    ├── note.rs         # 5 个 IPC 命令
    ├── search.rs       # 1 个 IPC 命令
    └── settings.rs     # 3 个 IPC 命令
```

### 验收结果

| 检查项 | 结果 |
|--------|------|
| `cargo check` 零错误 | ✅ |
| WAL + foreign_keys 启用 | ✅ |
| 版本化迁移（user_version） | ✅ |
| FTS5 虚拟表 + 触发器 | ✅ |
| 40 个 IPC 命令注册 | ✅ |

---

## Step 7：Zustand 状态管理层

### 执行内容

按功能域拆分 9 个 Store，每个 Store 封装对应的 IPC 调用。

### 产出文件

```
src/stores/
├── index.ts            # 统一导出
├── libraryStore.ts     # 知识库管理
├── projectStore.ts     # 项目管理
├── assetStore.ts       # 素材管理 + 视图模式 + 排序
├── timelineStore.ts    # 时间轴 + 播放控制 + 视口缩放
├── tagStore.ts         # 标签管理
├── noteStore.ts        # 笔记管理
├── searchStore.ts      # 全局搜索
├── settingsStore.ts    # 应用设置（含主题切换）
└── uiStore.ts          # UI 状态（布局/模态框/通知/悬浮窗）
```

### Store 职责矩阵

| Store | State 字段 | Action 数量 | 调用 IPC |
|-------|-----------|------------|----------|
| libraryStore | 4 | 5 | ✅ |
| projectStore | 4 | 6 | ✅ |
| assetStore | 6 | 10 | ✅ |
| timelineStore | 8 | 16 | ✅ |
| tagStore | 3 | 5 | ✅ |
| noteStore | 4 | 5 | ✅ |
| searchStore | 4 | 3 | ✅ |
| settingsStore | 2 | 4 | ✅ |
| uiStore | 7 | 10 | ❌（纯前端状态） |

### 验收结果

| 检查项 | 结果 |
|--------|------|
| `tsc --noEmit` 零错误 | ✅ |
| 9 个 Store 全部创建 | ✅ |
| 异步操作在 action 中执行 | ✅ |
| 不使用 localStorage | ✅ |
| 按功能域拆分 | ✅ |

---

## Step 8：Tauri IPC 通信桥

### 执行内容

1. **Rust 端**（已在 Step 6 中实现）：
   - 40 个 `#[tauri::command]` 函数
   - 全部在 `lib.rs` 的 `invoke_handler` 中注册
   - 数据库通过 `State<Database>` 注入

2. **前端封装层**：
   - `src/lib/tauri-commands.ts`：统一的 IPC 调用层
   - 按实体分组（Library/Project/Asset/Timeline/Tag/Note/Search/Settings）
   - 所有函数有完整 TypeScript 类型签名

### IPC 命令清单

| 实体 | 命令数 | 命令名称 |
|------|--------|---------|
| Library | 4 | get_libraries, create_library, update_library, delete_library |
| Project | 5 | get_projects, get_project, create_project, update_project, delete_project |
| Asset | 7 | get_assets, get_asset, create_asset, update_asset, delete_asset, toggle_asset_star, get_asset_analysis |
| Timeline | 3 | get_timeline, create_timeline |
| AudioTrack | 2 | get_audio_tracks, create_audio_track |
| Keyframe | 3 | get_keyframes, create_keyframe, delete_keyframe |
| Marker | 3 | get_markers, create_marker, delete_marker |
| Tag | 5 | get_tags, create_tag, delete_tag, link_tag_to_asset, get_asset_tags |
| Note | 5 | get_notes, get_note, create_note, update_note, delete_note |
| Search | 1 | search |
| Settings | 3 | get_setting, set_setting, get_all_settings |
| **合计** | **40** | |

### 验收结果

| 检查项 | 结果 |
|--------|------|
| Rust `cargo check` 零错误 | ✅ |
| TypeScript `tsc --noEmit` 零错误 | ✅ |
| 命令命名前后端一致 | ✅ |
| 所有返回结构可序列化 | ✅ |
| 前端封装层类型完整 | ✅ |

---

## 接口契约门禁更新

W1-A 完成后，C1 门禁的四项契约已就绪：

| 契约 | 文件 | 状态 |
|------|------|------|
| TypeScript 类型契约 | `src/types/*.ts` | ✅ 已锁定（44 个类型） |
| IPC 命令签名契约 | `src/lib/tauri-commands.ts` | ✅ 已锁定（40 个命令） |
| Zustand store 接口 | `src/stores/*.ts` | ✅ 已锁定（9 个 Store） |
| Tauri Event 命名契约 | 待 W2 定义 | 🔲 |

**Agent β（前端 UI）可以安全使用以上契约接口进行开发。**

---

## 文件结构总览（W1-A 新增）

```
项目启动/NCdesktop/
├── src/
│   ├── types/                     # ← Step 5 新增
│   │   ├── index.ts
│   │   ├── library.ts
│   │   ├── project.ts
│   │   ├── timeline.ts
│   │   ├── asset.ts
│   │   ├── common.ts
│   │   ├── export.ts
│   │   ├── settings.ts
│   │   ├── sync.ts
│   │   ├── llm.ts
│   │   └── ui.ts
│   ├── lib/                       # ← Step 8 新增
│   │   └── tauri-commands.ts
│   └── stores/                    # ← Step 7 新增
│       ├── index.ts
│       ├── libraryStore.ts
│       ├── projectStore.ts
│       ├── assetStore.ts
│       ├── timelineStore.ts
│       ├── tagStore.ts
│       ├── noteStore.ts
│       ├── searchStore.ts
│       ├── settingsStore.ts
│       └── uiStore.ts
├── src-tauri/src/
│   ├── models/                    # ← Step 6 新增
│   │   ├── mod.rs
│   │   ├── library.rs
│   │   ├── project.rs
│   │   ├── asset.rs
│   │   ├── timeline.rs
│   │   ├── common.rs
│   │   └── settings.rs
│   ├── db/                        # ← Step 6 新增
│   │   ├── mod.rs
│   │   ├── migration.rs
│   │   ├── library.rs
│   │   ├── project.rs
│   │   ├── asset.rs
│   │   ├── timeline.rs
│   │   ├── tag.rs
│   │   ├── note.rs
│   │   ├── search.rs
│   │   └── settings.rs
│   ├── commands/                  # ← Step 6/8 新增
│   │   ├── mod.rs
│   │   ├── library.rs
│   │   ├── project.rs
│   │   ├── asset.rs
│   │   ├── timeline.rs
│   │   ├── tag.rs
│   │   ├── note.rs
│   │   ├── search.rs
│   │   └── settings.rs
│   ├── utils/
│   │   └── mod.rs
│   └── lib.rs                     # ← 更新：数据库初始化 + 40 命令注册
└── Cargo.toml                     # ← 更新：新增 rusqlite, uuid, chrono
```

---

## 后续 AI 阅读指引

如果你是 **Agent β（前端 UI）**，请重点阅读：
1. `src/types/index.ts` — 所有可用类型
2. `src/stores/index.ts` — 所有 Store 的 state 和 action
3. `src/lib/tauri-commands.ts` — IPC 调用层（Store 已封装，一般不需直接调用）

如果你是 **Agent γ（独立功能）**，Step 18（全局悬浮窗）可参考：
1. `src/types/ui.ts` 中的 `DropzoneState` / `DropzoneItem`
2. `src/stores/uiStore.ts` 中的 `dropzone` 状态和 `setDropzone` action

**下一步建议**：
- W1-B（Agent β）：开始 Step 9（知识库/项目列表 UI）→ 10 → 11
- W2（Agent α 续）：等待 Step 8 完成后可启动 Step 12（TF 卡同步引擎）、Step 13（音频处理核心）
