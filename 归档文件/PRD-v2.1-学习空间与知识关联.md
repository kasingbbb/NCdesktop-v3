# NCdesktop v2.1 PRD — 学习空间与知识关联

> 版本: v2.1.0-draft
> 日期: 2026-04-03
> 目标用户: 美国大学生（本科及研究生）

---

## 一、产品背景与目标

### 1.1 核心洞察

NoteCapt 目前解决了「素材采集 → 归档 → 回看」的基本链路，但对学生用户而言，缺少两个关键价值环：

| 价值环 | 当前状态 | 本次目标 |
|--------|---------|---------|
| **即时价值** — 课前预习 | 无 | 日历驱动的预习空间，AI 自动生成预习指南 |
| **长期价值** — 知识沉淀 | 素材堆积，无结构化 | 知识关联视图，概念级别的跨文档梳理 |

### 1.2 成功指标

- 日历导入成功率 ≥ 95%（支持主流 .ics 格式）
- 预习生成延迟 ≤ 15s（GPT-4o 级别模型）
- 知识关联页面中概念提取准确率 ≥ 80%（用户主观评价）
- 单用户单周使用预习功能 ≥ 3 次

---

## 二、功能一：课程日历导入（Calendar Import）

### 2.1 功能概述

用户可在设置面板（左下角齿轮图标 → 新增「课程日历」选项卡）一键导入学校课程表。导入后，侧边栏新增「课程 / Courses」分区，以时间线形式展示本周/下周课程。

### 2.2 导入方式

| 方式 | 优先级 | 说明 |
|------|--------|------|
| **本地 .ics 文件** | P0 | 几乎所有大学系统（Canvas, Blackboard, Banner, Workday Student）均支持导出 .ics。用户从学校系统下载后，拖入或文件选择器导入。 |
| **Google Calendar 订阅** | P1 | 通过 .ics URL 订阅（Google Calendar → 「Settings > Import & Export > Secret address in iCal format」），定期拉取更新。不走 OAuth，降低实现复杂度。 |

### 2.3 数据模型

```
CourseEvent {
  id: string (UUID)
  projectId: string | null       // 关联的 NoteCapt 项目（可选）
  title: string                   // 课程名 e.g. "ECON 101 - Intro to Microeconomics"
  courseCode: string | null        // 从 title 中解析 e.g. "ECON 101"
  instructor: string | null       // 教授名（从 DESCRIPTION 字段提取）
  location: string | null         // 教室
  startTime: string (RFC3339)     // 课程开始时间
  endTime: string (RFC3339)       // 课程结束时间
  recurrenceRule: string | null   // RRULE 字符串
  dayOfWeek: number[]             // 周几有课 [1,3,5] = MWF
  description: string | null      // 原始 DESCRIPTION
  calendarSource: "ics_file" | "ics_url"
  sourceUrl: string | null        // .ics URL（订阅模式）
  lastSynced: string | null       // 上次同步时间
  createdAt: string
}
```

### 2.4 导入流程

```
用户操作                        系统行为
  |                              |
  [设置 → 课程日历]              |
  |                              |
  [选择 .ics 文件 / 粘贴 URL]   → 解析 iCalendar (VEVENT)
  |                              → 提取: SUMMARY, DTSTART, DTEND, RRULE, LOCATION, DESCRIPTION
  |                              → 去重（UID + DTSTART）
  |                              → 展开 RRULE 为本学期的所有实例
  |                              |
  [预览课程列表]                 → 显示解析出的课程，用户可取消勾选不需要的
  |                              |
  [确认导入]                     → 写入 course_events 表
                                 → 侧边栏出现「课程」分区
```

### 2.5 侧边栏展示

在 `ProjectTree` 上方新增 `CourseSection` 分区：

```
┌─ Courses ─────────────────┐
│  ▸ Today (Mon, Apr 3)     │
│    09:00  ECON 101        │ ← 点击进入预习空间
│    11:00  CS 231          │
│    14:00  PHIL 220        │
│  ▸ Tomorrow (Tue, Apr 4)  │
│    10:00  MATH 301        │
│    13:00  HIST 150        │
│  ▸ This Week              │
│    ...                    │
└───────────────────────────┘
```

### 2.6 设置面板 — 课程日历选项卡

```
┌─ 课程日历 ─────────────────────────────────────────────────┐
│                                                            │
│  导入方式                                                   │
│  ┌────────────────────────────────────────────────────┐    │
│  │ [拖入 .ics 文件，或点击选择]                          │    │
│  └────────────────────────────────────────────────────┘    │
│                                                            │
│  — 或 —                                                    │
│                                                            │
│  iCal 订阅链接                                              │
│  ┌────────────────────────────────────────────────────┐    │
│  │ https://calendar.google.com/calendar/ical/...       │    │
│  └────────────────────────────────────────────────────┘    │
│  [订阅]                                                     │
│                                                            │
│  ─────────────────────────────────────────────────────     │
│                                                            │
│  已导入的日历                                                │
│  ● Spring 2026 Schedule    132 events    [刷新] [删除]     │
│                                                            │
│  自动刷新  [开启]  每 [ 6 ] 小时                             │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

---

## 三、功能二：AI 课程预习（AI Course Preview）

### 3.1 功能概述

用户点击侧边栏中的某个课程事件后，进入该课程的「预习空间」。系统自动调用 LLM，根据课程信息生成结构化的预习指南。用户可以阅读、标注、保存为笔记。

### 3.2 触发方式

- 点击侧边栏课程事件 → 进入预习空间
- 预习空间替换当前的 ContentArea（非模态）
- 顶部显示课程信息栏：课程名、时间、教授、教室

### 3.3 预习空间布局

```
┌─────────────────────────────────────────────────────────────────┐
│ ← Back    ECON 101 · Intro to Microeconomics                   │
│           Mon Apr 3, 09:00-10:15 · Prof. Smith · Room 302      │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─ 预习指南 ──────────────────────────────────────────────┐   │
│  │                                                          │   │
│  │  ## 本次课程主题预测                                       │   │
│  │  Based on course progression, this session likely         │   │
│  │  covers: **Price Elasticity of Demand**                   │   │
│  │                                                          │   │
│  │  ## 核心概念 Key Concepts                                 │   │
│  │  1. Price Elasticity of Demand (PED)                      │   │
│  │  2. Elastic vs. Inelastic goods                           │   │
│  │  3. Determinants of elasticity                            │   │
│  │  ...                                                     │   │
│  │                                                          │   │
│  │  ## 课前思考问题 Pre-Class Questions                       │   │
│  │  1. Why might luxury goods be more elastic than           │   │
│  │     necessities?                                          │   │
│  │  ...                                                     │   │
│  │                                                          │   │
│  │  ## 与已有知识的联系 Connections                            │   │
│  │  📎 Your note from Lecture 3 mentioned "supply shifts"    │   │
│  │     — elasticity builds directly on that foundation.      │   │
│  │                                                          │   │
│  │  ## 推荐预读 Suggested Reading                             │   │
│  │  • Mankiw Ch.5: Elasticity and Its Application            │   │
│  │  • Khan Academy: Intro to Elasticity (video, 12 min)      │   │
│  │                                                          │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
│  ┌─ 我的预习笔记 ──────────────────────────────────────────┐   │
│  │  (可编辑的 Markdown 区域，自动保存为 Note)                  │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
│  [重新生成]  [保存为素材]                                        │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 3.4 AI 预习 Prompt

以下是发送给 LLM 的完整 System + User Prompt：

#### System Prompt

```
You are a world-class academic tutor specializing in helping American college
students prepare for their upcoming classes. Your goal is to create a concise,
actionable preview guide that helps the student walk into class feeling confident
and ready to engage.

Rules:
- Write at a level appropriate for undergraduate/graduate college students
- Be concise — students are time-constrained; aim for 5-minute read max
- Use both English section headers and bilingual hints where helpful
- Ground your suggestions in the course context provided
- If the student has prior notes/documents, reference them specifically to show
  continuity
- Do NOT fabricate specific page numbers or reading assignments — use general
  chapter/topic references
- Structure output in Markdown
```

#### User Prompt Template

```
# Course Preview Request

## Course Info
- Course: {{courseTitle}}
- Course Code: {{courseCode}}
- Instructor: {{instructor}}
- Session Time: {{startTime}} — {{endTime}}
- Location: {{location}}

## Context
- This is session #{{sessionNumber}} of the semester (approx. week {{weekNumber}})
- Course description: {{courseDescription}}
{{#if previousTopics}}
- Topics covered in recent sessions (from student's notes):
{{previousTopics}}
{{/if}}

{{#if relatedAssets}}
## Student's Related Materials
The student has these relevant materials in their library:
{{relatedAssets}}
{{/if}}

## Task
Generate a structured preview guide with these sections:

### 1. 本次课程主题预测 (Predicted Topic)
Based on the course progression and week number, predict what this session
will likely cover. Be specific but note this is a prediction.

### 2. 核心概念 (Key Concepts to Preview)
List 3-5 key concepts the student should familiarize themselves with before
class. For each concept, give a 1-2 sentence plain-language explanation.

### 3. 课前思考问题 (Pre-Class Thinking Questions)
Pose 2-3 thought-provoking questions that will prime the student's thinking.
These should be questions that the lecture will help answer.

### 4. 与已有知识的联系 (Connections to Prior Knowledge)
If the student has prior notes or materials, draw explicit connections.
If not, connect to general prerequisite knowledge.

### 5. 推荐预读 (Suggested Pre-Reading)
Suggest 1-2 accessible resources (textbook chapter topics, short videos,
articles) that would help. Be general (e.g., "Chapter on X" rather than
"page 142").
```

### 3.5 预习数据流

```
1. 用户点击课程事件
2. 前端从 course_events 获取课程信息
3. 前端从 notes / assets 中检索该课程相关的历史内容（基于 courseCode 匹配 projectId）
4. 组装 prompt → 调用 LLM (llmProbe 或新增 llmPreview 命令)
5. 流式返回 Markdown → 渲染到预习空间
6. 用户可编辑下方笔记区 → 自动保存为 Note（关联到 courseEventId）
```

### 3.6 预习结果存储

```
CoursePreview {
  id: string (UUID)
  courseEventId: string            // 关联的课程事件
  content: string                  // AI 生成的 Markdown 预习内容
  userNotes: string | null         // 用户的预习笔记
  model: string                    // 使用的模型名
  generatedAt: string
  createdAt: string
}
```

---

## 四、功能三：知识关联（Knowledge Association）

### 4.1 第一性原理分析

从第一性原理出发，学生的知识管理有一个根本矛盾：

> **信息是按课程/时间线性输入的，但知识本身是网状结构。**

一个学生在 ECON 101 学了「边际效用递减」，在 PSYCH 201 遇到了「享乐适应」（Hedonic Adaptation），在 BIO 110 看到了「药物耐受性」—— 这三个概念本质上是同一个底层原理的不同表现。但在传统笔记系统中，它们被锁在三个不同的文件夹里，永远不会相遇。

**知识关联的核心目的：打破课程边界，让概念自己找到彼此。**

四个模块的设计逻辑：

```
                         用户的所有文档
                              │
                    ┌─────────┼─────────┐
                    ▼         ▼         ▼
               ┌────────┐ ┌────────┐ ┌────────┐
               │ECON 101│ │PSYCH201│ │BIO 110 │  ← 按课程存储
               └────────┘ └────────┘ └────────┘
                    │         │         │
                    └─────────┼─────────┘
                              ▼
                    AI 概念提取 & 聚合
                              │
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
        ┌──────────┐   ┌──────────┐   ┌──────────┐
        │  概念     │   │ 相关观点  │   │ 观点案例  │
        │ Concepts │   │Viewpoints│   │  Cases   │
        │          │   │          │   │          │
        │ 从文档中  │   │ 同一概念  │   │ 文档中的  │
        │ 提取的    │   │ 在不同课  │   │ 具体引用  │
        │ 核心术语  │   │ 程/语境   │   │ 和论据    │
        │          │   │ 下的不同  │   │          │
        │ 用户可    │   │ 解读，AI  │   │ 锚定到    │
        │ 编辑/删除 │   │ 归纳整理  │   │ 原始素材  │
        └──────────┘   └──────────┘   └──────────┘
              │
              ▼
        ┌──────────┐
        │ 知识拓展  │
        │Extension │
        │          │
        │ 这个概念  │
        │ 的上游    │
        │ (前置知识) │
        │ 和下游    │
        │ (应用方向) │
        │          │
        │ 只读展示  │
        │ 暂不可展开│
        └──────────┘
```

### 4.2 功能入口

在 `Toolbar.tsx` 的项目级工具栏中，时间流按钮右侧新增「知识关联」按钮：

```
┌────────────────────────────────────────────────────┐
│ ← Back   Project Name   [Grid|List] [Timeline] [知识关联] │
└────────────────────────────────────────────────────┘
```

点击后，ContentArea 切换为知识关联视图（替换 AssetListView）。

### 4.3 数据模型

```
Concept {
  id: string (UUID)
  libraryId: string               // 所属知识库
  name: string                    // 概念名 e.g. "Diminishing Marginal Utility"
  aliases: string[]               // 别名 e.g. ["边际效用递减", "DMU"]
  definition: string              // 一句话定义（AI 生成，用户可编辑）
  sourceAssetIds: string[]        // 从哪些素材中提取出的
  sourceProjectIds: string[]      // 涉及哪些项目/课程
  userEdited: boolean             // 用户是否手动修改过
  createdAt: string
  updatedAt: string
}

ConceptViewpoint {
  id: string (UUID)
  conceptId: string               // 所属概念
  perspective: string             // 观点标题 e.g. "Economic perspective: rational choice theory"
  summary: string                 // AI 生成的观点摘要
  sourceContext: string           // 来源语境 e.g. "ECON 101 - Lecture 5"
  sourceAssetId: string           // 来源素材
  generatedAt: string
}

ConceptCase {
  id: string (UUID)
  conceptId: string               // 所属概念
  title: string                   // 案例标题
  excerpt: string                 // 原文摘录
  sourceAssetId: string           // 来源素材
  sourceLocation: string | null   // 在素材中的位置提示（段落编号等）
  relevanceNote: string | null    // AI 的关联说明
}

ConceptExtension {
  id: string (UUID)
  conceptId: string               // 所属概念
  direction: "upstream" | "downstream"  // 前置知识 or 应用方向
  name: string                    // 拓展知识名
  description: string             // 一句话描述
  relationship: string            // 与当前概念的关系 e.g. "prerequisite", "application"
}
```

### 4.4 知识关联视图布局

```
┌─────────────────────────────────────────────────────────────────┐
│  知识关联  ·  共提取 47 个概念  ·  来自 12 个项目               │
│  [搜索概念...]                            [重新扫描] [筛选 ▾]   │
├──────────────────┬──────────────────────────────────────────────┤
│                  │                                              │
│  概念列表         │  边际效用递减                                 │
│  ──────────      │  Diminishing Marginal Utility                │
│                  │  ──────────────────────────────              │
│  📌 边际效用递减  │                                              │
│    3 个项目引用   │  定义 Definition                             │
│                  │  ┌──────────────────────────────────────┐   │
│  📌 供需均衡      │  │ The decrease in additional satisfaction│   │
│    2 个项目引用   │  │ gained from consuming one more unit   │   │
│                  │  │ of a good or service.                 │   │
│  📌 认知偏差      │  │                              [编辑]   │   │
│    4 个项目引用   │  └──────────────────────────────────────┘   │
│                  │                                              │
│  📌 享乐适应      │  相关观点 Viewpoints (3)                     │
│    2 个项目引用   │  ┌──────────────────────────────────────┐   │
│                  │  │ 🔹 经济学视角 (ECON 101)              │   │
│  📌 信息不对称    │  │   理性消费者在边际效用递减下的最优       │   │
│    1 个项目引用   │  │   配置策略...                          │   │
│                  │  │                                       │   │
│  ...             │  │ 🔹 心理学视角 (PSYCH 201)             │   │
│                  │  │   享乐适应与边际效用递减的认知机制...     │   │
│                  │  │                                       │   │
│                  │  │ 🔹 生物学视角 (BIO 110)               │   │
│                  │  │   神经递质的受体脱敏与药物耐受...        │   │
│                  │  └──────────────────────────────────────┘   │
│                  │                                              │
│                  │  观点案例 Cases (4)                           │
│                  │  ┌──────────────────────────────────────┐   │
│                  │  │ 📎 "The first slice of pizza brings   │   │
│                  │  │    immense joy, but by the 5th..."    │   │
│                  │  │    — ECON 101 Lecture 5 notes         │   │
│                  │  │                         [查看原文]     │   │
│                  │  │                                       │   │
│                  │  │ 📎 "Hedonic treadmill experiment:     │   │
│                  │  │    lottery winners reported..."        │   │
│                  │  │    — PSYCH 201 Reading summary         │   │
│                  │  │                         [查看原文]     │   │
│                  │  └──────────────────────────────────────┘   │
│                  │                                              │
│                  │  知识拓展 Extension                           │
│                  │  ┌──────────────────────────────────────┐   │
│                  │  │  ⬆ 前置知识 Prerequisites              │   │
│                  │  │  · 效用理论 (Utility Theory)           │   │
│                  │  │  · 消费者行为学 (Consumer Behavior)    │   │
│                  │  │                                       │   │
│                  │  │  ⬇ 应用方向 Applications               │   │
│                  │  │  · 价格歧视策略 (Price Discrimination) │   │
│                  │  │  · 产品定价模型 (Pricing Models)       │   │
│                  │  │  · 行为经济学 (Behavioral Economics)   │   │
│                  │  └──────────────────────────────────────┘   │
│                  │                                              │
└──────────────────┴──────────────────────────────────────────────┘
```

### 4.5 概念提取流程

概念提取是异步后台任务，触发时机：

1. **首次进入知识关联视图** — 扫描当前知识库所有素材（增量，跳过已处理的）
2. **新素材导入后** — 自动对新素材做概念提取
3. **用户手动触发「重新扫描」**

#### 提取 Prompt

```
System: You are a knowledge extraction engine. Given a student's academic
document, extract key concepts with precision.

User:
# Document Analysis Request

## Document
Title: {{assetName}}
Project/Course: {{projectName}}
Content:
---
{{documentContent}}
---

## Task
Extract all significant academic concepts from this document. For each concept:

1. **name**: The canonical English term
2. **aliases**: Alternative names (including translations if bilingual)
3. **definition**: A one-sentence definition as used in this context
4. **excerpts**: 1-2 direct quotes from the document that discuss this concept

Return as JSON array:
[
  {
    "name": "...",
    "aliases": ["..."],
    "definition": "...",
    "excerpts": ["..."]
  }
]

Rules:
- Only extract concepts that are substantive (not generic terms like "example" or "chapter")
- Prefer established academic terminology
- If the same concept appears multiple times, merge into one entry
- Include 3-10 concepts per document (fewer for short documents)
```

### 4.6 观点聚合逻辑

当同一概念（通过 name/alias 匹配）出现在多个素材中时，AI 聚合出观点：

```
System: You are a knowledge synthesis engine. You help students see how the
same concept appears across different courses and contexts.

User:
# Viewpoint Synthesis Request

## Concept: {{conceptName}}
Definition: {{conceptDefinition}}

## Appearances across student's documents:

### Context 1: {{projectName1}} ({{assetName1}})
{{excerpt1}}

### Context 2: {{projectName2}} ({{assetName2}})
{{excerpt2}}

(... more contexts ...)

## Task
For each context where this concept appears, synthesize a viewpoint:
1. **perspective**: A title like "Economic perspective" or "Psychological lens"
2. **summary**: 2-3 sentences explaining how this concept is understood/applied
   in this particular context
3. **sourceContext**: Which course/document this perspective comes from

Return as JSON array.
```

### 4.7 用户交互

| 操作 | 行为 |
|------|------|
| 点击左侧概念 | 右侧展示该概念详情 |
| 点击「编辑」(定义区) | 变为可编辑文本框，保存后标记 userEdited=true |
| 点击「查看原文」(案例区) | 打开 DocumentViewer 定位到对应素材 |
| 搜索框输入 | 实时过滤概念列表（模糊匹配 name + aliases） |
| 「重新扫描」按钮 | 对所有素材重新提取概念（覆盖 AI 生成内容，保留用户编辑） |
| 「筛选」下拉 | 按项目/课程筛选概念来源 |

### 4.8 不做什么（v2.1 Scope）

- **不做** 知识拓展的点击深入学习（标记为 "Coming Soon"）
- **不做** 概念之间的手动连线/图谱可视化
- **不做** 实时协作/共享概念库
- **不做** 概念的自动合并（仅提示可能重复，由用户手动确认）

---

## 五、技术约束与非功能性需求

### 5.1 性能

| 指标 | 目标 |
|------|------|
| 日历解析 (.ics, 500 events) | ≤ 2s |
| 预习生成（首字延迟） | ≤ 3s |
| 概念提取（单文档） | ≤ 10s |
| 知识关联页面首屏加载 | ≤ 500ms（数据库查询） |
| 概念搜索 | ≤ 100ms |

### 5.2 数据安全

- 所有课程和概念数据存储在本地 SQLite，不上传
- LLM 调用走用户自配的 API endpoint（与现有 AI 设置复用）
- .ics URL 订阅仅读取，不修改远程日历

### 5.3 离线能力

- 日历数据导入后完全离线可用
- 预习和概念提取需要 LLM API 连接
- 已生成的预习内容和概念数据离线可查看

---

## 六、迭代计划

| 阶段 | 范围 | 预计 |
|------|------|------|
| Phase 1 | 日历导入 (.ics 文件) + 侧边栏课程展示 | Sprint 1 |
| Phase 2 | AI 预习生成 + 预习空间 UI | Sprint 1-2 |
| Phase 3 | 知识关联 — 概念提取 + 列表展示 | Sprint 2 |
| Phase 4 | 知识关联 — 观点聚合 + 案例 + 拓展 | Sprint 3 |

---

## 附录：竞品参考

| 产品 | 相关功能 | NoteCapt 差异点 |
|------|---------|----------------|
| Notion Calendar | 日历集成 | NoteCapt 聚焦学术预习，不做通用日历 |
| Readwise | 知识高亮回顾 | NoteCapt 做跨文档概念关联，而非单文档高亮 |
| Obsidian Graph | 知识图谱 | NoteCapt 由 AI 自动提取概念，无需用户手动 [[link]] |
| Quizlet | 学习卡片 | NoteCapt 保留原始语境，不做脱离语境的闪卡 |
