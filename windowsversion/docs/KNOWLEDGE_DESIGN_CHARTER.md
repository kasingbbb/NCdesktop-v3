# NCdesktop 知识相关功能原始设计宪章

> **文档目的**：本文档作为独立上下文文件，供 AI 助手或团队成员在参与知识功能迭代时快速建立完整认知。包含：设计意图、数据模型、功能边界、已知问题、设计约束。
>
> **版本**：v1.0（基于当前代码库提炼，日期 2026-04-21）
> **适用范围**：所有涉及知识提取、知识管理、知识理解的迭代讨论

---

## 一、产品定位与核心理念

### 1.1 NCdesktop 是什么

NCdesktop 是一款面向**学生**的桌面学习助手（Tauri + React + SQLite），核心价值是帮助用户将自己收集的**学习素材**（课件、笔记、PDF、音频）转化为**可理解、可关联、可内化**的知识体系。

### 1.2 知识功能的核心信念

> **"你的理解只来自你的文档。"**

所有知识相关功能严格遵守一条原则：AI 的输出**只引用用户自己导入的素材**，不引入外部知识库或预训练知识。这一承诺贯穿所有 UI 文案（"基于你的文档"横幅）、Prompt 设计和来源标注。

### 1.3 知识三层模型

知识功能按照从原始材料到内化理解的路径，分为三层：

```
原始素材（Assets）
    ↓ 知识提取
概念体系（Concepts）
    ↓ 知识关联
观点 / 案例 / 拓展（Viewpoints / Cases / Extensions）
    ↓ 知识理解
摘要 + 理解框架 + 用户笔记 + 镜子反馈（Understanding）
```

---

## 二、功能模块详述

### 2.1 知识提取（Knowledge Extraction）

#### 设计意图

将用户导入的原始素材（PDF、图片、音频）自动解析为结构化文本，作为后续所有知识功能的数据基础。提取在后台异步运行，不阻塞用户操作。

#### 数据结构

```typescript
interface ExtractedContent {
  id: string;
  assetId: string;
  status: 'pending' | 'extracting' | 'extracted' | 'failed' | 'unsupported';
  rawText: string | null;
  structuredMd: string | null;    // 结构化 Markdown（段落、标题保留）
  qualityLevel: number;           // 提取质量评分 0-1
  extractorType: string;          // 提取器类型（pdf/ocr/whisper等）
  segmentsJson: string | null;    // 分段 JSON（供后续按段检索）
  retryCount: number;
  errorMessage: string | null;
}

interface PipelineProgress {
  queued: number;
  running: number;
  completed: number;
  failed: number;
  cancelled: number;
}
```

#### 交互流程

```
用户触发「重新扫描」
  → Tauri: extract_project_assets(projectId)
  → 后台异步批量提取
  → emit "extraction:progress" (单项进度)
  → emit "extraction:completed" / "extraction:failed"
  → 前端 extractionStore 更新进度状态
```

#### 已知问题

- **无优先级调度**：所有素材平等排队，用户无法优先提取某个文件。
- **unsupported 类型无降级**：不支持的格式直接标记失败，无提示说明原因。
- **qualityLevel 评分未公开使用**：字段已有但 UI 中未展示，下游知识提取是否依赖该评分不明确。

---

### 2.2 知识关联（Knowledge Association）

#### 设计意图

从提取后的文本中自动识别"概念"，并为每个概念组织多视角观点、应用案例、前后置知识拓展，形成可编辑的个人知识库。

#### 数据结构

```typescript
// 概念（列表级）
interface ConceptWithStats {
  id: string;
  name: string;
  aliases: string[];
  definition: string | null;
  sourceProjectCount: number;   // 涉及几个项目
  viewpointCount: number;
  caseCount: number;
  userEdited: boolean;          // 用户是否手动编辑过
}

// 概念（详情级）
interface Concept {
  id: string;
  libraryId: string;
  name: string;
  aliases: string[];
  definition: string | null;    // 用户可编辑
  sourceAssetIds: string[];     // 来源素材 IDs
  sourceProjectIds: string[];   // 来源项目 IDs
  userEdited: boolean;
}

// 观点（同一概念在不同课程/视角下的解读）
interface ConceptViewpoint {
  id: string;
  conceptId: string;
  perspective: string;          // 视角标签（如"物理课"、"化学课"）
  summary: string;              // 该视角下的综合观点
  sourceContext: string | null; // 原文片段
  sourceAssetId: string | null;
}

// 案例（概念的应用实例）
interface ConceptCase {
  id: string;
  conceptId: string;
  title: string;
  excerpt: string;              // 原文摘录
  sourceAssetId: string | null;
  sourceLocation: string | null;  // 位置信息（如"第3页"）
  relevanceNote: string | null;
}

// 知识拓展（上下游关系）
interface ConceptExtension {
  id: string;
  conceptId: string;
  direction: "upstream" | "downstream";
  name: string;
  description: string | null;
  relationship: string | null;  // 关系描述
}
```

#### UI 结构

```
KnowledgeAssociationView（两栏布局）
├── 左栏：概念列表
│   ├── 搜索框（前端 fuzzy match，含 aliases）
│   ├── 项目筛选
│   └── ConceptItem（可点击选中）
└── 右栏：ConceptDetailPanel
    ├── 概念名 + 别名标签
    ├── 定义区（可编辑 textarea）
    ├── 相关观点区（ViewpointCard × N）
    ├── 案例区（CaseCard × N）
    ├── 知识拓展区（上游 + 下游）
    └── "深入理解" 按钮（进入理解页面）
```

#### 关键 Tauri 命令

| 命令 | 作用 |
|------|------|
| `get_concepts(libraryId)` | 获取全部概念（含统计） |
| `get_concept_detail(conceptId)` | 获取完整详情（观点+案例+拓展） |
| `update_concept(id, name?, def?)` | 编辑概念 |
| `delete_concept(conceptId)` | 删除概念 |
| `extract_concepts_for_library(libraryId, force)` | 触发 AI 概念提取 |
| `synthesize_viewpoints(conceptId)` | AI 合成多视角观点 |
| `generate_extensions(conceptId)` | AI 生成上下游拓展 |

#### 已知问题

1. **观点/案例生成时机不明确**：提取流程中是自动生成还是需用户手动触发，代码层面未统一文档化。
2. **项目筛选不精准**：前端只判断 `sourceProjectCount > 0`，无法精确过滤"仅属于某项目"。
3. **synthesize_viewpoints / generate_extensions 实现状态未确认**：前端命令接口已定义，Rust 侧是否完整实现需核查。
4. **概念去重策略不透明**：当同一概念在不同素材中出现，如何合并/去重，用户无感知。
5. **userEdited 标记后的行为**：被用户编辑的概念在重新扫描时是否会被覆盖，策略未文档化。

---

### 2.3 知识理解（Knowledge Understanding）

#### 设计意图

提供一个**主动学习辅助环境**，引导用户从"被动接收摘要"走向"自主内化理解"。通过四个步骤循序递进：

1. **摘要**（"你的文档怎么说"）：整合素材，给用户一个起点
2. **理解框架**（"核心机制 + 典型场景 + 常见误区 + 一句话精华"）：结构化帮助用户建立模型
3. **用户笔记**（"用你自己的话解释"）：逼迫用户主动输出
4. **镜子反馈**（"和 AI 核对"）：对照用户理解与文档内容，给出非评判性差异提示

> 关键设计哲学：AI 是**镜子**，不是评判者。反馈只说"你说到了哪些"和"文档里还有哪些角度"，不评分、不说"错了"。

#### 数据结构

```typescript
// 文档摘要
interface ConceptSummaryResult {
  id: string;
  conceptId: string;
  summary: string;                  // 整合自用户文档的摘要文本
  sourceAssetIds: string[];
  model: string;
  generatedAt: string;
}

// 理解框架
interface ConceptExplanationResult {
  id: string;
  conceptId: string;
  mechanism: ExplanationItem;             // 核心机制（1条）
  typicalScenarios: ExplanationItem[];    // 典型场景（多条）
  commonMisconceptions: ExplanationItem[] | null;  // 常见误区（可能无）
  essenceSentence: string;                // 一句话精华
  sourceAssetIds: string[];
  model: string;
  generatedAt: string;
}

// 解释条目（带来源引用）
interface ExplanationItem {
  text: string;
  source: string;   // 来源描述，如 "来自 Asset X 第3页"
}

// 用户笔记
interface UserNoteResult {
  id: string;
  conceptId: string;
  userExplanation: string;                // 用户自由文本
  mirrorFeedback: MirrorFeedbackResult | null;
  lastValidatedAt: string | null;
  createdAt: string;
  updatedAt: string;
}

// 镜子反馈（AI核对结果）
interface MirrorFeedbackResult {
  coveredCount: number;                   // 用户捕捉到的要点数
  coveredPoints: string[];                // 具体要点（积极呈现）
  additionalPerspectives: FeedbackPerspective[];  // 文档中用户未提到的角度
  differenceNote: string | null;          // 整体差异说明（温和措辞）
}

// 概念关系
interface ConceptRelationResult {
  id: string;
  conceptAId: string;
  conceptBId: string;
  relationType: "co_occurrence" | "upstream" | "downstream";
  coOccurrenceCount: number;
  sourceAssetIds: string[];
  otherConceptId: string;               // JOIN 后另一侧
  otherConceptName: string;
}

// Store 完整状态
interface KnowledgeUnderstandingState {
  conceptId: string | null;
  summary: ConceptSummaryResult | null;
  explanation: ConceptExplanationResult | null;
  userNote: UserNoteResult | null;
  mirrorFeedback: MirrorFeedbackResult | null;
  relations: ConceptRelationResult[];
  // 流式状态
  summaryStatus: "idle" | "streaming" | "done" | "error";
  explanationStatus: "idle" | "streaming" | "done" | "error";
  mirrorStatus: "idle" | "streaming" | "done" | "error";
  // 流式 buffer
  summaryStreamBuffer: string;
  explanationStreamBuffer: string;
  mirrorStreamBuffer: string;
}
```

#### UI 结构

```
KnowledgeUnderstandingPage（单页全屏）
├── 顶部：← 返回 + 概念名
├── TransparencyBanner（"基于你的文档"声明）
└── 滚动内容区
    ├── SummarySection
    │   ├── 摘要文本（流式 / 缓存）
    │   ├── 来源素材标签
    │   └── 重新生成按钮
    ├── ExplanationSection（手动触发生成）
    │   ├── 核心机制 ExplanationItemCard（text + source）
    │   ├── 典型场景 × N
    │   ├── 常见误区 × N（null 时隐藏整个区块）
    │   └── 一句话精华
    ├── UserNotesSection
    │   ├── Textarea（1s debounce 自动保存）
    │   ├── "给我一个出发点" → 填充 essenceSentence
    │   ├── "和 AI 核对一下" → 触发镜子反馈
    │   └── MirrorFeedbackDisplay
    │       ├── 捕捉到的要点列表（✓ 展示）
    │       ├── 补充视角列表（带来源）
    │       └── 差异说明（温和措辞）
    └── RelationNetworkSection
        ├── 同现关系（co_occurrence）
        ├── 上游前置（upstream）
        └── 下游应用（downstream，可点击导航）
```

#### 关键 Tauri 命令与流式事件

```typescript
// 查询缓存（纯 DB）
knowledge_get_understanding_data(conceptId) -> UnderstandingData

// 生成摘要（流式推送 "knowledge:summary:chunk"）
knowledge_generate_summary(conceptId, forceRegenerate: bool)

// 生成理解框架（流式推送 "knowledge:explanation:chunk"）
knowledge_generate_explanation(conceptId, forceRegenerate: bool)

// 保存用户笔记
knowledge_save_user_note(conceptId, userExplanation: string)

// 镜子反馈（流式推送 "knowledge:mirror:chunk"）
knowledge_validate_explanation(conceptId, userExplanation: string)

// 获取概念关系
knowledge_get_relations(conceptId) -> ConceptRelationResult[]

// 流式事件 payload
interface ChunkPayload {
  conceptId: string;
  chunk: string;      // 本次文本片段
  isFinal: boolean;   // true 表示完成，前端随后重新加载完整数据
}
```

#### 页面挂载流程

```
挂载
  → knowledge_get_understanding_data()
  → 有缓存 → 直接设置 status=done
  → 无 summary 缓存 → 自动触发 knowledge_generate_summary(force=false)
  → 注册3路流式事件监听
     - knowledge:summary:chunk → appendSummaryChunk()
     - knowledge:explanation:chunk → appendExplanationChunk()
     - knowledge:mirror:chunk → appendMirrorChunk()
  → isFinal=true → 重新加载 knowledge_get_understanding_data()

概念切换
  → resetForConcept(newConceptId)  // 清空所有缓存+buffer，全部 idle
  → 重新执行挂载流程
```

#### 已知问题

1. **镜子反馈的流式体验问题**：`mirrorStreamBuffer` 是纯文本拼接，而最终结果是 JSON 对象（`MirrorFeedbackResult`）。流式期间 UI 展示什么？目前设计语焉不详，可能只显示 loading 而非流式文字。
2. **explanationSection 手动触发的引导缺失**：摘要自动生成，而理解框架需要手动点击，但 UI 中没有明显的"下一步"引导，用户可能不知道要点击。
3. **"给我一个出发点"仅填充 essenceSentence**：essenceSentence 是一句话精华，可能过于简短，对不同学习风格的用户启发效果存疑。
4. **mirrorFeedback 不随 userExplanation 实时更新**：反馈只在用户主动点击"核对"时生成，但 userExplanation 在自动保存。两者之间存在"已保存但未核对"的中间态，前端是否有状态指示不明确。
5. **概念关系数据依赖**：concept_relations 表存在，但关系的**自动提取逻辑**（何时运行、如何识别上下游）在代码中未见清晰文档，RelationNetworkSection 的内容完整性不可保证。
6. **forceRegenerate 策略不完整**：缓存存在时用缓存，但没有版本控制——当用户新增素材后，旧摘要不会自动失效，需要用户手动点"重新生成"，且无任何提示。

---

## 三、数据库 Schema（知识相关表）

```sql
-- 概念主表
concepts (
  id TEXT PK,
  library_id TEXT,
  name TEXT,
  aliases TEXT,           -- JSON: string[]
  definition TEXT,        -- 用户可编辑
  source_asset_ids TEXT,  -- JSON: string[]
  source_project_ids TEXT,-- JSON: string[]
  user_edited INTEGER,    -- 0/1
  created_at TEXT,        -- RFC3339 UTC
  updated_at TEXT
)

-- 多视角观点
concept_viewpoints (
  id TEXT PK,
  concept_id TEXT,
  perspective TEXT,       -- 视角名（课程/来源）
  summary TEXT,
  source_context TEXT,    -- 原文片段
  source_asset_id TEXT,
  generated_at TEXT
)

-- 应用案例
concept_cases (
  id TEXT PK,
  concept_id TEXT,
  title TEXT,
  excerpt TEXT,
  source_asset_id TEXT,
  source_location TEXT,   -- "第N页" 等
  relevance_note TEXT
)

-- 知识拓展
concept_extensions (
  id TEXT PK,
  concept_id TEXT,
  direction TEXT,         -- upstream | downstream
  name TEXT,
  description TEXT,
  relationship TEXT       -- 关系描述
)

-- 文档摘要（知识理解）
concept_summaries (
  id TEXT PK,
  concept_id TEXT,
  summary TEXT,
  source_asset_ids TEXT,  -- JSON
  model TEXT,
  generated_at TEXT
)

-- 理解框架（知识理解）
concept_explanations (
  id TEXT PK,
  concept_id TEXT,
  mechanism TEXT,             -- JSON: ExplanationItem
  typical_scenarios TEXT,     -- JSON: ExplanationItem[]
  common_misconceptions TEXT, -- JSON: ExplanationItem[] | null
  essence_sentence TEXT,
  source_asset_ids TEXT,      -- JSON
  model TEXT,
  generated_at TEXT
)

-- 用户笔记（知识理解）
concept_user_notes (
  id TEXT PK,
  concept_id TEXT,
  user_explanation TEXT,
  mirror_feedback TEXT,       -- JSON: MirrorFeedbackResult | null
  last_validated_at TEXT,
  created_at TEXT,
  updated_at TEXT
)

-- 概念关系（知识理解）
concept_relations (
  id TEXT PK,
  concept_a_id TEXT,
  concept_b_id TEXT,
  relation_type TEXT,         -- co_occurrence | upstream | downstream
  source_asset_ids TEXT,      -- JSON
  co_occurrence_count INTEGER,
  created_at TEXT
)
```

---

## 四、Store 架构与边界

### 4.1 Store 职责划分

| Store | 管理内容 | 边界 |
|-------|----------|------|
| `extractionStore` | 素材提取状态、管道进度 | 不感知概念 |
| `knowledgeStore` | 概念列表、选中状态、提取进度、搜索/筛选 | 不感知理解流程 |
| `knowledgeUnderstandingStore` | 单概念的理解数据（摘要/框架/笔记/反馈/关系）、流式状态 | 不感知概念列表 |

### 4.2 跨 Store 通信

- **Store 之间不直接 import**，通过**组件层组合**传递数据。
- `KnowledgeAssociationView` 负责：从 `knowledgeStore` 读 selectedConceptId → 传给 `KnowledgeUnderstandingPage` → 后者初始化 `knowledgeUnderstandingStore`。
- 概念切换时，组件层调用 `resetForConcept(newId)` 清空理解 Store。

### 4.3 已知的 Store 边界问题

- **概念 ID 变更不自动触发 reset**：若 selectedConceptId 在 knowledgeStore 中变化，但 UnderstandingPage 未在视图栈中，不会触发 `resetForConcept`，可能导致页面打开时显示旧概念数据。
- **流式 buffer 与最终数据的双写**：流式期间用 `appendXxxChunk()` 累积 buffer，isFinal 后重新从 DB 加载最终结构化数据。两套路径并存，理论上一致，但 race condition（用户快速切换概念）时可能错乱。

---

## 五、架构约束（编码级宪章）

| 编号 | 规则 | 适用范围 |
|------|------|----------|
| A1 | 所有组件使用命名导出 `export function` | 所有 .tsx |
| A2 | 颜色、间距全部使用 CSS 变量 `var(--xxx)` | 所有 .css/.tsx |
| A3 | IPC 调用只通过 `lib/tauri-commands.ts` 封装 | 前端所有文件 |
| A4 | Store 之间不直接 import，通过组件层组合 | 所有 Store |
| A5 | Rust 用 snake_case，TypeScript 用 camelCase，Tauri 自动映射 | 所有命令参数 |
| A6 | Rust command 统一返回 `Result<T, String>` | 所有 commands/ |
| A7 | 所有 ID 使用 UUID v4 | 前后端均适用 |
| A8 | 时间戳统一 RFC3339 UTC | 所有 DB 字段 |

---

## 六、已知设计问题汇总

以下是经过代码分析识别的、当前设计中存在的问题，按影响层级分类：

### 6.1 用户体验层

| 问题 | 描述 | 影响 |
|------|------|------|
| UX-1 | 理解框架需手动触发但无明确引导 | 用户不知道"下一步"是什么 |
| UX-2 | 素材更新后摘要/框架不自动失效 | 用户看到过期内容却无感知 |
| UX-3 | 镜子反馈流式期间无中间状态可读 | 等待期间体验空洞 |
| UX-4 | "给我一个出发点"内容质量依赖 essenceSentence | 可能过于抽象，启发性有限 |
| UX-5 | 观点/案例/拓展的生成时机对用户不透明 | 用户不知道这些内容是什么时候生成的 |

### 6.2 数据一致性层

| 问题 | 描述 | 风险 |
|------|------|------|
| D-1 | userEdited 概念在重新扫描时的保护策略未文档化 | 用户编辑可能被覆盖 |
| D-2 | concept_relations 的自动提取逻辑不明确 | 关系数据可能为空，关系图无内容 |
| D-3 | 流式 buffer 与最终数据的 race condition | 快速切换概念时可能显示错误数据 |
| D-4 | mirrorFeedback 存储为 JSON 字符串，解析层在哪侧未统一说明 | 可能存在序列化不一致 |

### 6.3 功能完整性层

| 问题 | 描述 | 状态 |
|------|------|------|
| F-1 | synthesize_viewpoints 后端实现状态未确认 | 可能是空实现 |
| F-2 | generate_extensions 后端实现状态未确认 | 可能是空实现 |
| F-3 | qualityLevel 字段已采集但下游未使用 | 数据浪费 |
| F-4 | 概念列表无虚拟化，大量概念时性能未知 | 规模化风险 |
| F-5 | 知识理解无直接路由入口，只能从知识关联导航 | 功能可达性受限 |

---

## 七、迭代建议方向（供讨论）

以下方向**不是已决策的规划**，仅作为后续讨论的起点：

1. **知识进化（Knowledge Evolution）**：跟踪用户对概念理解的深化历史，让用户看到自己的理解随时间如何演变。
2. **主动学习触发（Active Recall）**：在合适时机（课前、课后）主动推送某个概念的"你上次是这样理解的，现在还记得吗？"。
3. **概念网络可视化**：将 concept_relations 中的关系渲染为可交互的知识图谱。
4. **跨概念综合**：当用户学了多个相关概念后，AI 帮助整合出"你已经掌握了XXX理论体系"的综合视图。
5. **笔记质量的渐进式引导**：镜子反馈的历史追踪，帮助用户看到"你的表达越来越清晰"。

---

## 八、关键文件索引

```
前端
├── src/types/knowledge.ts                          知识关联类型定义
├── src/types/knowledge-understanding.types.ts      知识理解类型定义
├── src/types/extraction.ts                         提取类型定义
├── src/stores/knowledgeStore.ts                    知识关联状态管理
├── src/stores/knowledgeUnderstandingStore.ts       知识理解状态管理
├── src/stores/extractionStore.ts                   提取状态管理
├── src/lib/tauri-commands.ts                       所有 IPC 封装（唯一出口）
├── src/components/features/knowledge/
│   ├── KnowledgeAssociationView.tsx                知识关联主容器
│   ├── ConceptList.tsx                             概念列表
│   ├── ConceptDetailPanel.tsx                      概念详情面板
│   ├── ViewpointCard.tsx                           观点卡片
│   ├── CaseCard.tsx                                案例卡片
│   └── ExtensionPanel.tsx                          拓展面板
└── src/components/KnowledgeUnderstanding/
    ├── KnowledgeUnderstandingPage.tsx              深入理解主页面
    ├── SummarySection.tsx                          摘要区
    ├── ExplanationSection.tsx                      理解框架区
    ├── ExplanationItem.tsx                         解释条目卡
    ├── UserNotesSection.tsx                        用户笔记区
    ├── MirrorFeedbackDisplay.tsx                   镜子反馈显示
    ├── RelationNetworkSection.tsx                  概念关系网络
    └── TransparencyBanner.tsx                      "基于文档"声明横幅

后端
├── src-tauri/src/db/knowledge.rs                   概念 CRUD
├── src-tauri/src/db/knowledge_understanding.rs     理解数据 CRUD
├── src-tauri/src/commands/knowledge.rs             概念命令
├── src-tauri/src/commands/knowledge_understanding.rs  理解命令
└── src-tauri/src/llm/
    ├── prompts.rs                                  Prompt 构建
    ├── chat.rs                                     LLM 调用
    └── client.rs                                   LLM 客户端
```

---

*本文档由 AI 基于代码库静态分析生成。如有与实际实现不符之处，以代码为准，并请更新本文档。*
