# NCdesktop 知识功能迭代 PRD v1.0
# 从「概念词汇表」到「知识理解辅助层」

> 版本：v1.0
> 日期：2026-04-11
> 基于：Debate Session 001（4 层完整辩论，5 轮）
> 目标读者：Conductor（下一阶段开发 Agent）
> 迭代基础：v2.1 PRD 第四章「知识关联」功能

---

## 一、产品背景与核心问题

### 1.1 现状与核心矛盾

NCdesktop v2.1 已经实现了「知识关联」功能，能够从用户文档中提取概念名称、一句话定义、原文摘录，并聚合同一概念在不同文档中的观点。

**根本问题**：这个实现的本质是「概念词汇表（Glossary）」，只解决了学习科学 Bloom 分类法第一层——**Remember（记忆/识别）**，即用户知道「这个概念存在」。

**缺失的价值**：从 Bloom 第一层到真正能使用知识，还有五层：
```
Remember（记忆）  ← v2.1 解决到这里
Understand（理解） ← 本次迭代目标
Apply（应用）
Analyze（分析）
Evaluate（评估）
Create（创造）
```

用户明确指出的问题：「光知道概念名称没用，知识获取 → 吸收 → 成长 → 进化 → 能引用，是一个递进过程，当前只解决了第一步。」

### 1.2 产品定位（Debate Layer 1 共识）

**本次迭代的定位：知识理解辅助层（Knowledge Understanding Assistance）**

- **辅助**（Assistance）而非**保证**（Guarantee）——软件提供条件，用户提供主动认知投入
- 用户的主动投入是不可替代的；软件的价值在于降低主动投入的摩擦成本
- **不追求**「软件检测到用户理解了知识」；**追求**「用户在需要深入理解时，有可用的高质量工具支持」

### 1.3 与现有功能的关系

| 功能 | 触发时机 | 用户意图 | 内容来源 | 持久性 |
|------|----------|----------|----------|--------|
| **课程预习** | 课前（有时间压力） | 「我需要在2小时内准备好上课」 | LLM + 课程信息 + 历史笔记 | 一次性消费 |
| **知识理解**（本次） | 任意时间（无时间压力） | 「我想真正搞懂这个概念」 | 严格基于用户文档 | 永久积累 |

两者**不竞争**——时间轴不同、用户意图不同、数据来源不同、使用心智模型不同。

### 1.4 成功标准

| 指标 | 目标值 | 说明 |
|------|--------|------|
| 「深入理解」入口点击率 | ≥ 40% | 在已查看过的概念中，至少 40% 被点击深入理解 |
| 用户添加个人理解注释的概念比例 | ≥ 10% | 用户主动产出是最强的理解信号 |
| 用户满意度 | ≥ 4/5 | 「这个功能帮助我更好地理解了这个概念」 |
| 深入理解页面首屏加载 | ≤ 500ms | 文档整合摘要（来自已提取数据，无需新 LLM 调用） |
| 理解框架生成首字延迟 | ≤ 3s | LLM 流式输出 |

---

## 二、用户定义与核心场景

### 2.1 目标用户

**主要用户**：美国大学生（本科及研究生），使用 NCdesktop 管理学术笔记和课程材料

**用户画像（与知识功能相关）**：
- 已经积累了多门课程的笔记/文档（NCdesktop 的素材库）
- 遇到了跨课程的同类概念（比如「边际效用递减」出现在经济学和心理学里）
- 希望真正理解某个概念，而不只是知道定义
- 时间有限，需要高效的学习辅助，而非完整的学习平台

### 2.2 核心场景

**场景 A（最核心）：单概念深度理解**
用户在知识关联视图中，点击一个概念后感觉「我知道这个词但说不清楚」，主动触发「深入理解」，希望系统帮助他真正搞懂这个概念——基于他自己已经读过的文档。

**场景 B：验证自己的理解**
用户读完了理解框架后，想测试自己是否真的理解了，尝试「用我的话说」写下自己的解释，通过 AI 镜子反馈发现自己遗漏了哪些要点。

**场景 C：发现概念联系**
用户在查看某个概念时，通过知识关系网络发现这个概念和另一个课程里的概念有关联，打开关联概念，在两个概念之间来回对照，形成跨课程的理解。

---

## 三、功能需求（带优先级）

### P0：本次迭代必须交付

---

#### 功能 1：「深入理解」入口（Feature Discovery）

**功能描述**：
在 v2.1 概念详情页面（右侧区域）新增「深入理解」按钮，作为进入理解模式的唯一入口。

**详细规范**：
- 按钮位置：概念详情页右侧区域，定义区域（Definition）的右上角
- 按钮样式：蓝色高亮（相比其他灰色辅助按钮，视觉上显著），文字为「深入理解」
- 点击行为：展开或跳转到「深入理解」页面（替换 ContentArea，保留返回入口）
- 首次引导：用户**第一次进入知识关联视图时**，显示一次性 Tooltip 指向该按钮，文字为：「点击「深入理解」，让 AI 基于你的文档帮你真正理解这个概念」——仅显示一次，不重复打扰
- 空状态引导：对于用户还没有触发过「深入理解」的概念，定义区域下方显示引导文字「想深入理解这个概念？」（弱引导，不打断查找模式）

**验收标准**：
- [ ] 按钮在所有已有概念的详情页面上可见
- [ ] 首次进入知识关联视图时 Tooltip 正确显示，且之后不再重复显示
- [ ] 点击按钮后正确触发深入理解页面
- [ ] 返回入口可用（用户可返回概念列表）

---

#### 功能 2：文档整合摘要（Document Integration Summary）

**功能描述**：
「深入理解」页面的首屏内容——将用户文档中关于该概念的所有原文摘录，整合成一段带来源标注的连贯摘要。

**详细规范**：
- 位置：深入理解页面最顶部，标题「你的文档怎么说」
- 内容来源：从已提取的 `concept_cases.excerpt` 和 `concept_viewpoints.summary` 聚合，通过新的 LLM 调用整合（不是直接拼接原文）
- 加载策略：首屏优先加载，目标 ≤ 500ms（可基于已有数据快速生成，无需等待理解框架）
- 来源标注：每个信息来自的文档名称显示在对应段落旁，可点击跳转到原文
- 展开原文：提供「展开原文」入口，允许用户查看完整的原始引用（保证细节不丢失）

**新增 Tauri Command**：`generate_concept_summary`

**新增 SQLite 表**：
```sql
concept_summaries (
  id TEXT PRIMARY KEY,
  concept_id TEXT NOT NULL,  -- 关联 concepts.id
  summary TEXT NOT NULL,     -- AI 整合的摘要
  source_asset_ids TEXT NOT NULL,  -- JSON 数组，来源素材 ID
  generated_at TEXT NOT NULL,
  FOREIGN KEY (concept_id) REFERENCES concepts(id)
)
```

**验收标准**：
- [ ] 首屏在 500ms 内加载完成（无需新 LLM 调用的快速路径）
- [ ] 摘要内容有来源标注，来源链接可点击
- [ ] 「展开原文」功能可用
- [ ] 数据存入 `concept_summaries` 表，不修改 `concepts` 表

---

#### 功能 3：理解框架生成（Understanding Framework）

**功能描述**：
「深入理解」页面的核心内容——LLM 基于用户文档，生成结构化的解释框架，帮助用户从碎片化的原文摘录中理解概念的核心机制。

**内容模块**：
| 模块 | 内容 | 来源约束 |
|------|------|----------|
| 核心机制 | 「这个概念的运作方式是...」 | 严格基于用户文档，LLM 重组表达 |
| 典型场景 | 「在你的文档中，这个概念出现在...这些情境里」 | 严格引用用户文档中的具体情境 |
| 常见误区 | 「很多人会把它和 X 混淆，区别是...」（X 来自用户文档中的相关概念） | 只能引用用户文档中出现过的相关概念 |
| 一句话精华 | 帮助记忆的核心句，浓缩概念本质 | LLM 生成，需标注「根据你的文档总结」 |

**透明度要求**（关键安全约束）：
- 每个模块条目必须附带来源链接「来源：[文档名]」
- 增加「查看依据」按钮——点击后展示 LLM 在生成该条目时实际使用的原文段落
- 页面顶部永久透明度声明：「以下解释基于你的文档由 AI 生成，AI 可能有理解偏差——点击来源链接查看原文对照」

**幻觉风险缓解（Prompt 层约束）**：
```
System Prompt 核心约束（必须包含）：
"You are a knowledge explanation engine. You MUST ONLY use information from 
the user's documents provided below. Do NOT introduce any information not 
present in these documents. For each explanatory point, you MUST cite which 
document it comes from using the format [Source: document_name]."
```

**LLM 调用策略**：
- 按需触发：用户首次点击「深入理解」时触发，非自动触发
- 流式输出：使用流式 API，首字延迟 ≤ 3s
- 结果缓存：生成完成后存入 `concept_explanations` 表，不重复调用（除非用户主动「重新生成」）

**新增 Tauri Command**：`generate_concept_explanation`

**新增 SQLite 表**：
```sql
concept_explanations (
  id TEXT PRIMARY KEY,
  concept_id TEXT NOT NULL,
  mechanism TEXT NOT NULL,        -- 核心机制
  typical_scenarios TEXT NOT NULL,-- 典型场景（JSON，带来源标注）
  common_misconceptions TEXT,     -- 常见误区（JSON，带来源标注）
  essence_sentence TEXT NOT NULL, -- 一句话精华
  source_asset_ids TEXT NOT NULL, -- JSON 数组
  model TEXT NOT NULL,            -- 使用的模型名
  generated_at TEXT NOT NULL,
  FOREIGN KEY (concept_id) REFERENCES concepts(id)
)
```

**验收标准**：
- [ ] 每个解释条目有可点击的来源链接
- [ ] 「查看依据」功能展示对应的原文段落
- [ ] 页面顶部透明度声明可见
- [ ] 流式加载，首字 ≤ 3s
- [ ] 结果缓存到 `concept_explanations`，第二次打开无需重新调用 LLM
- [ ] 提供「重新生成」按钮，允许用户主动刷新

---

#### 功能 4：「用我的话说」+ AI 镜子反馈

**功能描述**：
理解框架下方的自由文本输入区，用户用自己的话解释这个概念；AI 镜子反馈对比用户解释与文档原意，告知用户覆盖了哪些要点、遗漏了哪些——措辞为「探索式」而非「批改式」。

**「用我的话说」区域规范**：
- 独立于 v2.1 的「定义编辑」功能——两者数据完全分离：
  - `concepts.definition`：官方定义（AI 生成或用户编辑），v2.1 已有，不修改
  - `concept_user_notes.user_explanation`：用户的个人理解表达（本次新增），仅供自我学习
- 文本框标题：「用你自己的话解释这个概念」
- 辅助入口：「给我一个出发点」按钮，LLM 生成可编辑草稿（让用户可以从基础修改，而非面对空白）
- 自动保存：用户停止输入 1s 后自动保存，无需手动保存
- 提交入口：「和 AI 核对一下」按钮，触发镜子反馈

**AI 镜子反馈规范**：
- 触发：用户主动点击「和 AI 核对一下」（不自动触发）
- 输出格式（措辞必须符合探索式原则）：
  ```
  你的解释捕捉到了 [X] 个核心要点 ✓
  
  在你的文档里，还有一些关于这个概念的角度你可能感兴趣：
  · [遗漏要点 1]（来源：[文档名]）
  · [遗漏要点 2]（来源：[文档名]）
  
  你的理解和文档的一个细微差异是：[差异说明]
  ```
- **禁止的措辞**：「你的解释不完整」「你的理解有误」「你遗漏了」「错误是...」
- **必须的措辞**：「还有一些角度」「文档里提到的」「你可能感兴趣」
- 严格锚定：反馈内容只能基于用户文档，不能引入通用知识批评用户的解释

**新增 Tauri Command**：`validate_user_explanation`

**新增 SQLite 字段/表**：
```sql
concept_user_notes (
  id TEXT PRIMARY KEY,
  concept_id TEXT NOT NULL,
  user_explanation TEXT NOT NULL,  -- 用户自己的解释
  mirror_feedback TEXT,            -- 最近一次 AI 镜子反馈（JSON）
  last_validated_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (concept_id) REFERENCES concepts(id)
)
```

**验收标准**：
- [ ] 「用我的话说」区域与「编辑定义」区域在 UI 上有明确区分（位置/标签不同）
- [ ] 两者数据存储在不同字段，不互相覆盖
- [ ] 自动保存工作正常（停止输入 1s 后保存）
- [ ] 镜子反馈输出的措辞符合探索式原则（无批改式语言）
- [ ] 镜子反馈每个要点均有文档来源标注

---

#### 功能 5：概念关系网络（共现版）

**功能描述**：
展示与当前概念在用户文档中共现的其他概念，以卡片列表形式呈现，帮助用户建立「这个概念连接了哪些知识」的系统性视图。

**数据来源与计算**：
- **共现关系**：概念 A 和概念 B 在同一文档中同时出现 → 一条共现关系边
- **计算时机**：在概念提取完成后的异步步骤中计算（与概念提取流程一致），**不需要 LLM 调用**
- **算法**：O(n²) 概念数，对于 47 个概念约 1081 对，每对做一次数据库查询（检查两个概念的 sourceAssetIds 是否有交集）

**UI 呈现规范**：
- 位置：深入理解页面底部，标题「在你的知识库里，这个概念连接了：」
- 每个关联概念显示：
  - 概念名称（可点击，导航到该概念的详情页）
  - 「一起出现在 [文档名]」（透明化数据来源，不宣称深层联系）
  - 共现次数越高，排序越靠前
- 排序：按共现次数降序，显示前 5-8 个关联概念
- 空状态：「暂时还没发现相关概念。随着你导入更多文档，关联会逐渐丰富。」

**透明度声明（重要）**：UI 文字只说「一起出现在你的文档中」，**不说**「紧密相关」或「有深层联系」——用户自己判断关联的意义。

**v2.1 ConceptExtension 升级**：
- 将现有的 upstream/downstream 关系接入关系网络展示
- 标注类型：「前置知识」/「应用方向」（与共现关系视觉区分）

**新增 SQLite 表**：
```sql
concept_relations (
  id TEXT PRIMARY KEY,
  concept_a_id TEXT NOT NULL,
  concept_b_id TEXT NOT NULL,
  relation_type TEXT NOT NULL,  -- "co_occurrence" | "upstream" | "downstream"
  source_asset_ids TEXT NOT NULL, -- JSON 数组，哪些文档中共现
  co_occurrence_count INTEGER DEFAULT 1,
  created_at TEXT NOT NULL,
  FOREIGN KEY (concept_a_id) REFERENCES concepts(id),
  FOREIGN KEY (concept_b_id) REFERENCES concepts(id)
)
```

**验收标准**：
- [ ] 概念关系网络在深入理解页面底部正确展示
- [ ] 关联概念卡片显示「一起出现在 [文档名]」
- [ ] 按共现次数降序排列
- [ ] 点击关联概念可导航到该概念详情页
- [ ] 共现关系计算在概念提取流程结束后自动执行（非 LLM 调用）
- [ ] 空状态文字正确显示

---

#### 功能 6：数据模型增量升级（数据迁移安全保障）

**这是 MVP 阻塞项，必须在所有其他 P0 功能之前设计完成。**

**核心原则**：
1. **不修改 v2.1 已有表**：`concepts`、`concept_viewpoints`、`concept_cases`、`concept_extensions` 保持不变
2. **增量添加**：所有新数据通过新建表存储，与已有表通过 `concept_id` 外键关联
3. **保护用户手动编辑**：`concepts.definition` 中 `user_edited = true` 的内容不被任何新功能修改
4. **按需生成**：新功能的数据在用户**首次点击「深入理解」**时生成，而非升级时批量预生成

**4张新增表（完整定义见上方各功能）**：
- `concept_summaries` — 文档整合摘要
- `concept_explanations` — 理解框架
- `concept_user_notes` — 用户个人理解笔记
- `concept_relations` — 概念关系网络

**验收标准**：
- [ ] v2.1 已有数据完整保留，无数据丢失
- [ ] `user_edited = true` 的定义未被覆盖
- [ ] 4张新表结构正确，外键约束完整
- [ ] 所有新 LLM 生成内容存储在新表中，不写入已有表

---

### P1：第一次迭代后交付

#### 功能 7：Socratic 自测

**功能描述**：
LLM 基于用户文档生成 1-2 个思辨式问题，用户可文字回答，AI 给出「你的回答 vs 文档的角度」对比。

**降级原因**：自测的题目质量对用户体验影响极大；MVP 阶段先通过「用我的话说」验证主动产出功能的价值，积累 Prompt 工程经验后再引入自测。

---

#### 功能 8：文件夹/方向级「知识结构概览」

**功能描述**：
用户可在文件夹或项目视图中触发「这个范围内的知识结构」，展示该范围内的概念列表 + 核心概念关系（共现网络的子图）。

**降级原因**：需要解决「范围选择」的 UX 设计；范围越大，展示越复杂；MVP 阶段先验证单概念深度理解的价值再扩展到范围级。

---

#### 功能 9：语义关联（Lazy LLM）

**功能描述**：
用户首次打开某概念的深入理解页面时，触发一次 LLM 调用识别超出共现关系的语义关联，结果缓存。

**降级原因**：共现关系已能满足 MVP 的知识网络需求；语义关联的质量取决于 LLM 对用户文档内容的理解深度，MVP 阶段先观察共现关系的用户接受度。

---

### P2：后续版本

#### 功能 10：Layer 3 通用知识背景（可关闭）

**降级原因**：安全边界模糊（用户开启后实际允许 LLM 引入超出用户文档的知识，需要更明确的 UX 设计来区分来源）；需要更多用户研究来确认需求真实性。

---

#### 功能 11：知识技能/工作流输出

**功能描述**：基于用户已积累的个人理解注释，推荐可能的应用场景或工作流（将知识「激活」为技能）。

**降级原因**：这是 session_context.md 中标注的「未来方向」，需要先在 MVP 中积累足够的用户理解数据才有意义推进。

---

## 四、深入理解页面整体布局

```
┌─────────────────────────────────────────────────────────────────┐
│ ← 返回概念列表                                                    │
│                                                                  │
│  认知失调 · Cognitive Dissonance                                  │
│  来自 3 个文档 · PSYCH 201, COMM 301, ECON 101                   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ ⚠️ 以下解释基于你的文档由 AI 生成，AI 可能有理解偏差         │   │
│  │    ——点击来源链接查看原文对照                               │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  ───── 你的文档怎么说 ─────                                       │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ 认知失调指个体在持有相互矛盾的认知时产生的心理不适感...       │   │
│  │ [来源: PSYCH 201 Reading Week 4]  [展开原文]              │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  ───── 深入理解 ─────                          [重新生成]         │
│                                                                  │
│  核心机制                                                         │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ 当个体的行为与其信念不一致时，会产生心理紧张感，驱动其       │   │
│  │ 通过改变信念或行为来消除紧张                               │   │
│  │ [来源: PSYCH 201] [查看依据]                              │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  典型场景                                                         │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ · 消费决策后的合理化（ECON 101：理性选择框架中的例外...）    │   │
│  │ · 说服与态度改变（COMM 301：说服理论中的自我说服...）       │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  常见误区                                                         │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ 与「自我欺骗」的区别：认知失调是无意识的心理机制，不是主动   │   │
│  │ 选择欺骗。[来源: PSYCH 201] [查看依据]                    │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  一句话精华                                                       │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ "当你做了和你相信的矛盾的事，你会改变信念来让自己好受"       │   │
│  │ （根据你的文档总结）                                        │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  ───── 用你自己的话解释这个概念 ─────                              │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                                                          │   │
│  │  [自由文本输入区，自动保存]                                 │   │
│  │                                                          │   │
│  └──────────────────────────────────────────────────────────┘   │
│  [给我一个出发点]              [和 AI 核对一下]                    │
│                                                                  │
│  ───── 在你的知识库里，这个概念连接了 ─────                        │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  自我欺骗                         一起出现在 PSYCH 201    │   │
│  │  消费决策                         一起出现在 ECON 101     │   │
│  │  说服理论                         一起出现在 COMM 301     │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 五、LLM Prompt 规范

### Prompt 1：文档整合摘要（generate_concept_summary）

```
System:
You are a document synthesis engine for a student's knowledge management app.
Your task is to integrate information from multiple document excerpts about 
the same concept into a coherent summary.

Rules:
- ONLY use information from the provided excerpts. Do NOT add any external knowledge.
- Maintain the original meaning. Do NOT reinterpret or expand.
- For each claim, note which source it came from using [Source: document_name].
- Keep the summary concise (3-5 sentences).
- Write in clear, student-friendly language.

User:
Concept: {{conceptName}}

Excerpts from student's documents:
{{#each excerpts}}
[Source: {{this.assetName}} / {{this.projectName}}]
{{this.text}}
---
{{/each}}

Task: Synthesize these excerpts into a coherent 3-5 sentence summary. 
Each sentence should reference its source. Do not add any information 
not present in the excerpts above.
```

### Prompt 2：理解框架（generate_concept_explanation）

```
System:
You are a knowledge explanation engine for a student's learning app.
You help students understand concepts they've encountered in their documents.

CRITICAL RULES:
1. You MUST ONLY use information from the student's documents provided below.
2. Do NOT introduce any information not present in these documents.
3. For EVERY explanatory point, you MUST cite the source document using 
   [Source: document_name].
4. If you cannot find sufficient information in the documents to answer a 
   section, write "Not enough information in your documents for this section."
5. Do NOT fabricate examples, mechanisms, or explanations.

User:
Concept: {{conceptName}}
Existing definition: {{definition}}

Student's documents about this concept:
{{#each documentSections}}
=== {{this.projectName}} / {{this.assetName}} ===
{{this.content}}

{{/each}}

Task: Based ONLY on the documents above, generate an explanation with these sections:

1. **核心机制 (Core Mechanism)**: How does this concept work? What is its 
   underlying logic? [Source required]

2. **典型场景 (Typical Scenarios)**: In which specific contexts does this 
   concept appear in the student's documents? List 2-3 examples from the 
   documents. [Source required for each]

3. **常见误区 (Common Misconceptions)**: Based on the documents, what concepts 
   might be confused with this one? What is the distinction? [Source required]
   If no relevant comparison exists in the documents, skip this section.

4. **一句话精华 (Essence)**: A single memorable sentence that captures the 
   core of this concept based on the documents.

Return as JSON:
{
  "mechanism": {"text": "...", "source": "document_name"},
  "scenarios": [{"text": "...", "source": "document_name"}],
  "misconceptions": [{"text": "...", "source": "document_name"}],
  "essence": "..."
}
```

### Prompt 3：AI 镜子反馈（validate_user_explanation）

```
System:
You are a gentle learning companion helping a student check their understanding
of a concept. Your job is to compare their explanation against their own 
documents — NOT against any external standard.

CRITICAL RULES:
1. Compare the student's explanation ONLY against the provided documents.
2. Use encouraging, exploratory language. NEVER use words like "wrong", 
   "incorrect", "incomplete", "missing", "failed to".
3. Acknowledge what the student captured correctly first.
4. Present any uncovered points as "additional perspectives from your documents 
   that you might find interesting", not as mistakes.
5. Do NOT judge whether their explanation is "good enough" or not.

User:
Concept: {{conceptName}}

Student's explanation:
{{userExplanation}}

Key points from student's documents:
{{#each keyPoints}}
- {{this.text}} [Source: {{this.source}}]
{{/each}}

Task: Generate mirror feedback in this exact format:
{
  "covered_count": [number of key points the student's explanation touched on],
  "covered_points": ["brief description of each covered point"],
  "additional_perspectives": [
    {
      "text": "In your documents, there's also the perspective that...",
      "source": "document_name"
    }
  ],
  "difference_note": "One subtle difference between your explanation and your 
    documents is..." (only if there's a genuine factual difference; otherwise null)
}
```

---

## 六、非功能需求

### 6.1 性能

| 指标 | 目标 | 说明 |
|------|------|------|
| 深入理解页面首屏（文档整合摘要） | ≤ 500ms | 从 SQLite 缓存读取，或基于已有 excerpts 快速组装 |
| 理解框架首字延迟 | ≤ 3s | LLM 流式输出 |
| AI 镜子反馈响应 | ≤ 5s | LLM 调用，可流式 |
| 概念关系网络加载 | ≤ 100ms | 数据库查询，无 LLM |
| 概念搜索 | ≤ 100ms | 继承 v2.1 要求 |

### 6.2 数据安全

- 所有数据本地存储，不上传到任何第三方服务
- LLM 调用走用户自配的 API endpoint
- LLM 调用时发送的内容：用户自己的文档摘录 + 概念信息（无其他用户数据）
- 用户手动编辑内容（definition、user_explanation）不得被 AI 重新生成覆盖

### 6.3 离线能力

- 已生成的理解内容（concept_summaries、concept_explanations）离线可查看
- 用户个人理解笔记（concept_user_notes）离线可查看和编辑
- 新的 LLM 生成需要网络连接；离线状态下显示缓存内容，并提示「需要网络连接来生成新内容」
- 概念关系网络（基于共现关系的部分）完全离线可用

### 6.4 可维护性

- 新增的 4 张 SQLite 表在独立的 migration 文件中定义
- 所有新 LLM Prompt 集中管理（Rust 侧的 prompts 模块），不散落在组件中
- Tauri Command 命名遵循 `snake_case`，新增命令前缀统一为 `knowledge_`（例：`knowledge_generate_explanation`）
- 前端新组件放在 `KnowledgeUnderstanding/` 目录下
- Zustand Store 不引入新的跨 Store 依赖

---

## 七、技术约束

| 约束 | 来源 | 说明 |
|------|------|------|
| Tauri 2.x + React 18 | 技术栈已固定 | 不引入新框架 |
| SQLite（本地，Rust 侧写操作） | 技术栈已固定 | 前端只读取 |
| LLM API（用户自配，OpenAI 兼容） | 已有 llmProbe/llmPreview commands | 复用现有调用架构，流式输出优先 |
| TypeScript 严格类型，避免 any | 代码规范 | 新接口定义放 types/ 目录 |
| 一人代码项目 | 项目约束 | 不设计过于复杂的架构，代码清晰可读优先 |

---

## 八、分期计划

| 阶段 | 功能 | 用户可感知价值 | 预计开发量 |
|------|------|--------------|-----------|
| **P0（本次迭代）** | 功能 1-6（深入理解入口 + 文档整合摘要 + 理解框架 + 用我的话说 + 知识网络 + 数据升级） | 用户可以真正深入理解一个概念，并验证自己的理解 | ~2 Sprint |
| **P1（下次迭代）** | Socratic 自测 + 文件夹/方向级知识概览 + 语义关联 Lazy LLM | 用户可以测试自己的理解；可以从文件夹视角看知识结构 | ~2 Sprint |
| **P2（后续版本）** | Layer 3 通用背景 + 知识技能/工作流输出 | 知识超出文档边界的延伸；知识转化为可用技能 | 待定 |

---

## 九、已知风险与缓解策略

| 风险 | 严重性 | 缓解策略 | 状态 |
|------|--------|----------|------|
| LLM 理解框架超出文档范围（幻觉） | 高 | 透明度优先策略：来源链接 + 「查看依据」功能 + 透明度声明；Prompt 层约束 | 已接受，透明度方案已设计 |
| 镜子反馈措辞引起用户抵触 | 高 | 探索式措辞原则；禁止批改式语言；详细的措辞规范 | 已在 Prompt 规范中处理 |
| 共现关系大量假阳性 | 中 | UI 透明化数据来源（「一起出现在你的文档中」），不宣称深层联系 | 已在 UI 规范中处理 |
| 数据迁移破坏 v2.1 已有数据 | 高 | 增量添加策略，不修改已有表 | 已在数据模型中处理 |
| 按需 LLM 调用延迟影响体验 | 中 | 流式输出 + 占位加载态；文档整合摘要快速路径（≤500ms） | 已在性能要求中处理 |
| 用户不知道「深入理解」功能存在 | 高 | 蓝色高亮入口 + 一次性 Tooltip + 空状态引导 | 已在 Feature Discovery 中处理 |

---

## Conductor 桥接摘要

### 核心功能清单（带优先级）

| 功能 | 优先级 | 核心用户场景 | 来自 Debate 的关键约束 |
|------|--------|-------------|----------------------|
| 「深入理解」入口 + Feature Discovery | P0 | 用户发现并触发深度理解模式 | 蓝色高亮 + 一次性 Tooltip（不能是弹窗） |
| 文档整合摘要 | P0 | 首屏快速展示「我读过什么」 | 必须 ≤500ms；来自已有数据，无新 LLM 调用 |
| 理解框架生成 | P0 | AI 帮用户把文档整理成结构化解释 | 每个条目必须有来源链接 + 查看依据；透明度声明 |
| 「用我的话说」+ AI 镜子反馈 | P0 | 用户验证自己的理解 | 独立于 definition 字段；措辞探索式不批改 |
| 概念关系网络（共现版） | P0 | 系统性理解的触发点 | 只说「一起出现」不说「紧密相关」；纯数据库计算 |
| 数据模型增量升级（4张新表） | P0 | 安全基础 | 不修改 v2.1 已有表；保护 userEdited 内容 |
| Socratic 自测 | P1 | 进阶学习验证 | Prompt 质量要求高；MVP 阶段先积累经验 |
| 文件夹/方向级知识概览 | P1 | 系统性学习某个主题域 | 范围 UX 设计待研究 |
| 语义关联 Lazy LLM | P1 | 超出共现关系的深层联系 | 首次打开触发，结果缓存 |

### 不可妥协的技术底线

1. 所有用户文档内容仅在用户触发 LLM 调用时发送给 LLM API（用户自配），**不上传到任何第三方服务**
2. `concepts.definition` 中 `user_edited = true` 的内容**不得被任何新功能覆盖**
3. 新功能的 LLM 内容存储在新增的独立表中，**不写入 v2.1 已有表**
4. 理解框架的每个解释条目**必须包含来源链接**，不允许无来源的 LLM 解释存在于界面上
5. AI 镜子反馈的措辞**必须遵循探索式原则**，代码层面应有措辞校验或强制 Prompt 规范

### 已识别的高风险项

| 风险 | 来源（Debate 哪一轮） | 当前状态 | 缓解策略 |
|------|---------------------|----------|----------|
| LLM 幻觉风险（理解框架超出文档） | Round 4（L3 差距分析） | 已设计缓解方案 | 透明度优先：来源链接 + 查看依据 + 透明度声明 |
| 镜子反馈 UX（产生批改感，用户抵触） | Round 3（L2理想态）→ Round 5 | 已设计缓解方案 | 探索式措辞规范 + Prompt 硬约束 |
| 数据迁移破坏性（v2.1 数据覆盖） | Round 4（L3差距分析，Reviewer 挑战） | 已解决 | 增量添加策略，4张全新表 |
| Feature Discovery 失败（用户不知道功能） | Round 5（L4策略，Reviewer 挑战） | 已解决 | 蓝色高亮 + 一次性 Tooltip + 空状态引导 |

### MVP 边界声明

**做什么**：
- 在现有概念详情页增加「深入理解」入口
- 基于用户文档生成结构化理解框架（文档整合摘要 + 理解框架）
- 提供「用我的话说」自由输入区 + AI 镜子反馈
- 基于共现关系的概念关系网络
- 增量数据模型升级（4张新表）

**不做什么**：
- 不修改 v2.1 已有的概念列表、观点聚合、案例引用功能（保持向后兼容）
- 不自动批量预生成理解内容（按需触发，控制 LLM 成本）
- 不提供 Socratic 自测（P1，需要更多 Prompt 工程经验）
- 不提供文件夹/方向级知识概览（P1）
- 不提供通用知识背景（P2，安全边界待定）
- 不提供知识图谱可视化（Debate 共识：卡片列表比图谱更适合当前用户心智模型）
- 不追踪用户的「理解程度」或给用户打分（Debate 共识：软件提供辅助，不评判理解）

### Debate 中未达成共识的争议

无根本争议。所有 L2/L3 挑战均已达成共识。

已知的已接受局限（非争议，是主动选择）：
1. **幻觉风险无法技术性消除**：在应用层，无法用另一个 LLM 验证第一个 LLM 的输出（成本翻倍且验证方同样有幻觉风险）。已接受局限，通过透明度策略缓解。
2. **共现关系可能存在假阳性**：两个概念共现不代表深层联系。已通过 UI 措辞处理（「一起出现」而非「紧密相关」）。
