# NCdesktop v1.1 文件转换 — 架构补救方案

**日期**：2026-05-12
**输入**：`product/prd/文件转换v1.1_prd_v1.md`
**对比对象**：`项目启动/NCdesktop` 当前代码
**结论**：管线零件已基本写就，主干电源未通；本文档梳理需补功能与执行顺序，**不进入开发**。

---

## 0. 问题根因（一句话）

`assets.source_asset_id` 列缺失 → `Asset` struct 字段缺失 → `extraction::scheduler` 无法编译 → 被 `extraction/mod.rs:3-4` 整模块禁用 → 物化、衍生 Asset、`notecapt/asset-converted` 事件全部沉默；前端"转换自"标记与监听也未实现。docx/pptx 拖入还在 `dropzone.rs` MIME 表落入 `other`；ASR 注册的是云端 iflytek 实现，违反 PRD §4"无网络上传"硬约束。

---

## 1. 现状分级

| 状态 | 含义 |
|---|---|
| ✅ DONE | PRD 验收标准已满足 |
| ⚠️ DEVIATION | 实现存在但偏离 PRD 约束 |
| 🟡 WRITTEN-DARK | 代码已写但未接电（编译被禁/未注册） |
| ❌ MISSING | 代码不存在 |

| # | P0 项 | 状态 |
|---|---|---|
| 1 | 物化写出 .md | 🟡 `scheduler.rs:588-731` 完整实现，模块被禁用 |
| 2 | 创建衍生 Asset (`converted_from`) | 🟡 同上，`scheduler.rs:654-676` |
| 3 | `assets.source_asset_id` 字段 | ❌ migration、struct、CRUD 三处都缺 |
| 4 | docx 提取器 | ✅ `extractors/docx.rs`，已注册 |
| 5 | pptx 提取器 | ✅ `extractors/pptx.rs`，已注册 |
| 6 | 录音 ASR 提取器 | ⚠️ 默认注册 `audio_asr_iflytek`（云端），PRD 要求 SFSpeechRecognizer；`audio_asr.rs` 本地实现已写但未注册 |
| 7 | dropzone MIME 扩展 | ⚠️ 音频齐全，**docx/pptx 完全缺** |
| 8 | 前端"转换自 [原件名]"标记 | ❌ `AssetListView.tsx` 无 |
| 9 | 前端 `notecapt/asset-converted` 监听 | ❌ `assetStore` 无 listener |

---

## 2. Scope 警告：scheduler 当前实现 > PRD

`scheduler.rs` 实际还做了 **派生版本管理**（`derivative_version` 字段、`_versions/<id>/v{N}.md` 归档、`content_hash` 去重、`propagate_tags_to_derivative` 标签继承、`notecapt/concept-extract-requested` 联动）。PRD §3 P0 没要求这些。

**决策项（需 PM 拍板）**：
- **A. 全量恢复**（推荐如果这些代码本身在 main 历史上已稳定）：补齐所有依赖字段/表 API，一次性激活
- **B. MVP 收敛**：把 scheduler 砍到只剩"写文件 + 建衍生 Asset + emit 事件"，跳过 versioning/content_hash/标签继承；后续 v1.2 再恢复

下表任务按 **方案 A** 列出；如选 B，标记 `[A-ONLY]` 的任务可删。

---

## 3. New Tasks（不进入开发，仅登记）

### 层 1：DB Schema 与模型（阻塞所有后续）

**T-01 — assets 表加 `source_asset_id` 列**
- 文件：`src-tauri/src/db/migration.rs`
- 改动：assets 建表加 `source_asset_id TEXT`；写 `ALTER TABLE` 升级语句（处理已建库）；加 `idx_assets_source` 索引
- 验收：旧库升级后字段存在且默认 NULL；新衍生 Asset 写入后该字段非空可查

**T-02 — assets 表加 `derivative_version` 列** `[A-ONLY]`
- 文件：同上
- 改动：`derivative_version INTEGER NOT NULL DEFAULT 0`

**T-03 — `Asset` struct 补字段**
- 文件：`src-tauri/src/models/asset.rs`
- 改动：加 `source_asset_id: Option<String>`、`derivative_version: i32`（A 方案）
- 验收：scheduler 引用编译通过

**T-04 — `db::asset` CRUD 同步**
- 文件：`src-tauri/src/db/asset.rs`
- 改动：`insert` / `get_by_id` / `get_by_project` 的 SQL 列表与 row mapping 加入新字段；新增 `find_markdown_derivative(conn, source_id) -> Option<Asset>`、`update_markdown_derivative`、`set_derivative_version` 三个函数（scheduler 调用） `[后两者 A-ONLY]`
- 验收：现有 asset 单测通过；scheduler 编译通过

**T-05 — `db::extraction` 补 `set_content_hash` 与 `upsert_extraction_result`** `[A-ONLY]`
- 文件：`src-tauri/src/db/extraction.rs`
- 验收：scheduler.rs:707-721 编译通过

**T-06 — `db::tag::propagate_tags_to_derivative`** `[A-ONLY]`
- 文件：`src-tauri/src/db/tag.rs`
- 验收：scheduler.rs:694-703 编译通过

### 层 2：管线激活

**T-07 — 取消 scheduler 禁用**
- 文件：`src-tauri/src/extraction/mod.rs:3-4`
- 改动：解开 `pub mod scheduler;` 注释；删除"暂不激活"说明
- 依赖：T-01 ~ T-06
- 验收：`cargo check` 通过

**T-08 — `commands::extraction` 注册到 Tauri**
- 文件：`src-tauri/src/commands/mod.rs`、`src-tauri/src/lib.rs` 的 `invoke_handler!` 宏
- 改动：`pub mod extraction;`，把 `extract_asset / extract_project_assets / get_extraction_status / get_extracted_content / retry_extraction / get_pipeline_progress` 加入 handler
- 验收：前端可 invoke 到这些命令

**T-09 — `PipelineScheduler` 注册为 Tauri State**
- 文件：`src-tauri/src/lib.rs` 启动初始化
- 改动：`.manage(PipelineScheduler::new(...))`（参照 scheduler.rs 构造）
- 验收：`commands::extraction:9` 的 `app.state::<PipelineScheduler>()` 不 panic

**T-10 — 入库后自动入队**
- 文件：`src-tauri/src/commands/dropzone.rs::import_drop_paths`
- 改动：每个成功 insert 的 asset 后调用 `PipelineScheduler::enqueue(&app, &id)` 并 `start`
- 验收：拖入文件无需手动按钮即自动进入提取
- 注意：确认旧逻辑是否已有自动触发；若有，仅需验证未在 v1.1 路径上回退

### 层 3：格式与 ASR

**T-11 — dropzone MIME 补 docx/pptx**
- 文件：`src-tauri/src/commands/dropzone.rs:447-460` `path_asset_meta`
- 改动：
  ```
  "docx" => ("docx", "application/vnd.openxmlformats-officedocument.wordprocessingml.document")
  "pptx" => ("pptx", "application/vnd.openxmlformats-officedocument.presentationml.presentation")
  ```
- 验收：docx/pptx 拖入后 `asset_type` 非 `other`，进入提取管线

**T-12 — ASR 默认实现切换到本地 SFSpeechRecognizer**
- 文件：`src-tauri/src/extraction/extractors/mod.rs:32`
- 改动：用 `audio_asr::AudioAsrExtractor` 替换 `audio_asr_iflytek::IflytekAsrExtractor`
- 理由：PRD §3 技术约束第 3 条 + §4 数据安全"无网络上传"
- 可选：加 setting 开关让用户显式启用云端
- 验收：m4a 拖入触发 SFSpeechRecognizer；非 macOS 构建 `can_handle=false`

### 层 4：前端

**T-13 — `assetStore` 监听 `notecapt/asset-converted`**
- 文件：`src/stores/assetStore.ts`
- 改动：`useEffect`/store 初始化中 `listen('notecapt/asset-converted', (e) => { if (e.payload.projectId === currentProjectId) refetchAssets() })`
- 验收：拖入 → 等待 → MD Asset 自动出现，无需手动刷新

**T-14 — Asset TypeScript 类型补 `sourceAssetId`**
- 文件：`src/types/asset.ts`（或对应文件）
- 改动：可选字段 `sourceAssetId: string | null`

**T-15 — `AssetListView` 显示"转换自 [原件名]"**
- 文件：`src/components/features/AssetListView.tsx`
- 改动：当 `asset.assetType === 'markdown' && asset.sourceAssetId` 时，在右栏副标题渲染 `转换自 ${原件名}`；原件名通过 lookup map 从列表内同 project 的 asset 取
- 验收：可视区分原件与衍生 MD

### 层 5：非功能与测试

**T-16 — 工作区目录策略复核**
- 文件：`src-tauri/src/workspace.rs::ensure_project_workspace`
- 问题：scheduler 写盘到此处，需确认目录路径对用户可见（PRD §1 "工作区内可见可用"）
- 验收：用户在 Finder/前端工作区视图都能直接看到 .md 文件

**T-17 — 失败降级测试**
- 类型：手动 / 集成测试
- 场景：磁盘满 / 权限拒绝 / 大文件超时 → `quality_level=0` 不写 .md 不建衍生 Asset、原件入库不受影响（PRD §4 可靠性）

**T-18 — README/文档更新**
- 文件：项目 README 或 `MarkItDown_集成开发宪章` 附注
- 改动：说明 v1.1 已完成；列出非 macOS 构建下 ASR 不可用的现状

---

## 4. 执行顺序图

```
T-01 ─┐
T-02 ─┤
T-03 ─┼─► T-04 ─► T-05 ─► T-06 ─► T-07 ─► T-08 ─► T-09 ─┐
      │                                                  ├─► T-10 ─► 整链通
T-11 ─┘                                                  │
T-12 ─────────────────────────────────────────────────── ┤
T-13 ─┐                                                  │
T-14 ─┼─► T-15 ──────────────────────────────────────────┤
      │                                                  │
                                            T-16 / T-17 / T-18（并行验收）
```

**关键路径**：T-01 → T-03 → T-04 → T-07 → T-08 → T-09 → T-10。前端 T-13/14/15 可与后端并行。

---

## 5. 风险

| 风险 | 影响 | 缓解 |
|---|---|---|
| scheduler.rs 中 `db::extraction` 接口名与现有不一致 | T-05 工作量膨胀 | 实施前先对 `db/extraction.rs` 全文 grep 比对差异 |
| ALTER TABLE 在用户已有库上执行顺序 | 升级失败 | migration.rs 用 `PRAGMA table_info` 检查后再 ALTER |
| iflytek 切到本地 ASR 后用户配置/历史依赖 | 行为回退 | 保留 iflytek 模块代码，仅切默认；加 setting 开关 |
| 工作区目录非用户可见 | 体验未达 PRD | T-16 强制校验 |
| `derivative_version` 字段在选方案 B 时被强行加入 | 浪费 | 决策项先确认 A/B |

---

## 6. 决策项（待 PM 确认）

1. **scheduler 范围**：A 全量（含 versioning/hash/标签继承） vs B MVP（仅写文件 + 衍生 Asset + 事件）
2. **ASR 切换策略**：直接换默认 vs 加 setting 开关并默认本地
3. **历史 Asset 回填**：PRD §3 已列 P1 搁置，确认本次不做

---

## 7. 不在本方案内

- 历史 Asset 批量回填（P1）
- 查看器原件跳转（P1）
- Word/PPT 嵌入图片提取（P2）
- MD 手动编辑（P2）
- 双栏对照（P2）
- 音频转写质量优化（P2）
