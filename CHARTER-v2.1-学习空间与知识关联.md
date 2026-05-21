# 开发宪章 — NCdesktop v2.1 学习空间与知识关联

> 本文档是 PRD-v2.1 的开发实施指南。所有开发工作必须严格遵循本宪章。
> 日期: 2026-04-03

---

## 一、架构总则

### 1.1 不可违背的约束

这些规则继承自项目既有宪章，在 v2.1 开发中同样生效：

| 编号 | 规则 | 说明 |
|------|------|------|
| A1 | **仅命名导出** | 所有 React 组件使用 `export function`，禁止 `export default` |
| A2 | **CSS 变量** | 所有颜色、间距、圆角通过 `var(--xxx)` 引用，禁止硬编码 `#hex` / `rgb()` 值（临时 rgba 用于 selection highlight 除外） |
| A3 | **IPC 隔离** | 前端禁止直接调用 `invoke()`，必须经 `lib/tauri-commands.ts` 封装 |
| A4 | **Zustand 边界** | 每个 Store 职责单一，跨 Store 数据在组件层组合，Store 之间不互相 import |
| A5 | **snake_case ↔ camelCase** | Rust 侧用 snake_case，通过 `serde(rename_all = "camelCase")` 自动映射到前端 |
| A6 | **错误返回 String** | Rust command 统一返回 `Result<T, String>`，错误信息使用中文描述 |
| A7 | **UUID v4** | 所有实体 ID 使用 `uuid::Uuid::new_v4().to_string()` |
| A8 | **时间戳 RFC3339 UTC** | 所有时间字段使用 `Utc::now().to_rfc3339()` |

### 1.2 v2.1 新增架构决策

| 编号 | 决策 | 理由 |
|------|------|------|
| B1 | iCalendar 解析在 Rust 侧完成 | .ics 文件可能很大（数千事件），Rust 解析性能远优于 JS |
| B2 | 概念提取为异步后台任务 | 单次扫描可能涉及数十个文档 × LLM 调用，不能阻塞 UI |
| B3 | LLM 调用复用现有 `llm.rs` 基础设施 | 新增 command 而非新模块，保持 LLM 配置统一 |
| B4 | 知识关联数据存入现有 SQLite | 新增 5 张表，通过 migration V3 创建 |
| B5 | 预习空间是 ContentArea 的新视图模式 | 不是模态弹窗，与 AssetListView / TimelineView 平级 |

---

## 二、数据库变更（Migration V3）

### 2.1 新增表结构

在 `src-tauri/src/db/migrations.rs` 中追加 V3 migration：

```sql
-- V3: Course Calendar + Knowledge Association

-- 课程日历事件
CREATE TABLE IF NOT EXISTS course_events (
    id            TEXT PRIMARY KEY,
    library_id    TEXT NOT NULL,
    project_id    TEXT,                          -- 关联项目（可选）
    title         TEXT NOT NULL,
    course_code   TEXT,
    instructor    TEXT,
    location      TEXT,
    start_time    TEXT NOT NULL,                  -- RFC3339
    end_time      TEXT NOT NULL,
    recurrence_rule TEXT,
    day_of_week   TEXT,                          -- JSON: [1,3,5]
    description   TEXT,
    calendar_source TEXT NOT NULL DEFAULT 'ics_file',  -- "ics_file" | "ics_url"
    source_url    TEXT,
    source_uid    TEXT,                           -- iCalendar UID for dedup
    last_synced   TEXT,
    created_at    TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_course_events_library ON course_events(library_id);
CREATE INDEX IF NOT EXISTS idx_course_events_start ON course_events(start_time);
CREATE INDEX IF NOT EXISTS idx_course_events_code ON course_events(course_code);

-- AI 预习内容
CREATE TABLE IF NOT EXISTS course_previews (
    id              TEXT PRIMARY KEY,
    course_event_id TEXT NOT NULL REFERENCES course_events(id) ON DELETE CASCADE,
    content         TEXT NOT NULL,               -- Markdown
    user_notes      TEXT,                        -- 用户附加笔记
    model           TEXT,                        -- 使用的 LLM 模型名
    prompt_hash     TEXT,                        -- prompt 指纹，用于判断是否需要重新生成
    generated_at    TEXT NOT NULL,
    created_at      TEXT NOT NULL,
    UNIQUE(course_event_id)                      -- 每个事件仅保留最新一份预习
);

-- 概念
CREATE TABLE IF NOT EXISTS concepts (
    id              TEXT PRIMARY KEY,
    library_id      TEXT NOT NULL,
    name            TEXT NOT NULL,
    aliases         TEXT,                        -- JSON: ["alias1", "alias2"]
    definition      TEXT,
    source_asset_ids TEXT,                       -- JSON: ["asset_id_1", ...]
    source_project_ids TEXT,                     -- JSON: ["project_id_1", ...]
    user_edited     INTEGER NOT NULL DEFAULT 0,  -- boolean
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_concepts_library ON concepts(library_id);
CREATE INDEX IF NOT EXISTS idx_concepts_name ON concepts(name);

-- 概念观点
CREATE TABLE IF NOT EXISTS concept_viewpoints (
    id              TEXT PRIMARY KEY,
    concept_id      TEXT NOT NULL REFERENCES concepts(id) ON DELETE CASCADE,
    perspective     TEXT NOT NULL,
    summary         TEXT NOT NULL,
    source_context  TEXT,
    source_asset_id TEXT,
    generated_at    TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_viewpoints_concept ON concept_viewpoints(concept_id);

-- 概念案例
CREATE TABLE IF NOT EXISTS concept_cases (
    id              TEXT PRIMARY KEY,
    concept_id      TEXT NOT NULL REFERENCES concepts(id) ON DELETE CASCADE,
    title           TEXT NOT NULL,
    excerpt         TEXT NOT NULL,
    source_asset_id TEXT,
    source_location TEXT,
    relevance_note  TEXT
);

CREATE INDEX IF NOT EXISTS idx_cases_concept ON concept_cases(concept_id);

-- 概念拓展
CREATE TABLE IF NOT EXISTS concept_extensions (
    id              TEXT PRIMARY KEY,
    concept_id      TEXT NOT NULL REFERENCES concepts(id) ON DELETE CASCADE,
    direction       TEXT NOT NULL,               -- "upstream" | "downstream"
    name            TEXT NOT NULL,
    description     TEXT,
    relationship    TEXT
);

CREATE INDEX IF NOT EXISTS idx_extensions_concept ON concept_extensions(concept_id);
```

### 2.2 Migration 实施规范

- 在 `run_migrations()` 中检查 `PRAGMA user_version` 是否 < 3
- 所有 DDL 使用 `IF NOT EXISTS` 保证幂等性
- Migration 结束后执行 `PRAGMA user_version = 3`

---

## 三、Rust 后端实现

### 3.1 新增模块清单

```
src-tauri/src/
├── commands/
│   ├── calendar.rs        ← 新增：日历导入/查询/删除
│   ├── course_preview.rs  ← 新增：预习生成/查询
│   └── knowledge.rs       ← 新增：概念/观点/案例/拓展 CRUD + 提取任务
├── db/
│   ├── calendar.rs        ← 新增：course_events CRUD
│   ├── course_preview.rs  ← 新增：course_previews CRUD
│   └── knowledge.rs       ← 新增：concepts + viewpoints + cases + extensions CRUD
├── ics_parser.rs          ← 新增：iCalendar 解析器
└── lib.rs                 ← 注册新 commands
```

### 3.2 commands/calendar.rs

```rust
// 必须实现的 Tauri commands：

#[tauri::command]
pub fn import_ics_file(db: State<Database>, library_id: String, file_path: String)
    -> Result<ImportIcsResult, String>
// 解析 .ics 文件，返回解析出的事件列表供用户预览

#[tauri::command]
pub fn import_ics_url(db: State<Database>, library_id: String, url: String)
    -> Result<ImportIcsResult, String>
// 从 URL 拉取 .ics 内容并解析

#[tauri::command]
pub fn confirm_import_events(db: State<Database>, event_ids: Vec<String>)
    -> Result<usize, String>
// 用户确认后，将选中的事件写入数据库

#[tauri::command]
pub fn get_course_events(db: State<Database>, library_id: String, start_after: Option<String>, end_before: Option<String>)
    -> Result<Vec<CourseEvent>, String>
// 按时间范围查询课程事件

#[tauri::command]
pub fn delete_calendar_source(db: State<Database>, library_id: String, calendar_source: String, source_url: Option<String>)
    -> Result<usize, String>
// 删除某个日历来源的所有事件

#[tauri::command]
pub fn refresh_ics_subscription(db: State<Database>, library_id: String, source_url: String)
    -> Result<ImportIcsResult, String>
// 刷新订阅日历
```

**ImportIcsResult 结构:**
```rust
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportIcsResult {
    pub events: Vec<CourseEvent>,
    pub total_parsed: usize,
    pub duplicates_skipped: usize,
}
```

### 3.3 ics_parser.rs

iCalendar 解析器要求：

- 使用 `ical` crate（轻量级 Rust iCalendar 解析库）
- 提取 VEVENT 的 SUMMARY, DTSTART, DTEND, RRULE, LOCATION, DESCRIPTION, UID
- 支持 RRULE 展开：将重复事件展开为本学期（当前日期起 4 个月内）的独立实例
- 从 SUMMARY 中用正则提取 course_code：匹配 `[A-Z]{2,5}\s*\d{3,4}` 模式
- 从 DESCRIPTION 中提取 instructor：匹配常见模式 "Instructor: XXX" / "Prof. XXX"
- 去重：基于 UID + DTSTART 组合

### 3.4 commands/course_preview.rs

```rust
#[tauri::command]
pub async fn generate_course_preview(
    db: State<'_, Database>,
    course_event_id: String,
    force_regenerate: bool,
) -> Result<CoursePreview, String>
// 1. 获取课程事件信息
// 2. 查找该 courseCode 关联的项目中的历史笔记/素材
// 3. 组装 prompt (System + User)
// 4. 调用 LLM（复用 llm.rs 的 HTTP 客户端）
// 5. 存储结果到 course_previews 表
// 6. 返回给前端渲染

#[tauri::command]
pub fn get_course_preview(db: State<Database>, course_event_id: String)
    -> Result<Option<CoursePreview>, String>
// 查询已生成的预习内容

#[tauri::command]
pub fn save_preview_notes(db: State<Database>, course_event_id: String, notes: String)
    -> Result<(), String>
// 保存用户的预习笔记
```

### 3.5 commands/knowledge.rs

```rust
// --- 概念 CRUD ---

#[tauri::command]
pub fn get_concepts(db: State<Database>, library_id: String)
    -> Result<Vec<ConceptWithStats>, String>
// 返回概念列表，附带引用项目数、观点数等统计

#[tauri::command]
pub fn get_concept_detail(db: State<Database>, concept_id: String)
    -> Result<ConceptDetail, String>
// 返回概念完整详情：含 viewpoints, cases, extensions

#[tauri::command]
pub fn update_concept(db: State<Database>, concept_id: String, name: Option<String>, definition: Option<String>)
    -> Result<(), String>
// 用户编辑概念名或定义

#[tauri::command]
pub fn delete_concept(db: State<Database>, concept_id: String)
    -> Result<(), String>

// --- 概念提取（异步） ---

#[tauri::command]
pub async fn extract_concepts_for_library(
    db: State<'_, Database>,
    library_id: String,
    force: bool,
) -> Result<ExtractionProgress, String>
// 扫描知识库所有素材，对每个素材调用 LLM 提取概念
// force=true 时重新处理所有素材；false 时仅处理新素材
// 通过 Tauri event 发送进度: "notecapt/concept-extraction-progress"

#[tauri::command]
pub async fn synthesize_viewpoints(
    db: State<'_, Database>,
    concept_id: String,
) -> Result<Vec<ConceptViewpoint>, String>
// 对指定概念，收集所有来源素材的相关段落，调用 LLM 生成观点

#[tauri::command]
pub async fn generate_extensions(
    db: State<'_, Database>,
    concept_id: String,
) -> Result<Vec<ConceptExtension>, String>
// 对指定概念，调用 LLM 生成上下游知识拓展
```

**关键数据结构:**

```rust
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConceptWithStats {
    pub id: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub definition: Option<String>,
    pub source_project_count: usize,
    pub viewpoint_count: usize,
    pub case_count: usize,
    pub user_edited: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConceptDetail {
    pub concept: Concept,
    pub viewpoints: Vec<ConceptViewpoint>,
    pub cases: Vec<ConceptCase>,
    pub extensions: Vec<ConceptExtension>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractionProgress {
    pub total_assets: usize,
    pub processed: usize,
    pub concepts_found: usize,
    pub status: String,  // "running" | "completed" | "error"
}
```

### 3.6 lib.rs 注册

在 `generate_handler![]` 中注册所有新 commands：

```rust
// Calendar
commands::calendar::import_ics_file,
commands::calendar::import_ics_url,
commands::calendar::confirm_import_events,
commands::calendar::get_course_events,
commands::calendar::delete_calendar_source,
commands::calendar::refresh_ics_subscription,

// Course Preview
commands::course_preview::generate_course_preview,
commands::course_preview::get_course_preview,
commands::course_preview::save_preview_notes,

// Knowledge Association
commands::knowledge::get_concepts,
commands::knowledge::get_concept_detail,
commands::knowledge::update_concept,
commands::knowledge::delete_concept,
commands::knowledge::extract_concepts_for_library,
commands::knowledge::synthesize_viewpoints,
commands::knowledge::generate_extensions,
```

### 3.7 Cargo 依赖新增

```toml
# Cargo.toml [dependencies] 新增
ical = "0.11"         # iCalendar 解析
```

---

## 四、前端实现

### 4.1 新增 TypeScript 类型

文件: `src/types/calendar.ts`

```typescript
export interface CourseEvent {
  id: string;
  libraryId: string;
  projectId: string | null;
  title: string;
  courseCode: string | null;
  instructor: string | null;
  location: string | null;
  startTime: string;
  endTime: string;
  recurrenceRule: string | null;
  dayOfWeek: number[];
  description: string | null;
  calendarSource: "ics_file" | "ics_url";
  sourceUrl: string | null;
  lastSynced: string | null;
  createdAt: string;
}

export interface CoursePreview {
  id: string;
  courseEventId: string;
  content: string;
  userNotes: string | null;
  model: string | null;
  generatedAt: string;
  createdAt: string;
}

export interface ImportIcsResult {
  events: CourseEvent[];
  totalParsed: number;
  duplicatesSkipped: number;
}
```

文件: `src/types/knowledge.ts`

```typescript
export interface Concept {
  id: string;
  libraryId: string;
  name: string;
  aliases: string[];
  definition: string | null;
  sourceAssetIds: string[];
  sourceProjectIds: string[];
  userEdited: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface ConceptWithStats {
  id: string;
  name: string;
  aliases: string[];
  definition: string | null;
  sourceProjectCount: number;
  viewpointCount: number;
  caseCount: number;
  userEdited: boolean;
}

export interface ConceptViewpoint {
  id: string;
  conceptId: string;
  perspective: string;
  summary: string;
  sourceContext: string | null;
  sourceAssetId: string | null;
  generatedAt: string;
}

export interface ConceptCase {
  id: string;
  conceptId: string;
  title: string;
  excerpt: string;
  sourceAssetId: string | null;
  sourceLocation: string | null;
  relevanceNote: string | null;
}

export interface ConceptExtension {
  id: string;
  conceptId: string;
  direction: "upstream" | "downstream";
  name: string;
  description: string | null;
  relationship: string | null;
}

export interface ConceptDetail {
  concept: Concept;
  viewpoints: ConceptViewpoint[];
  cases: ConceptCase[];
  extensions: ConceptExtension[];
}

export interface ExtractionProgress {
  totalAssets: number;
  processed: number;
  conceptsFound: number;
  status: "running" | "completed" | "error";
}
```

### 4.2 IPC 命令封装

文件: `src/lib/tauri-commands.ts`（在文件末尾追加）

```typescript
// ─── Calendar ────────────────────────────────────────
export async function importIcsFile(libraryId: string, filePath: string): Promise<ImportIcsResult> {
  return invoke<ImportIcsResult>("import_ics_file", { libraryId, filePath });
}
export async function importIcsUrl(libraryId: string, url: string): Promise<ImportIcsResult> {
  return invoke<ImportIcsResult>("import_ics_url", { libraryId, url });
}
export async function confirmImportEvents(eventIds: string[]): Promise<number> {
  return invoke<number>("confirm_import_events", { eventIds });
}
export async function getCourseEvents(libraryId: string, startAfter?: string, endBefore?: string): Promise<CourseEvent[]> {
  return invoke<CourseEvent[]>("get_course_events", { libraryId, startAfter, endBefore });
}
export async function deleteCalendarSource(libraryId: string, calendarSource: string, sourceUrl?: string): Promise<number> {
  return invoke<number>("delete_calendar_source", { libraryId, calendarSource, sourceUrl });
}
export async function refreshIcsSubscription(libraryId: string, sourceUrl: string): Promise<ImportIcsResult> {
  return invoke<ImportIcsResult>("refresh_ics_subscription", { libraryId, sourceUrl });
}

// ─── Course Preview ──────────────────────────────────
export async function generateCoursePreview(courseEventId: string, forceRegenerate: boolean): Promise<CoursePreview> {
  return invoke<CoursePreview>("generate_course_preview", { courseEventId, forceRegenerate });
}
export async function getCoursePreview(courseEventId: string): Promise<CoursePreview | null> {
  return invoke<CoursePreview | null>("get_course_preview", { courseEventId });
}
export async function savePreviewNotes(courseEventId: string, notes: string): Promise<void> {
  return invoke<void>("save_preview_notes", { courseEventId, notes });
}

// ─── Knowledge Association ───────────────────────────
export async function getConcepts(libraryId: string): Promise<ConceptWithStats[]> {
  return invoke<ConceptWithStats[]>("get_concepts", { libraryId });
}
export async function getConceptDetail(conceptId: string): Promise<ConceptDetail> {
  return invoke<ConceptDetail>("get_concept_detail", { conceptId });
}
export async function updateConcept(conceptId: string, name?: string, definition?: string): Promise<void> {
  return invoke<void>("update_concept", { conceptId, name, definition });
}
export async function deleteConcept(conceptId: string): Promise<void> {
  return invoke<void>("delete_concept", { conceptId });
}
export async function extractConceptsForLibrary(libraryId: string, force: boolean): Promise<ExtractionProgress> {
  return invoke<ExtractionProgress>("extract_concepts_for_library", { libraryId, force });
}
export async function synthesizeViewpoints(conceptId: string): Promise<ConceptViewpoint[]> {
  return invoke<ConceptViewpoint[]>("synthesize_viewpoints", { conceptId });
}
export async function generateExtensions(conceptId: string): Promise<ConceptExtension[]> {
  return invoke<ConceptExtension[]>("generate_extensions", { conceptId });
}
```

### 4.3 新增 Store

#### calendarStore.ts

```typescript
// 职责: 课程日历事件的前端状态管理
// 状态:
//   events: CourseEvent[]
//   selectedEventId: string | null
//   isLoading: boolean
//   error: string | null
//
// 方法:
//   fetchEvents(libraryId, startAfter?, endBefore?)
//   selectEvent(id: string | null)
//   importFromFile(libraryId, filePath)
//   importFromUrl(libraryId, url)
//   confirmImport(eventIds)
//   deleteSource(libraryId, source, url?)
//   refreshSubscription(libraryId, url)
//
// 派生:
//   getEventsGroupedByDay() → Map<string, CourseEvent[]>
//   getTodayEvents() → CourseEvent[]
```

#### knowledgeStore.ts

```typescript
// 职责: 知识关联页面的前端状态管理
// 状态:
//   concepts: ConceptWithStats[]
//   selectedConceptId: string | null
//   conceptDetail: ConceptDetail | null
//   extractionProgress: ExtractionProgress | null
//   searchQuery: string
//   filterProjectId: string | null
//   isLoading: boolean
//   error: string | null
//
// 方法:
//   fetchConcepts(libraryId)
//   selectConcept(id: string | null) → 自动 loadDetail
//   loadDetail(conceptId)
//   updateConcept(id, name?, definition?)
//   deleteConcept(id)
//   startExtraction(libraryId, force)
//   setSearchQuery(q)
//   setFilterProject(projectId | null)
//
// 派生:
//   getFilteredConcepts() → 基于 searchQuery + filterProjectId 过滤
```

### 4.4 新增组件清单

```
src/components/features/
├── calendar/
│   ├── CourseSection.tsx          ← 侧边栏课程分区（日程列表）
│   ├── CourseEventItem.tsx        ← 单个课程事件条目
│   └── CalendarImportTab.tsx      ← 设置面板的「课程日历」选项卡
├── preview/
│   └── CoursePreviewSpace.tsx     ← 预习空间主视图（替换 ContentArea）
└── knowledge/
    ├── KnowledgeAssociationView.tsx  ← 知识关联主视图（替换 ContentArea）
    ├── ConceptList.tsx               ← 左侧概念列表
    ├── ConceptDetailPanel.tsx        ← 右侧概念详情
    ├── ViewpointCard.tsx             ← 观点卡片
    ├── CaseCard.tsx                  ← 案例卡片
    └── ExtensionPanel.tsx            ← 知识拓展面板
```

### 4.5 路由与视图切换

#### uiStore 变更

在 `uiStore.ts` 中扩展 `rightPanelMode`：

```typescript
// 现有值: "asset_list" | "timeline" | ...
// 新增值: "course_preview" | "knowledge_association"

// 新增状态:
activeCourseEventId: string | null
setActiveCourseEventId: (id: string | null) => void
```

#### ContentArea.tsx 变更

在 ContentArea 中根据 `rightPanelMode` 切换视图：

```tsx
// 现有逻辑之外新增：
if (rightPanelMode === "course_preview" && activeCourseEventId) {
  return <CoursePreviewSpace courseEventId={activeCourseEventId} />;
}
if (rightPanelMode === "knowledge_association") {
  return <KnowledgeAssociationView />;
}
```

#### Toolbar.tsx 变更

项目级工具栏新增「知识关联」按钮：

```tsx
<button
  type="button"
  className={`toolbar-btn ${rightPanelMode === "knowledge_association" ? "active" : ""}`}
  onClick={() => setRightPanelMode("knowledge_association")}
  title="知识关联"
>
  <BrainCircuit size={16} />  {/* 来自 lucide-react */}
  知识关联
</button>
```

#### Sidebar.tsx 变更

在 `<ProjectTree />` 上方插入 `<CourseSection />`：

```tsx
<CourseSection />
<div className="h-px my-[var(--space-2)]" style={{ background: "var(--border-primary)" }} />
<ProjectTree />
```

### 4.6 设置面板扩展

在 `SettingsPanel.tsx` 的选项卡列表中新增：

```typescript
// 在现有 tabs 数组中追加：
{ id: "calendar", label: "课程日历", icon: <Calendar size={16} /> }

// 对应内容区渲染 <CalendarImportTab />
```

---

## 五、交互规范

### 5.1 日历导入交互

| 步骤 | 用户动作 | 系统反馈 |
|------|---------|---------|
| 1 | 打开设置 → 课程日历 | 显示导入区域 |
| 2 | 拖入 .ics 文件 | 解析中 spinner → 显示事件预览列表 |
| 3 | 取消勾选不需要的课程 | 实时更新计数 |
| 4 | 点击「确认导入」 | 写入数据库 → 侧边栏出现课程列表 → Toast 提示「已导入 N 门课程」 |
| 5 | 导入完成后关闭设置 | 侧边栏课程分区可见 |

### 5.2 预习空间交互

| 步骤 | 用户动作 | 系统反馈 |
|------|---------|---------|
| 1 | 点击侧边栏课程事件 | ContentArea 切换为预习空间 |
| 2 | 首次进入 | 显示骨架屏 → LLM 生成预习内容 → 渲染 Markdown |
| 3 | 再次进入（已有缓存） | 直接显示已生成的内容 |
| 4 | 点击「重新生成」 | 重新调用 LLM，替换旧内容 |
| 5 | 在笔记区输入 | 自动保存（debounce 1s） |
| 6 | 点击「保存为素材」 | 将预习 Markdown 创建为 Asset 关联到对应项目 |
| 7 | 点击左上角「← Back」 | 返回之前的视图 |

### 5.3 知识关联交互

| 步骤 | 用户动作 | 系统反馈 |
|------|---------|---------|
| 1 | 点击工具栏「知识关联」 | ContentArea 切换为知识关联视图 |
| 2 | 首次进入（无概念数据） | 提示「需要扫描您的文档以提取概念」→ 用户点击「开始扫描」 |
| 3 | 扫描进行中 | 进度条 + 实时计数（已处理 X/Y 文档，发现 Z 个概念） |
| 4 | 扫描完成 | 左侧出现概念列表 |
| 5 | 点击某个概念 | 右侧加载概念详情（定义 + 观点 + 案例 + 拓展） |
| 6 | 观点/拓展首次加载 | 如果尚未生成，触发 LLM 合成，显示 spinner |
| 7 | 点击「编辑」(定义区) | 文本变可编辑 → 保存后标记 userEdited |
| 8 | 点击案例「查看原文」 | 打开 DocumentViewer 定位到对应素材 |
| 9 | 搜索框输入 | 实时过滤概念列表 |
| 10 | 点击「重新扫描」 | 重新跑提取流程（保留 userEdited 的概念不被覆盖） |

---

## 六、事件通信

### 6.1 Tauri Events（后端 → 前端）

| 事件名 | 载荷 | 触发时机 |
|--------|------|---------|
| `notecapt/concept-extraction-progress` | `ExtractionProgress` | 概念提取过程中的进度更新 |
| `notecapt/concept-extraction-done` | `{ libraryId, conceptCount }` | 概念提取完成 |
| `notecapt/calendar-sync-done` | `{ libraryId, eventCount }` | 日历订阅刷新完成 |

### 6.2 前端监听

在 `App.tsx` 中注册新事件监听：

```typescript
// 概念提取完成 → 刷新知识关联列表
listen("notecapt/concept-extraction-done", () => {
  knowledgeStore.getState().fetchConcepts(activeLibraryId);
});

// 日历刷新完成 → 刷新课程列表
listen("notecapt/calendar-sync-done", () => {
  calendarStore.getState().fetchEvents(activeLibraryId);
});
```

---

## 七、样式规范

### 7.1 新增 CSS 变量（如有需要）

本次迭代不新增 CSS 变量。所有新组件使用已有的设计系统变量：

- 表面: `--surface-primary`, `--surface-secondary`, `--surface-tertiary`, `--surface-elevated`
- 文字: `--text-primary`, `--text-secondary`, `--text-tertiary`
- 边框: `--border-primary`, `--border-hover`, `--border-active`
- 间距: `--space-1` ~ `--space-8`
- 圆角: `--radius-sm`, `--radius-md`, `--radius-lg`
- 阴影: `--shadow-sm`, `--shadow-lg`, `--shadow-float`
- 品牌色: `--brand-navy`（用于强调元素）

### 7.2 组件级样式约定

- 概念列表项激活态: `background: var(--sidebar-active-bg)`
- 观点卡片: `border-left: 3px solid var(--brand-navy)` + `background: var(--surface-secondary)`
- 案例卡片: `border-left: 3px solid var(--text-tertiary)` + `font-style: italic`（引文效果）
- 拓展面板: `background: var(--surface-tertiary)` + 虚线边框
- 预习空间 Markdown 渲染: 使用项目已有的 prose 样式或最小化自定义

---

## 八、测试要求

### 8.1 Rust 单元测试

| 模块 | 必测场景 |
|------|---------|
| `ics_parser.rs` | 标准 .ics 解析、RRULE 展开、course_code 正则、去重逻辑 |
| `db/calendar.rs` | CRUD 操作、时间范围查询 |
| `db/knowledge.rs` | 概念 CRUD、级联删除（概念删除时观点/案例/拓展跟随删除） |

### 8.2 前端组件测试

| 组件 | 必测场景 |
|------|---------|
| `CourseSection` | 按日分组渲染、空状态展示 |
| `ConceptList` | 搜索过滤、概念选中状态 |
| `ConceptDetailPanel` | 编辑模式切换、保存回调 |

---

## 九、实施顺序

严格按以下顺序开发，每步完成后可验证：

```
Phase 1: 日历基础设施
  ├── Step 1: Migration V3 — 新建所有表
  ├── Step 2: ics_parser.rs — iCalendar 解析器
  ├── Step 3: db/calendar.rs — 课程事件 CRUD
  ├── Step 4: commands/calendar.rs — Tauri commands
  ├── Step 5: tauri-commands.ts — IPC 封装
  ├── Step 6: calendarStore.ts — 前端 Store
  ├── Step 7: CalendarImportTab.tsx — 设置面板选项卡
  └── Step 8: CourseSection.tsx — 侧边栏课程列表

Phase 2: AI 预习
  ├── Step 9: db/course_preview.rs — 预习内容 CRUD
  ├── Step 10: commands/course_preview.rs — 预习生成 command（含 prompt）
  ├── Step 11: tauri-commands.ts — 预习 IPC
  └── Step 12: CoursePreviewSpace.tsx — 预习空间 UI

Phase 3: 知识关联
  ├── Step 13: db/knowledge.rs — 概念相关表 CRUD
  ├── Step 14: commands/knowledge.rs — 概念提取/观点合成/拓展生成
  ├── Step 15: tauri-commands.ts — 知识关联 IPC
  ├── Step 16: knowledgeStore.ts — 前端 Store
  ├── Step 17: KnowledgeAssociationView.tsx — 主视图
  ├── Step 18: ConceptList.tsx + ConceptDetailPanel.tsx
  ├── Step 19: ViewpointCard.tsx + CaseCard.tsx + ExtensionPanel.tsx
  └── Step 20: Toolbar.tsx 集成「知识关联」按钮
```

---

## 十、风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|---------|
| iCalendar 格式碎片化（不同学校系统导出格式差异） | 解析失败 | 收集 5+ 所大学的 .ics 样本做兼容测试；对无法解析的字段 graceful fallback |
| LLM 预习内容质量不稳定 | 用户信任度下降 | 提供「重新生成」按钮；显示 "AI Generated" 标签设预期；prompt 模板持续迭代 |
| 概念提取大量文档时 LLM 成本高 | API 费用 | 增量处理（已处理的不重复）；提取 prompt 优化 token 效率；显示预估 token 数 |
| 概念去重/合并困难 | 重复概念泛滥 | 基于 name 精确匹配 + alias 模糊匹配自动合并；提示用户手动确认 |
