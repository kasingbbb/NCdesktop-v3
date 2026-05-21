# MarkItDown 集成补完计划 v1.0

> **生成日期**：2026-05-12
> **对应宪章**：`MarkItDown_集成开发宪章_v1.0.md`
> **当前完成度评估**：约 15%（仅"按需手动单文件转换"骨架，自动管线完全未接通）
> **本计划性质**：架构补完 + 任务清单，**不进入开发**

---

## 一、当前真实状态（事实清单）

### 1.1 已有
- `extraction/extractors/markitdown.rs`（113 行）：作为 `Extractor` 实现，能调用 `python -m markitdown <file>`，根据 `options.markitdown_python_cmd` / `python3` / `python` 依次尝试。
- `commands/conversion.rs`（101 行）：注册了两个 Tauri 命令
  - `check_markitdown_status` —— 探测 `markitdown --version`
  - `convert_asset_to_markdown` —— 对单个 asset 同步运行 MarkItDown，返回 markdown **字符串**（不入库、不写盘、不生成衍生 asset）
- `src/lib/tauri-commands.ts:150,161` 暴露 JS helper `checkMarkitdownStatus` / `convertAssetToMarkdown`。
- `extractors/mod.rs:17 get_extractor_for` 启用 MarkItDown 优先（单层二选一）。

### 1.2 关键缺失或损坏
- **scheduler 模块被注释停用**（`extraction/mod.rs:3-4`），整个 `scheduler.rs`（814 行）是孤儿代码。
- **数据库 schema 严重落后于宪章设计**：
  - `assets` 表无 `source_asset_id`、`original_name`、`derivative_version` 字段
  - 无 `extracted_content` 表
  - 无 `pipeline_tasks` 表
  - 无 `conversion_meta` 表
- **`db/asset.rs` 缺函数**：`find_markdown_derivative` / `update_markdown_derivative` / `set_derivative_version`（被 scheduler.rs 调用但不存在）
- **`db/tag.rs` 缺函数**：`propagate_tags_to_derivative` / `sync_tags_to_canonical_derivatives`
- **`models::Asset` struct 缺字段**：`source_asset_id` / `derivative_version`
- **`commands/dropzone.rs`**：导入后只跑 AI 打标，**没有把 asset 加入提取队列**，没有自动 MD 转换。
- **前端无消费者**：`convertAssetToMarkdown` / `checkMarkitdownStatus` 在 `src/` 中无任何 import（除 `tauri-commands.ts` 自身），Inspector UI 没有"转换器/质量/重试"区域。
- **fallback 链不存在**：MarkItDown 失败时不会自动回退到 `pdf_text` / `docx` / `pptx`。
- **macOS Vision OCR / 音频 ASR**：`pdf_scan` / `image_ocr` / `audio_asr` 模块在 `extractors/mod.rs:5-7` 被注释。

---

## 二、Architect 决策（与原宪章的差异说明）

原宪章按 Step 1→2→3→…→7 串行推进，但当前代码状态决定了**实际依赖链**必须重排，否则会反复返工：

```
W0 (前置)  数据库 schema 升级 + Asset model 扩字段
   │
W1 (基础) db::asset / db::tag 补缺函数
   │
W2 (恢复) 恢复 scheduler 模块编译 + extracted_content / pipeline_tasks 表
   │
W3 (接通) dropzone 落库后 enqueue → scheduler 主循环 → materialize_md 物化
   │
W4 (链路) MarkItDown→内置 fallback 链 + conversion_meta 表
   │
W5 (前端) Inspector 转换信息面板 + 重新转换按钮 + 状态指示
   │
W6 (下游) 知识抽取/搜索改读 canonical markdown
```

**关键差异**：
- 原宪章 Step 1（标签继承）单独排首，但其依赖 `Asset.source_asset_id` 字段 + 物化后的衍生 asset，必须先有 W0+W2 才能验收。本计划把标签传播合并进 W3。
- 原宪章 Step 3 的 `Converter` trait 抽象在当前已有 `Extractor` trait 基础上**不重新引入**——直接复用 `Extractor`，避免双套接口并存。`ConversionResult` 用作 `extracted_content` + `conversion_meta` 的写入字段，不作为 trait 返回值。
- 原宪章 Step 4 的"独立 `markitdown_adapter`"在当前已实现为 `extractors/markitdown.rs`，**不重写**，只补健康检查缓存 + 错误归类。

---

## 三、新任务清单（W0 → W6）

### W0 · 数据库与模型升级 ⏱ 1 天 · 优先级 P0

| # | 任务 | 文件 | 验收 |
|---|---|---|---|
| W0-1 | 新迁移：`ALTER TABLE assets ADD COLUMN source_asset_id TEXT REFERENCES assets(id) ON DELETE SET NULL` | `db/migration.rs` | 启动后查询 `PRAGMA table_info(assets)` 有该列 |
| W0-2 | 新迁移：`ADD COLUMN original_name TEXT NOT NULL DEFAULT ''`、`ADD COLUMN derivative_version INTEGER NOT NULL DEFAULT 0` | `db/migration.rs` | 同上 |
| W0-3 | `models::Asset` 加 `source_asset_id: Option<String>` / `derivative_version: i32` 字段 + 序列化 | `models/asset.rs` | 编译通过 |
| W0-4 | `db::asset::insert` / `update` / `get_by_*` 全部对齐新字段（row.get 索引修正） | `db/asset.rs` | 单测拖入文件成功，无 panic |

### W1 · 数据库层补函数 ⏱ 0.5 天 · 优先级 P0

| # | 任务 | 文件 | 验收 |
|---|---|---|---|
| W1-1 | `db::asset::find_markdown_derivative(conn, root_id) -> Option<Asset>` | `db/asset.rs` | 单测：插入原件 + 衍生件，查询返回衍生件 |
| W1-2 | `db::asset::update_markdown_derivative(conn, id, file_size, imported_at)` | `db/asset.rs` | 单测：更新后字段变化 |
| W1-3 | `db::asset::set_derivative_version(conn, id, version)` | `db/asset.rs` | 同上 |
| W1-4 | `db::tag::propagate_tags_to_derivative(conn, root_id, derived_id)` —— `INSERT OR IGNORE` 子查询 | `db/tag.rs` | 单测：原件 3 标签传播后衍生件有同样 3 标签 |
| W1-5 | `db::tag::sync_tags_to_canonical_derivatives(conn, root_id)` | `db/tag.rs` | 单测：原件后续加标签，所有衍生件同步获得 |

### W2 · 恢复 scheduler 模块 ⏱ 2 天 · 优先级 P0

| # | 任务 | 文件 | 验收 |
|---|---|---|---|
| W2-1 | 新迁移：`extracted_content` 表（status/extractor_type/structured_md/quality_level/needs_ocr_fallback/error_class/created_at/updated_at） | `db/migration.rs` | 表存在，索引齐 |
| W2-2 | 新迁移：`pipeline_tasks` 表（id/asset_id/status/retry_count/last_error/created_at/updated_at） | `db/migration.rs` | 同上 |
| W2-3 | 新建 `db/extraction.rs`：`upsert_extraction_result` / `set_task_status` / `enqueue_task` / `pop_pending_task` | `db/extraction.rs` | 单测：任务入队后能查到 pending |
| W2-4 | `Cargo.toml` 增加 `sha2 = "0.10"` 依赖 | `Cargo.toml` | `cargo check` 通过 |
| W2-5 | 取消 `extraction/mod.rs:4` 的 `pub mod scheduler;` 注释 | `extraction/mod.rs` | 编译通过（依赖 W0/W1 全部完成）|
| W2-6 | scheduler.rs 内逐处对齐：`models::Asset` 新字段、连接获取方式、错误类型 | `extraction/scheduler.rs` | `cargo check` 0 错误 |

### W3 · 接通自动管线 ⏱ 1.5 天 · 优先级 P0

| # | 任务 | 文件 | 验收 |
|---|---|---|---|
| W3-1 | `dropzone.rs` 落库后调用 `PipelineScheduler::enqueue(asset_id)` | `commands/dropzone.rs` | 拖入文件后 `pipeline_tasks` 有新记录 |
| W3-2 | scheduler `start()` 后台循环（已存在）启动入口接入 `lib.rs setup` | `lib.rs` | 应用启动后日志显示调度器 running |
| W3-3 | `materialize_md` 物化成功后调用 `db::tag::propagate_tags_to_derivative` | `extraction/scheduler.rs` | E2E：拖 PDF → 等待 → 衍生 `.md` 出现且继承原件标签 |
| W3-4 | AI 打标完成后调用 `db::tag::sync_tags_to_canonical_derivatives` | `commands/dropzone.rs` | E2E：AI 后补打的标签也同步到衍生件 |
| W3-5 | 物化幂等：同一原件重提取不产生第二个 `.md` 文件 + UUID | `extraction/scheduler.rs` | E2E：手动重触发 2 次，工作区只有 1 个 md |

### W4 · MarkItDown→内置 fallback 链 + conversion_meta ⏱ 1.5 天 · 优先级 P1

| # | 任务 | 文件 | 验收 |
|---|---|---|---|
| W4-1 | 新迁移：`conversion_meta`（source_asset_id/derived_asset_id/converter_name/converter_version/source_mime/source_hash/quality_level/fallback_used/error_class/conversion_ms/converted_at） | `db/migration.rs` | 表与索引存在，UNIQUE(source_asset_id, converter_name) |
| W4-2 | `db/conversion_meta.rs`：`upsert` / `get_by_source` | 新文件 | 单测通过 |
| W4-3 | scheduler 主循环改造：`should_use_markitdown(mime) → try markitdown → 失败/空输出 fallback get_fallback_extractor_for` | `extraction/scheduler.rs` | E2E：禁用 markitdown（重命名 python 包）后 PDF 仍能被 `pdf_text` 处理 |
| W4-4 | 每次转换写 `conversion_meta`：成功 / 失败 / fallback 三种情形 | `extraction/scheduler.rs` | DB 查询有 quality_level、fallback_used 等字段值 |
| W4-5 | `markitdown.rs` 增加错误归类：`classify_error(stderr)` 返回 `file_not_found` / `permission_denied` / `unsupported_format` / `markitdown_not_installed` / `conversion_error` | `extractors/markitdown.rs` | 单测：mock stderr 输入返回对应分类 |
| W4-6 | `MarkItDown` 健康检查结果缓存 5 分钟，避免每次 convert 都重新探测 | `extractors/markitdown.rs` 或新建 state 单例 | 日志查得只探测一次 |

### W5 · 前端透传与可观测性 ⏱ 1.5 天 · 优先级 P1

| # | 任务 | 文件 | 验收 |
|---|---|---|---|
| W5-1 | 新 Tauri 命令 `get_conversion_meta(asset_id)` → 返回 `ConversionMetaRow` | `commands/conversion.rs` | invoke 返回完整字段 |
| W5-2 | 新 Tauri 命令 `retrigger_extraction(asset_id)`：重置 `extracted_content.status='queued'` 和 `pipeline_tasks` | `commands/conversion.rs` | 点击后 scheduler 拾起任务 |
| W5-3 | Inspector 新增"转换信息"区块：转换器名/版本/质量等级图标/耗时/失败原因 | `src/components/layout/Inspector.tsx` | 可视化显示，失败有清晰原因文字 |
| W5-4 | Inspector 顶部状态栏显示 MarkItDown 健康状态（绿/黄/红） | `src/components/layout/Inspector.tsx` 或 `Toolbar.tsx` | 安装/未安装两种状态正确切换 |
| W5-5 | "重新转换"按钮调用 `retrigger_extraction` + Toast 反馈 | Inspector | 点击后状态变 "queued" → "extracting" → "extracted" |

### W6 · 知识下游对齐 ⏱ 1 天 · 优先级 P2

| # | 任务 | 文件 | 验收 |
|---|---|---|---|
| W6-1 | 搜索：对有 `source_asset_id` 的 markdown 命中时显示原件元数据 | `commands/search.rs` + `db/search.rs` | 搜索关键词返回带原件名/图标的结果 |
| W6-2 | 搜索去重：同一原件不返回原件+衍生件两条 | 同上 | 单测：1 个 PDF 关键词只回 1 条 |
| W6-3 | 知识抽取入口优先读 canonical markdown 衍生件 | `commands/knowledge*.rs` | 抽取日志显示读取的是 `.md` 而非 PDF 二进制 |

---

## 四、风险与前置阻塞

### 4.1 必须先解决的根因
1. **scheduler.rs 是定时炸弹**：不解决就无法做任何自动管线工作。W0 + W1 + W2 必须连成一气，否则中间任何一步留 stub 都会导致编译失败。
2. **schema 迁移顺序**：现网用户 DB 已有 assets 表，迁移必须用 `ALTER TABLE ADD COLUMN`，不能 `DROP/RECREATE`。
3. **macOS Vision FFI 仍停用**：`pdf_scan` / `image_ocr` / `audio_asr`（macOS）在 `extractors/mod.rs:5-7` 注释中。若产品上要保留扫描 PDF → OCR 链路，需单独排期恢复 Swift bridge，**本计划不含**。

### 4.2 已有的可观测风险
- `convert_asset_to_markdown` 当前是同步阻塞主线程的 Tauri 命令，大文件会卡 UI。W3 接通后应弃用此命令或改为异步。
- `MarkItDownExtractor::extract` 默认会等待 Python 子进程完成，无超时；恶意/损坏文件可能让进程卡死。建议 W4 加 `Command::output()` 超时（如 90s）。

---

## 五、估时与里程碑

| 里程碑 | 内容 | 工时 | 累计 |
|---|---|---|---|
| **M0：数据底座** | W0 + W1 | 1.5 天 | 1.5 天 |
| **M1：管线复活** | W2 + W3 | 3.5 天 | 5 天 |
| **M2：链路与可观测** | W4 + W5 | 3 天 | 8 天 |
| **M3：下游对齐** | W6 | 1 天 | 9 天 |

**关键里程碑验收**：
- M0 完成 ⇒ `cargo check` 0 错误，数据库迁移可重入。
- M1 完成 ⇒ 拖入 PDF → 自动出现同目录 `.md` 衍生件且标签继承。
- M2 完成 ⇒ Inspector 显示"由 markitdown 转换，质量 2，耗时 850ms"；禁用 Python 后自动 fallback 不报错。
- M3 完成 ⇒ 搜索/知识抽取统一基于 markdown 衍生件。

---

## 六、不在本计划范围

- 重写 `Extractor` 为宪章建议的 `Converter` trait（视为过度设计，沿用现有 trait）。
- 内嵌 Python 运行时打包（已在 `内置Python_MarkItDown_DMG打包开发宪章_v1.0.md` 中规划，独立推进）。
- 知识进化系统、合成错误诊断（独立路线）。
- macOS Vision OCR FFI 恢复。

---

## 七、文件索引（确认改动面）

| 路径 | 改动量 |
|---|---|
| `src-tauri/src/db/migration.rs` | 新增 3 条迁移 |
| `src-tauri/src/db/asset.rs` | +3 函数，对齐新字段 |
| `src-tauri/src/db/tag.rs` | +2 函数 |
| `src-tauri/src/db/extraction.rs` | 新文件 |
| `src-tauri/src/db/conversion_meta.rs` | 新文件 |
| `src-tauri/src/models/asset.rs` | +2 字段 |
| `src-tauri/src/extraction/mod.rs` | 取消 scheduler 注释 |
| `src-tauri/src/extraction/scheduler.rs` | 对齐字段 + 加 fallback 主循环 |
| `src-tauri/src/extraction/extractors/markitdown.rs` | +错误归类、+健康检查缓存、+超时 |
| `src-tauri/src/extraction/extractors/mod.rs` | （可选）`get_extractor_for` 改为不在此处做 fallback，由 scheduler 控制 |
| `src-tauri/src/commands/dropzone.rs` | 落库后 enqueue + AI 后 sync_tags |
| `src-tauri/src/commands/conversion.rs` | +`get_conversion_meta` + `retrigger_extraction`，弃用旧 `convert_asset_to_markdown` |
| `src-tauri/src/lib.rs` | 注册新命令 + 启动 scheduler |
| `src-tauri/Cargo.toml` | +`sha2 = "0.10"` |
| `src/components/layout/Inspector.tsx` | 转换信息面板 + 重试按钮 |
| `src/lib/tauri-commands.ts` | +新 helper、清理旧 helper |

---

**完。下一步等待用户确认计划后再进入开发。**
