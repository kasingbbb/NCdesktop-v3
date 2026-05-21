# 技术方案 — markitdown_rescue

> **角色**：Architect Worker
> **输入**：宪章 v1.0（开发宪章 + 迭代规划宪章）+ session_context.md + 现有代码库 review
> **状态**：方案已定，已进入 Dev 循环（task_002 PASS）

---

## ⚠️ Correction Note（2026-05-12，task_002 Reviewer 发现）

§〇.1（需求复述）中"scheduler.rs 引用未定义符号 → cargo check 必然失败"的判断**是错的**。真实情况：`src/extraction/mod.rs:4` 当前是 `// pub mod scheduler;`（注释状态），整个 scheduler.rs 不参与编译。因此先前 review 看到的"4 个未实现底层函数 + 2 个未实现 Asset 字段"并不会触发编译错误，只是 dead-code 状态。

这**不否定**本方案的有效性，但带来 1 个跨 task 待办（M-1）：
- task_008 必须在动手前**取消 mod.rs:4 注释**，让 scheduler 重新参与编译；
- 取消注释后预期会暴露 task_002~007 已经铺好的底层符号——若仍有缺失，按 ESCALATE 处理。

M-1 已分别回填到 task_003 / task_004 / task_008 input.md。

---

---

## 〇、Architect 思考协议（内部推理记录）

### 0.1 需求理解复述（用自己的话）

宪章描述的目标是把"文件 → Markdown"这条链路从**一堆按格式分散的提取器**升级为**统一转换管线**，使下游搜索/知识抽取只消费 canonical markdown。但当前代码处于一个尴尬状态：

- `scheduler.rs` 已经按统一管线的形状重写了 `write_derivative_md`，引入了 `derivative_version` 版本归档、frontmatter 注入、content_hash 双写——这些都是**超出宪章**的好东西；
- 但它依赖的 4 个底层函数（`db::tag::propagate_tags_to_derivative`、`db::asset::{find_markdown_derivative, update_markdown_derivative, set_derivative_version}`）和 2 个 `Asset` 字段（`source_asset_id`、`derivative_version`）**全都没实现**——`cargo check` 必然失败；
- 同时 MarkItDown 失败时**没有 fallback 链路**（`get_extractor_for` 命中即返回，调用方拿到 `Err` 就结束），这恰好是用户最容易感知的稳定性缺口；
- `conversion_meta` 表完全不存在，所以 fallback_used / 错误类别 / 耗时 / 转换器版本这些事实没有任何落库。

**因此本次迭代的真实任务不是"接入 MarkItDown"**（适配器已经写了 80%），而是：

1. **先把代码补回能编**（地基缺口），
2. **再把 fallback 链路真正接上**（用户感知缺口），
3. **最后把元数据持久化 + UI 透传补上**（可观测性缺口）。

### 0.2 约束识别（从 session_context.md 提取）

| 约束 | 来源 | 设计影响 |
|------|------|---------|
| 标签传播只能有一处实现 | 不可妥协底线 #3 + 可维护性 | 必须建 `db::tag::propagate_*` / `sync_*` 公共函数；禁止 dropzone/scheduler/inspector inline 实现 |
| 同一 root 唯一 canonical .md | 不可妥协底线 #2 | DB 层级 `(source_asset_id, asset_type='markdown')` 查询路径必须返回最多 1 行 |
| MarkItDown 失败必须 fallback | 不可妥协底线 #4 | `get_extractor_for` 不能再"命中即终结"，要么改逻辑要么在 scheduler 主循环外做选择 |
| 迁移仅向后兼容 | 代码规范 | `assets.source_asset_id` / `derivative_version` 必须用 `ALTER TABLE ADD COLUMN`，**默认值不能 NULL 否则旧行 SELECT 会炸** |
| 子进程参数化 | 安全 | 已有 `Command::args(...)`，复审不要回退到字符串拼接 |
| 错误信息脱敏 | 安全 | stderr 不直接给前端，前端只展示 `error_class` |

### 0.3 风险扫描

| 风险 | 概率 | 影响 | 缓解 |
|------|------|------|------|
| `Asset` 加字段后 30+ 处 `Asset { ... }` 字面量构造全部需要更新 | 高 | 高（编译爆裂） | T2 完成前禁止合并其他 task；用 `#[serde(default)]` 减少破坏面 |
| `derivative_version` 在 placeholder 与真成功路径间错乱 | 中 | 中 | T8 必须在写 placeholder 时**不推进版本号**，只在真成功时推进 |
| 现有 `materialize_placeholder` 会让 placeholder 写入 `extracted_content` 占住 status=extracted，导致后续真成功被认为"已完成"跳过 | 中 | 高 | T8 验收用例必须覆盖 "placeholder → 真成功"链路 |
| 数据库迁移在已有 sqlite 文件上跑 `ADD COLUMN` 是否会撞已存在的列 | 低 | 中 | 用 `PRAGMA table_info` 守卫或忽略 `duplicate column` 错误 |
| Python 子进程在 macOS 沙盒（DMG 打包后）找不到 PATH 中的 python3 | 中 | 高 | 已有 `detect_embedded_markitdown_python`；T7 必须把 embedded 路径作为最高优先级而非"补充" |
| MarkItDown stderr 含用户文件路径 → 错误日志泄露 | 低 | 低 | `classify_error` 之后只暴露 enum，不外传 stderr 原文 |
| 知识抽取系统（上一次 commit 184c6c0 已 GA）当前消费哪种数据源未知 | 中 | 高 | T12 单独审计，不在本轮 Dev 闭环内 |

### 0.4 技术决策点清单

需要 ADR 锁定的点：

- ADR-001：`Asset` 模型扩展方式（加字段 vs 新表 `asset_derivative_meta`）
- ADR-002：`derivative_version` 的归属（写在 source 还是 derivative 还是双写）
- ADR-003：fallback 链路触发位置（`get_extractor_for` 内 vs scheduler 主循环）
- ADR-004：`conversion_meta` 与 `extracted_content` 的边界（合并 vs 并列）
- ADR-005：MarkItDown 适配器结构（保留 `Extractor` trait vs 新建 `Converter` trait）
- ADR-006：placeholder 是否推进版本号

---

## 一、项目概述

补完 NCdesktop 当前 WIP 的 MarkItDown 集成。先恢复编译，再补真正的 MarkItDown→内置 fallback 链路与转换元数据持久化，最后把状态透传到 Inspector。**不重写已经存在的合理设计**（derivative_version 归档、frontmatter 注入、content_hash 双写、embedded venv 探测均保留）。

---

## 二、技术选型

| 选项 | 决策 | ADR |
|------|------|-----|
| 数据模型扩展 | `Asset` 表加 2 列 + 新建 `conversion_meta` 表 | ADR-001 / ADR-004 |
| 版本号归属 | 同时写 source 和 derivative，但只在真成功路径推进 | ADR-002 / ADR-006 |
| Fallback 触发层 | 在 scheduler 主循环，**不**在 `get_extractor_for` 内 | ADR-003 |
| 转换器抽象 | 继续用 `Extractor` trait，**不**新建 `Converter` trait | ADR-005 |
| MarkItDown 调用 | 子进程 + 嵌入式 venv 优先 + python3 fallback | 继承现有 |
| 错误模型 | 新增 `error_class: enum` 用于 UI；stderr 仅入日志 | 继承约束 |

---

## 三、Architecture Decision Records

### ADR-001：Asset 模型扩展方式

- **状态**：已接受
- **上下文**：scheduler 引用 `asset.source_asset_id` / `asset.derivative_version`，但 model 没字段。要么改 model，要么新建 `asset_derivative_meta` 联结表。
- **决策**：在 `Asset` 上直接加两个字段。`source_asset_id: Option<String>`、`derivative_version: i32 (default 0)`。
- **被排除**：单独建 `asset_derivative_meta` 表。
- **理由**：当前所有调用都是"每个原件最多一个 canonical markdown derivative"。1:1 关系建联结表是过度设计。`source_asset_id` 本身就是 SQL 层的外键足够表达家族关系。
- **后果**：迁移必须用 `ALTER TABLE ADD COLUMN ... DEFAULT NULL/0`；30+ 处 `Asset { ... }` 字面量构造全部要补两个字段（建议同时在结构体加 `#[serde(default)]`，但必填字段仍需逐处补）。

### ADR-002：derivative_version 归属

- **状态**：已接受
- **上下文**：scheduler 现状是 source 和 derivative 都写同一个 `derivative_version`。这让"原件已被转换到第 v3 版"和"衍生件本身是第 v3 版"两件事共享一列。
- **决策**：保留双写，但语义统一为"该原件的转换轮次计数"。derivative 上的 `derivative_version` 必须**永远等于其 source 当前的 `derivative_version`**。
- **被排除**：只在 derivative 上写、source 不动。
- **理由**：搜索/UI 经常按 source 过滤，要快速取"这个原件被转换过多少次"必须不 join；双写代价小、读取无 join。
- **后果**：placeholder 路径若推进版本号会污染此语义（见 ADR-006）。

### ADR-003：Fallback 触发层

- **状态**：已接受
- **上下文**：当前 `get_extractor_for` 命中 markitdown 就直接返回；调用方拿到 `Err` 就失败。
- **决策**：fallback 决策**搬到 scheduler 主循环**。`get_extractor_for` 仍返回首选 extractor；scheduler 调用失败/空输出时**显式**调用 `get_fallback_extractor_for(mime_type)` 重试一次，再失败才写 placeholder。
- **被排除**：在 `get_extractor_for` 内返回 `Vec<Box<dyn Extractor>>` 让调用方迭代。
- **理由**：fallback 需要写 `conversion_meta` 含 `fallback_used=true`，这个元数据归属属于 scheduler 业务编排层而非选择器。把链路决策塞进选择器会让单测难写。
- **后果**：scheduler 主循环新增 20-40 行 fallback 编排逻辑；选择器 API 不变。

### ADR-004：conversion_meta 与 extracted_content 的边界

- **状态**：已接受
- **上下文**：现有 `extracted_content` 已存 `extractor_type` / `quality_level` / `structured_md` / `content_hash`。新增 `conversion_meta` 看起来字段重叠。
- **决策**：**并列存在**。`extracted_content` 是"提取产物"（markdown 文本 + 当前最优的质量与 extractor），`conversion_meta` 是"每次转换尝试的事实日志"（含失败尝试、fallback 记录、耗时、错误类别）。
- **被排除**：扩展 `extracted_content` 列。
- **理由**：单一 row 无法表达"先 markitdown 失败、再 pdf_text 成功"这种历史；做成 append-only 日志后，失败率/格式可观测面才能落地。
- **后果**：UI 查询要 `JOIN`（或新增 `get_conversion_meta` 命令）；append-only 即唯一约束改为 `(source_asset_id, converted_at)` 而非宪章原稿 `(source_asset_id, converter_name)`。

### ADR-005：保留 Extractor trait，不新建 Converter trait

- **状态**：已接受
- **上下文**：宪章 Step 3 要求新建 `Converter` trait 与 `ConversionResult`。但现有 `Extractor` trait + `ExtractionResult` 已经承担了 markitdown / pdf_text / docx / pptx 的所有调用。
- **决策**：**保留** `Extractor` trait。新增的 `ConversionResult` 概念以**结构体 + scheduler 编排**形式存在，不引入新 trait。
- **被排除**：双 trait 并存或一刀切换 trait。
- **理由**：新建 trait 会引发"现有 5 个 extractor 都要双实现"的连锁改动；本次目标是补救而非重构。`ExtractionResult` 多两个可选字段（`fallback_used` / `conversion_ms`）即可承载新语义。
- **后果**：`extraction/conversion.rs` 角色降级为"一个 `ConversionAttempt` 结构 + `file_sha256` 工具 + `classify_error` 工具"，**不含 trait**。

### ADR-006：placeholder 不推进版本号

- **状态**：已接受
- **上下文**：scheduler 当前 `materialize_placeholder` 共享 `write_derivative_md`，会推进 `derivative_version`。如果"先失败写 placeholder vN+1 → 再次重试成功写 vN+2"，归档目录会留下 v1...vN+1 的 placeholder 历史，污染版本语义。
- **决策**：placeholder **不推进**版本号（写入时 `next_version = source_asset.derivative_version`，而非 +1），且不归档前一版本。
- **被排除**：placeholder 也推进版本号。
- **理由**：placeholder 的语义是"占位"，不是"一次有效转换轮次"。
- **后果**：`write_derivative_md` 需要 `is_placeholder: bool` 参数；或拆出 `write_placeholder_md` 子函数。本方案选择拆函数（更清晰）。

---

## 四、系统架构

```
┌──────────────────────────────────────────────────────────────────┐
│ 前端 (src/)                                                       │
│  Inspector → InspectorExtraction.tsx                              │
│    - 显示 extractor_type / quality / fallback / conversion_ms     │
│    - "重试" 按钮 → tauri-commands.retriggerExtraction(assetId)    │
└────────────────────────┬─────────────────────────────────────────┘
                         │ invoke
┌────────────────────────▼─────────────────────────────────────────┐
│ Tauri 命令层 (src-tauri/src/commands/)                            │
│  - conversion::check_markitdown_status        [已有]              │
│  - conversion::convert_asset_to_markdown      [已有]              │
│  - conversion::get_conversion_meta(asset_id)  [新]                │
│  - extraction::retrigger_extraction(asset_id) [新]                │
└────────────────────────┬─────────────────────────────────────────┘
                         │
┌────────────────────────▼─────────────────────────────────────────┐
│ 调度层 (src-tauri/src/extraction/scheduler.rs)                    │
│  start() 主循环                                                   │
│   ├─ select primary: get_extractor_for(mime)                     │
│   ├─ run primary                                                  │
│   ├─ if Err / empty:                                              │
│   │    └─ select fallback: get_fallback_extractor_for(mime)       │
│   │       └─ run fallback (fallback_used=true)                    │
│   ├─ if all fail → materialize_placeholder()                      │
│   └─ in every branch → upsert conversion_meta                     │
│                                                                   │
│  write_derivative_md()          [已有，需补 placeholder 分支]     │
│  materialize_placeholder()      [已有，需改不推进版本号]          │
└─────────┬───────────────────────┬─────────────────────────────────┘
          │                       │
┌─────────▼─────────────┐   ┌─────▼──────────────────────────────┐
│ 转换器层               │   │ DB 层 (src-tauri/src/db/)          │
│ extractors/            │   │  asset.rs (新增 3 fn)              │
│  markitdown.rs         │   │  tag.rs   (新增 2 fn)              │
│   - 缓存 version       │   │  conversion_meta.rs (新建)         │
│   - classify_error     │   │  migration.rs (加列 + 建表)        │
│  pdf_text.rs (fallback)│   └────────────────────────────────────┘
│  docx.rs   (fallback)  │
│  pptx.rs   (fallback)  │
└────────────────────────┘
```

---

## 五、数据模型

### 5.1 `assets` 表新增列（Migration）

```sql
ALTER TABLE assets ADD COLUMN source_asset_id TEXT DEFAULT NULL;
ALTER TABLE assets ADD COLUMN derivative_version INTEGER NOT NULL DEFAULT 0;
CREATE INDEX IF NOT EXISTS idx_assets_source_asset_id ON assets(source_asset_id);
```

> 守卫策略：迁移函数用 `PRAGMA table_info(assets)` 检查列是否存在，避免重跑时报 `duplicate column`。

### 5.2 `conversion_meta` 表（新建）

```sql
CREATE TABLE IF NOT EXISTS conversion_meta (
  id                 TEXT PRIMARY KEY,
  source_asset_id    TEXT NOT NULL,
  derived_asset_id   TEXT,
  converter_name     TEXT NOT NULL,
  converter_version  TEXT NOT NULL DEFAULT 'builtin',
  source_mime        TEXT NOT NULL,
  source_hash        TEXT NOT NULL,
  quality_level      INTEGER NOT NULL DEFAULT 0,
  fallback_used      INTEGER NOT NULL DEFAULT 0,
  error_class        TEXT,
  conversion_ms      INTEGER,
  converted_at       TEXT NOT NULL,
  FOREIGN KEY (source_asset_id) REFERENCES assets(id) ON DELETE CASCADE,
  FOREIGN KEY (derived_asset_id) REFERENCES assets(id) ON DELETE SET NULL
);
CREATE INDEX IF NOT EXISTS idx_cm_source ON conversion_meta(source_asset_id);
CREATE INDEX IF NOT EXISTS idx_cm_derived ON conversion_meta(derived_asset_id);
CREATE INDEX IF NOT EXISTS idx_cm_converted_at ON conversion_meta(converted_at);
```

> 与宪章原稿差异：唯一约束**取消**（改为 append-only 日志，多条记录按 `converted_at` 排序）。

### 5.3 `Asset` Rust 模型

```rust
pub struct Asset {
    pub id: String,
    pub project_id: String,
    pub asset_type: String,
    pub name: String,
    #[serde(default)] pub original_name: String,
    pub file_path: String,
    pub file_size: i64,
    pub mime_type: String,
    pub captured_at: String,
    pub imported_at: String,
    pub source_type: String,
    pub source_data: Option<String>,
    pub is_starred: bool,
    // ↓ 新增
    #[serde(default)]
    pub source_asset_id: Option<String>,
    #[serde(default)]
    pub derivative_version: i32,
}
```

### 5.4 `error_class` 枚举（前后端约定）

```
file_not_found | permission_denied | unsupported_format |
markitdown_not_installed | python_unavailable | empty_output |
timeout | conversion_error
```

---

## 六、API 设计（Tauri 命令）

| 命令 | 状态 | 关键参数 | 返回 |
|------|------|---------|------|
| `check_markitdown_status` | 保留 | — | `MarkitdownStatus` |
| `convert_asset_to_markdown` | 保留 | `assetId: String` | `ConversionResult` |
| `get_conversion_meta` | **新** | `assetId: String` | `Vec<ConversionMetaRow>`（按 `converted_at desc`） |
| `retrigger_extraction` | **新** | `assetId: String` | `Result<(), String>` |

### 6.1 `get_conversion_meta` 响应（camelCase）

```typescript
interface ConversionMetaRow {
  id: string;
  sourceAssetId: string;
  derivedAssetId: string | null;
  converterName: string;
  converterVersion: string;
  sourceMime: string;
  sourceHash: string;
  qualityLevel: number;
  fallbackUsed: boolean;
  errorClass: string | null;
  conversionMs: number | null;
  convertedAt: string;
}
```

### 6.2 `retrigger_extraction` 行为

1. 校验 asset 存在
2. `extracted_content.status` ← `queued`
3. `pipeline_tasks` 中该 asset 的最近一条记录 `status` ← `queued`，`retry_count` ← 0
4. 唤醒 scheduler（通过既有事件或直接 `enqueue`）

---

## 七、目录结构

```
src-tauri/src/
├── models/
│   └── asset.rs                              [M] 加 2 字段
├── db/
│   ├── asset.rs                              [M] 加 3 fn + 既有 fn 适配新字段
│   ├── tag.rs                                [M] 加 2 fn
│   ├── conversion_meta.rs                    [N] CRUD + upsert
│   ├── migration.rs                          [M] V{N+1} 迁移
│   └── mod.rs                                [M] re-export conversion_meta
├── extraction/
│   ├── conversion.rs                         [N] ConversionAttempt 结构 + classify_error + file_sha256
│   ├── scheduler.rs                          [M] 主循环 fallback 编排 + placeholder 拆分
│   └── extractors/
│       └── markitdown.rs                     [M] 缓存版本 + classify_error 调用
├── commands/
│   ├── conversion.rs                         [M] +get_conversion_meta
│   ├── extraction.rs                         [M] +retrigger_extraction
│   └── dropzone.rs                           [M] AI 打标后 sync_tags_to_canonical_derivatives
└── lib.rs                                    [M] 注册新命令

src/
├── lib/
│   └── tauri-commands.ts                     [M] +getConversionMeta / +retriggerExtraction（如未有）
├── stores/
│   └── extractionStore.ts                    [M] 缓存 conversionMeta
└── components/layout/
    └── InspectorExtraction.tsx               [M] 展示 fallback / 耗时 / errorClass / version
```

`[N]` 新建 `[M]` 修改

---

## 八、安全考量

- 子进程调用：`Command::new(python).args(["-m", "markitdown", path_str])`，**禁止** `format!` 拼字符串再 `sh -c`。
- 错误外暴：前端只读 `error_class`，stderr 仅 `log::warn!`，不写入数据库。
- 文件路径：`workspace::ensure_project_workspace` 已沙箱化；新增的 `_versions/<source_id>/vN.md` 必须用 `Path::join`，禁止字符串拼接，防止 path traversal。
- 数据库迁移：所有 ALTER 必须可重入；存在性检查走 `PRAGMA`。

---

## 九、风险登记表

| ID | 风险 | 概率 | 影响 | 缓解措施 | 责任 task |
|----|------|------|------|----------|----------|
| R1 | Asset 加字段后大量 `Asset { ... }` 构造失败 | 高 | 高 | T2 一次性扫全仓改完；CI 跑 `cargo check` 闸门 | task_002 |
| R2 | placeholder 推进版本号污染 derivative_version 语义 | 中 | 中 | ADR-006 / T8 拆 `write_placeholder_md` 子函数 | task_008 |
| R3 | placeholder 写入 extracted_content 占住 status=extracted 导致后续真成功被跳过 | 中 | 高 | T8 验收用例覆盖 "placeholder → 真成功" 链路；placeholder 不写 extracted_content（仅写 .md 文件） | task_008 |
| R4 | DMG 打包后 PATH 中无 python3 | 中 | 高 | T7 把 `detect_embedded_markitdown_python` 提到候选首位 | task_007 |
| R5 | 迁移在已有库上重跑遇到 duplicate column | 低 | 中 | `PRAGMA table_info` 守卫 | task_002 |
| R6 | 标签传播在 dropzone / scheduler / inspector 重复实现 | 中 | 中 | code review checklist；只允许 `db::tag::propagate_*` 单一入口 | task_004 |
| R7 | 知识抽取消费数据源不明，可能与 canonical .md 解耦 | 中 | 高 | T12 单独审计，独立 PR | task_012 |
| R8 | conversion_meta append-only 长期积累膨胀 | 低 | 低 | 暂不做清理；后续单独迭代加 retention 策略 | — |

---

## 十、Task 清单（11 个 Dev/Review task）

详见各 `tasks/task_00N_*/input.md`。

```
task_002_dev_asset_model               — Asset 模型 + 迁移列（含 PRAGMA 守卫）
task_003_dev_db_asset_funcs            — db/asset.rs 新增 3 函数 + 既有 fn 适配新字段
task_004_dev_db_tag_funcs              — db/tag.rs 新增 2 函数 + dropzone 接 sync
task_005_dev_conversion_abstraction    — extraction/conversion.rs（ConversionAttempt + 工具函数）
task_006_dev_conversion_meta           — conversion_meta 表 + db/conversion_meta.rs CRUD
task_007_dev_markitdown_enrich         — markitdown.rs 版本缓存 + classify_error + embedded venv 优先
task_008_dev_scheduler_fallback        — scheduler 主循环 fallback 编排 + placeholder 拆分 + conversion_meta upsert
task_009_dev_get_conversion_meta_cmd   — get_conversion_meta Tauri 命令
task_010_dev_inspector_meta            — Inspector 展示 fallback / 耗时 / errorClass / version
task_011_dev_retrigger_extraction      — retrigger_extraction 命令 + 前端 wiring 校验
task_012_ux_review_knowledge           — 知识下游消费源审计（review-only，不改码）
```

---

## 十一、Task 依赖拓扑

```
task_002 ────► task_003 ────► task_004
                  │              │
                  ▼              ▼
              task_005 ────► task_006 ────► task_007
                                              │
                                              ▼
                                          task_008 ────► task_009 ────► task_010 ────► task_011
                                                                                            │
                                                                                            ▼
                                                                                       task_012

可并行：
  - {task_003, task_004} 在 task_002 完成后可并行（前者改 db/asset.rs，后者改 db/tag.rs + dropzone.rs，无文件冲突）
  - {task_005, task_006} 在 task_004 完成后可并行（前者新建 conversion.rs，后者新建 conversion_meta.rs + migration）
  - task_007 依赖 task_005（用 classify_error）
  - task_008 是关键路径节点（依赖 task_005/006/007 全部）
  - task_010 依赖 task_008/009（要有 meta 才能展示）
  - task_011 与 task_010 可并行
```

**关键路径**：`002 → 003 → 006 → 007 → 008 → 009 → 010 → 011 → 012`，全长 9 个 task。

---

## 十二、Task 粒度自检

| Task | 单一目标 | 可独立测试 | 规模 | 依赖清晰 | AC 可验证 |
|------|---------|-----------|------|---------|----------|
| 002 | ✅ 加字段 + 迁移 | ✅ `cargo check` | <300 行 | ✅ 无前置 | ✅ |
| 003 | ✅ 3 个 fn | ✅ rusqlite 单测 | <250 行 | ✅ 依赖 002 | ✅ |
| 004 | ✅ 2 个 fn + 1 处 wire | ✅ 集成测试 + 单测 | <200 行 | ✅ 依赖 002 | ✅ |
| 005 | ✅ 1 个文件 + 工具函数 | ✅ 单测 | <150 行 | ✅ 无业务依赖 | ✅ |
| 006 | ✅ 1 表 + CRUD | ✅ rusqlite 单测 | <250 行 | ✅ 依赖 002（迁移机制） | ✅ |
| 007 | ✅ 适配器增强 | ✅ probe + classify 单测 | <200 行 | ✅ 依赖 005 | ✅ |
| 008 | ⚠️ 范围最大 | ✅ 集成测试 | <500 行 | ✅ 依赖 005/006/007 | ✅ |
| 009 | ✅ 1 命令 | ✅ 手测 + 单测 | <100 行 | ✅ 依赖 006 | ✅ |
| 010 | ✅ 1 组件 | ✅ 手测 | <200 行 | ✅ 依赖 009 | ✅ |
| 011 | ✅ 1 命令 + 前端联调 | ✅ 手测 | <150 行 | ✅ 依赖 008 | ✅ |
| 012 | ✅ Review-only | ✅ 输出报告 | 0 行（仅文档） | ✅ 全部完成后 | ✅ |

> 唯一规模偏大的是 task_008（scheduler 主循环改造）。**已校验**该 task 不可再拆——三件事（primary 调用 / fallback 调用 / conversion_meta upsert）共享同一份 mime/asset 上下文，拆开后会导致状态在 task 间穿透，违背"task 可独立测试"。

---

## 十三、对 PM 的请求

1. 确认 ADR-001~006 是否接受；如有异议在进入 task_002 前提出。
2. 确认 `task_012` 在 task_011 完成后才启动（review-only 任务，可以阻塞或异步）。
3. 确认是否需要把 `sha2` 加入 `Cargo.toml`——现有 scheduler 已经 `use sha2`，意味着已经在 toml 里；task_005 验收会确认。

— END —
