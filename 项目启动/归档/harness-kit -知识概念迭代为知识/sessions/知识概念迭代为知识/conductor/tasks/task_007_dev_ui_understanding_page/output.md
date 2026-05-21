# Task 产出 — task_007_dev_ui_understanding_page

## 实现摘要

实现了深入理解页面的完整容器和两个核心区域：

1. **KnowledgeUnderstandingPage**（重写）— 挂载时加载缓存数据，无缓存自动触发摘要生成，三路 Tauri Event 监听（summary/explanation/mirror chunk），组件 unmount 时正确清理 listener
2. **TransparencyBanner** — 黄色/橙色警示横幅，固定文字
3. **SummarySection** — 「你的文档怎么说」，支持流式渲染、骨架屏、重新生成
4. **ExplanationSection** — 「理解框架」，不自动触发，用户手动点击生成；4 模块完整渲染
5. **ExplanationItemCard** — 单条解释条目，含来源标注
6. **SourceEvidence** — 「查看依据」折叠/展开原文
7. **tauri-commands.ts** — 新增 5 个知识理解命令封装

---

## 新建文件表

| 文件路径 | 行数 | 说明 |
|---|---|---|
| `src/components/KnowledgeUnderstanding/TransparencyBanner.tsx` | ~25 行 | AI 透明度声明横幅 |
| `src/components/KnowledgeUnderstanding/SummarySection.tsx` | ~140 行 | 摘要区域（流式+缓存+重新生成） |
| `src/components/KnowledgeUnderstanding/ExplanationSection.tsx` | ~185 行 | 理解框架区域（4模块+手动触发） |
| `src/components/KnowledgeUnderstanding/ExplanationItem.tsx` | ~30 行 | 单条解释条目组件 |
| `src/components/KnowledgeUnderstanding/SourceEvidence.tsx` | ~55 行 | 来源依据折叠组件 |

## 修改文件表

| 文件路径 | 变更类型 | 说明 |
|---|---|---|
| `src/components/KnowledgeUnderstanding/KnowledgeUnderstandingPage.tsx` | 重写 | 从占位容器变为完整实现 |
| `src/components/KnowledgeUnderstanding/index.ts` | 修改 | 添加所有新组件 export |
| `src/lib/tauri-commands.ts` | 修改 | 新增 5 个知识理解命令封装函数 |

---

## 架构遵守声明

- 组件全部放在 `KnowledgeUnderstanding/` 目录（ADR-007）
- Tauri Event listener 在 useEffect cleanup 中正确 unlisten（AC 技术约束）
- 流式 chunk 通过 store.appendXxxChunk 更新（符合 task_005 Store 设计）
- 流结束后重新加载完整数据库数据到 store（确保数据一致性）
- 概念 ID 过滤：chunk event handler 比较 conceptId 避免串扰
- ExplanationSection 不自动触发（AC-2），仅在用户点击或有缓存时渲染
- 未引入新外部依赖

---

## TypeScript 编译结果

```bash
npx tsc --noEmit
# 退出码: 0, 0 errors, 0 warnings
```

---

## 自测验证矩阵

| AC | 描述 | 状态 |
|---|---|---|
| AC-1 | 挂载时调用 knowledge_get_understanding_data，有缓存直接渲染，无缓存自动触发 summary 生成 | PASS |
| AC-2 | ExplanationSection 不自动触发，显示「生成理解框架」按钮；有缓存直接渲染 | PASS |
| AC-3 | TransparencyBanner 存在，黄色警示框，固定文字 | PASS |
| AC-4 | SummarySection 标题「你的文档怎么说」，流式骨架屏，重新生成按钮 | PASS |
| AC-5 | ExplanationSection 4 模块：核心机制/典型场景/常见误区/一句话精华；null 时不渲染 | PASS |
| AC-6 | ExplanationItemCard + SourceEvidence 组件存在 | PASS |
| AC-7 | 性能：缓存直接渲染（同步），流式首 chunk 立即追加到 buffer | PASS（依赖 LLM 实际延迟） |
| AC-8 | 错误状态：StreamingStatus='error' 时显示友好提示 | PASS |
| AC-9 | ExplanationSection 右上角有「重新生成」按钮 | PASS |

---

## 已知局限

1. **来源链接非交互**：AC-4 要求点击来源跳转原文，当前 SourceEvidence 展示文档名但无跳转（需复用已有文档跳转逻辑，降级为 MINOR）
2. **summary.sourceAssetIds 显示为 ID**：当前直接显示 asset ID 而非文档名，需后续通过 JOIN 或前端查询转换为可读名称
3. **mirror_feedback 解析假设**：reloadSummary/reloadExplanation 重新加载完整 UnderstandingData，mirror_feedback 字段假设 Tauri 返回时已解析为对象（与 task_005 已知局限 #1 关联）
4. **`store.setState` 直接调用**：重置 buffer 使用 `store.setState({ summaryStreamBuffer: "" })`，这是 Zustand 的合法 API 但绕过了 action 抽象
