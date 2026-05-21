# MarkItDown 集成 — 待补功能清单 v1.0

> **目的**：基于 `MarkItDown_集成开发宪章_v1.0.md` 与 `MarkItDown_文件格式转化迭代规划宪章_v1.0.md`，对照当前 `项目启动/NCdesktop/src-tauri/` 实际代码，列出**仍需补齐**的功能、当前所处的具体阻塞点，以及推荐的实施顺序。
>
> **review 日期**：2026-05-12
> **代码基线**：`项目启动/NCdesktop/src-tauri/`（注意：该目录在 git 中尚未跟踪，是一份未提交的 WIP）
> **结论摘要**：宪章 Step 1/2/4 已经写了一半但**当前无法编译**；Step 3 完全没做；Step 5 的真正 fallback 链路缺失；Step 6 缺数据库支撑。

---

## 一、当前状态总览

| 宪章步骤 | 主题 | 状态 | 说明 |
|---------|------|------|------|
| Step 1 | 标签继承到衍生 .md | ⚠️ 一半 | scheduler 已调用，但 `db::tag` 里函数未实现；dropzone AI 路径未接 |
| Step 2 | 物化幂等 | ⚠️ 一半 | scheduler 已实现 find/update 流程，但 `db::asset` 里函数未实现，`Asset` 模型缺字段 |
| Step 3 | Converter 抽象 + conversion_meta 表 | ❌ 未做 | 没有 `extraction/conversion.rs`，没有迁移 |
| Step 4 | MarkItDown 适配层 | ✅ 大部分 | 子进程调用、健康检查、embedded venv 探测都有 |
| Step 5 | 接入主路径 + fallback | ⚠️ 缺核心 | MarkItDown 失败时**没有回退到 pdf_text/docx/pptx** |
| Step 6 | 前端透传 + 重新转换 | ⚠️ 一半 | Inspector 显示 extractor/quality；缺 conversion_meta、缺 retrigger 命令 |
| Step 7 | 知识下游对齐 | ❓ 未在本次 review | 留待单独审计 |

---

## 二、阻塞性问题（必须先解决，否则无法 `cargo check` 通过）

### 阻塞 #1：`Asset` 模型缺字段
- 文件：`src-tauri/src/models/asset.rs`
- 缺失：`source_asset_id: Option<String>`、`derivative_version: i32`
- 影响：`scheduler.rs` 第 514/610/635/669/670 行直接访问这些字段；编译失败。

### 阻塞 #2：数据库迁移缺列
- 文件：`src-tauri/src/db/migration.rs`
- 缺失：`assets` 表无 `source_asset_id`、`derivative_version` 列；无对应索引（`idx_assets_source_asset_id`）。
- 影响：即便 model 改完，运行时 INSERT/SELECT 也会跑挂。

### 阻塞 #3：`db/asset.rs` 缺函数
- 文件：`src-tauri/src/db/asset.rs`
- 缺失：`find_markdown_derivative`、`update_markdown_derivative`、`set_derivative_version`。
- 影响：`scheduler.rs:627 / 678 / 691 / 692` 调用未定义符号；编译失败。
- 附带：现有 `insert / update / get_by_*` 需要扩展 SELECT/INSERT 列表以包含新字段。

### 阻塞 #4：`db/tag.rs` 缺函数
- 文件：`src-tauri/src/db/tag.rs`
- 缺失：`propagate_tags_to_derivative`、`sync_tags_to_canonical_derivatives`。
- 影响：`scheduler.rs:695` 调用未定义符号；编译失败。
- 实现要点：`INSERT OR IGNORE` 保证幂等；按 `source_asset_id` 联结找 canonical markdown 衍生件。

---

## 三、功能性缺口（编译通过之后必须做）

### F1. AI 打标完成后同步标签到已存在衍生件
- 文件：`src-tauri/src/commands/dropzone.rs`
- 位置：`apply_llm_classify_to_asset` 中 `db::tag::link_to_asset` 调用之后
- 做什么：调用 `db::tag::sync_tags_to_canonical_derivatives(&conn, &asset.id)`
- 为什么必要：用户拖入后 AI 异步打标，而 .md 衍生件可能此前已生成 → 不同步会重现"标签只挂原件"的老 bug。

### F2. MarkItDown 失败回退链路（**最大用户感知缺口**）
- 文件：`src-tauri/src/extraction/extractors/mod.rs` 或 `scheduler.rs` 选择层
- 现状：`get_extractor_for` 命中 MIME 直接返回 `MarkItDownExtractor`；一旦返回 `Err`，调用方拿到失败就结束。
- 期望：MarkItDown 报错或输出空 → 自动走 `get_fallback_extractor_for(mime_type)`（pdf_text / docx / pptx）；都失败再走 placeholder。
- 推荐做法：把"选择 + 调用 + 回退 + 元数据落库"提到 scheduler 主循环里，不要塞在 `get_extractor_for` 内部。

### F3. `conversion_meta` 表与持久化
- 文件：`src-tauri/src/db/migration.rs`（建表）+ 新文件 `src-tauri/src/db/conversion_meta.rs`（CRUD）
- 字段（参考宪章 Step 3.2）：
  - `id`、`source_asset_id`、`derived_asset_id`、`converter_name`、`converter_version`、`source_mime`、`source_hash`、`quality_level`、`fallback_used`、`error_class`、`conversion_ms`、`converted_at`
  - 约束：`UNIQUE(source_asset_id, converter_name)`
- 接入点：scheduler 在每次 MarkItDown 调用 + fallback 调用后 `upsert` 一行。

### F4. `Converter` trait 与 `ConversionResult` 抽象
- 新文件：`src-tauri/src/extraction/conversion.rs`
- 内容：定义 `ConversionResult`（含 converter/quality/fallback/error_class/conversion_ms）+ `Converter` trait + `file_sha256` 工具函数。
- 关系：作为 `Extractor` trait 之上的薄层；现有 Extractor 可以在 scheduler 处适配产出 `ConversionResult`，不要求一刀切替换。

### F5. `error_class` 归类与版本缓存
- 文件：`src-tauri/src/extraction/extractors/markitdown.rs`
- 现状：错误直接以 stderr 字符串外抛；版本未缓存。
- 期望：
  - 新增 `classify_error(stderr)` → `file_not_found | permission_denied | unsupported_format | markitdown_not_installed | conversion_error`
  - 适配器创建时一次性 probe 版本，缓存于 struct；后续 `convert()` 用缓存值填 `conversion_meta.converter_version`。

### F6. 主动重跑命令 `retrigger_extraction`
- 文件：`src-tauri/src/commands/extraction.rs` 或 `commands/conversion.rs`
- 行为：重置 `extracted_content.status = 'queued'`，重置/重入 `pipeline_tasks`，唤醒 scheduler。
- 前端联动：`InspectorExtraction.tsx` 的"重试"按钮目前调用 `retryExtraction(asset.id)`（已在 `extractionStore`），需要验证它在 failed/extracted 两种状态下都能干净重跑；如已经能，则本任务降级为"对齐 + 单测"。

### F7. 前端展示转换元数据
- 文件：`src/components/layout/InspectorExtraction.tsx`、`src/lib/tauri-commands.ts`、`src/stores/extractionStore.ts`
- 现状：已经展示 `extractor_type` 与 `qualityLevel`，并能复制 markdown。
- 缺口：
  - 没有展示 `fallback_used`（"已自动回退到内置 PDF 提取器"提示）
  - 没有展示 `conversion_ms`、`error_class`、`converter_version`
- 依赖：F3（`conversion_meta` 表必须先有）+ 新增 `get_conversion_meta(assetId)` 命令。

### F8. 资产家族 / 知识下游消费 canonical markdown
- 范围：搜索（全文 + 标签筛选）、知识抽取入口
- 待审计点：
  - 搜索命中 canonical markdown 时如何关联回原件（避免一个原件出两条结果）
  - 知识抽取读取的是 `extracted_content.structured_md` 还是 canonical 衍生 .md 的真实文件内容？
  - F-7/F-8 增量抽取 (notecapt/concept-extract-requested 事件) 已经在 scheduler 中 emit，前端是否真的监听？
- 建议：在 Step 1-6 完成后单独跑一次 Step 7 专项 review。

---

## 四、当前实现里**超出宪章**且建议保留的部分

这些不在宪章里，但已经在代码中实现，且有架构价值：

1. **`derivative_version` 单调递增 + `_versions/<source_id>/vN.md` 归档**（`scheduler.rs:534-558, 610, 691-692`）
   - 比宪章"覆盖式更新"更稳，可在前端做历史版本对照。
   - 但需要在 Asset 模型 + migration 里把 `derivative_version` 字段补齐才能落地。

2. **YAML frontmatter 注入**（`build_frontmatter`，`scheduler.rs:521-532`）
   - 衍生 .md 头部带 `source_asset_id / version / extractor_type / quality_level`，可读性 + 可追溯性都好。

3. **`content_hash` 同时写到 source + derivative**（`scheduler.rs:720-721`）
   - 为知识抽取增量判重（F-7/F-8）提供干净的 hash 基线。
   - 与宪章 Step 3 提的 `source_hash` 字段语义重合，建议在 `conversion_meta` 落地时复用而不是再起一个字段。

4. **失败 / 不支持 → `materialize_placeholder` 占位 .md**（`scheduler.rs:769-791`）
   - 保证"每个原件都有 .md 邻居"，下游搜索/标签视图不会出现孤儿。
   - 但目前 placeholder 也会推进 `derivative_version`，需要审一下是否会和真正成功后的版本号冲突。

5. **嵌入式 venv 探测**（`detect_embedded_markitdown_python`，`scheduler.rs:567-579`）
   - 对应 DMG 内嵌 Python 打包路径；正式版策略已经有承载。

---

## 五、推荐实施顺序

```
M1（让它先编过）
  T1  补 Asset 模型字段 + 迁移列
  T2  补 db/asset.rs 的 3 个函数
  T3  补 db/tag.rs 的 2 个函数
  T4  dropzone 接 sync_tags_to_canonical_derivatives
  ✓ 验收：cargo check 通过 + 拖入 PDF 后 .md 自动继承标签

M2（架构稳态）
  T5  新建 extraction/conversion.rs（ConversionResult + Converter trait + file_sha256）
  T6  新建 conversion_meta 表 + db/conversion_meta.rs CRUD
  T7  MarkItDown 适配器加版本缓存 + error_class
  ✓ 验收：转换完成后能在 sqlite 里查到一行 conversion_meta

M3（真正的可靠性）
  T8  scheduler 主循环改造：MarkItDown 失败 → 自动 fallback → 都失败 → placeholder
  T9  每条路径都写 conversion_meta（含 fallback_used）
  ✓ 验收：卸载 markitdown 后拖 PDF，自动走 pdf_text，extractor_type=pdf_text，fallback_used=true

M4（可观测面）
  T10 新增 get_conversion_meta 命令
  T11 InspectorExtraction 展示 fallback / 耗时 / 错误类别 / 转换器版本
  T12 验证或新增 retrigger_extraction 命令，统一前端"重试"语义
  ✓ 验收：UI 上能区分"成功 / 已 fallback / 失败"三态

M5（知识下游）
  T13 单独 review 搜索 + 知识抽取链路，确认基于 canonical markdown 工作
  ✓ 验收：以 canonical markdown 为唯一真相源，新增格式不再触动知识模块
```

**严格依赖**：T1-T4 必须先做完，否则后面任何改动都无法编译验证。T8 依赖 T5-T7，因为 fallback 失败信息需要落到 `conversion_meta`。

---

## 六、风险与边界

1. **不要在 T8 之前合并到 main**：当前 scheduler.rs 引用未定义符号，main 上一旦 merge 会立即破坏 CI。
2. **`derivative_version` 与 `placeholder` 的版本冲突**：placeholder 也会推进版本号，要确认"真转换成功"覆盖 placeholder 时不会丢失最初的 placeholder 历史。
3. **knowledge 系统已 GA**：上一次提交 `184c6c0` 已把知识进化系统铺开，所以 Step 7 是真改造而不是新建，影响面更大，必须独立 review。
4. **src-tauri 未跟踪**：当前 review 基于工作区 WIP；如果有别处 worktree（如 `.claude/worktrees/`）含更新版本，本清单需要重新对照。

---

## 七、最终判断标准（沿用规划宪章 §15）

本轮迭代算成功，看三点：

1. 上传任意主流学习文档（PDF/DOCX/PPTX）后，都能稳定得到可读 Markdown 衍生件。
2. 衍生件不是孤儿，自动继承原件标签，并在工作区视图与 TagTree 中可见。
3. 知识抽取消费的是 canonical Markdown，而不是各格式 raw_text 的细节。

— END —
