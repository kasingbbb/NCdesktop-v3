# Task 产出 — task_009_dev_ui_relation_network

## 实现摘要

实现了概念关系网络卡片列表区域：

1. **RelationNetworkSection** — 挂载时加载关系数据（knowledge_get_relations），卡片列表展示共现/上游/下游关系
2. **RelationCard** — 单张关系卡片，含概念名称（可点击导航）、关系描述、类型标签
3. 点击关联概念调用 resetForConcept 切换到新概念的深入理解页面
4. co_occurrence/upstream/downstream 三种类型视觉区分（边框颜色 + 标签）

---

## 新建文件表

| 文件路径 | 行数 | 说明 |
|---|---|---|
| `src/components/KnowledgeUnderstanding/RelationNetworkSection.tsx` | ~155 行 | 关系网络区域 + 卡片 + 导航 |

## 修改文件表

| 文件路径 | 变更类型 | 说明 |
|---|---|---|
| `KnowledgeUnderstandingPage.tsx` | 修改 | 嵌入 RelationNetworkSection + handleNavigateToRelatedConcept 回调 |
| `index.ts` | 修改 | 添加 export |

---

## 自测验证矩阵

| AC | 描述 | 状态 |
|---|---|---|
| AC-1 | 挂载时调用 knowledge_get_relations，存入 store.relations | PASS |
| AC-2 | 卡片列表：标题「在你的知识库里，这个概念连接了：」，名称+描述 | PASS |
| AC-3 | 关系类型 UI 文字：co_occurrence→「一起出现在」，upstream→「前置知识」，downstream→「应用方向」 | PASS |
| AC-4 | 点击关联概念导航（resetForConcept） | PASS |
| AC-5 | 空状态文字「暂时还没发现相关概念…」 | PASS |
| AC-6 | 性能：纯 SQLite 查询，前端渲染即时 | PASS |
| AC-7 | 嵌入 KnowledgeUnderstandingPage 最底部 | PASS |
| AC-8 | upstream/downstream 与 co_occurrence 视觉区分（边框+标签颜色） | PASS |

TypeScript 编译：`tsc --noEmit` 0 errors。
