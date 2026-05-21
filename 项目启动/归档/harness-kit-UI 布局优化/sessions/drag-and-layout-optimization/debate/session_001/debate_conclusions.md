# Debate 结论 — 拖拽外发与中栏布局优化

> Session: drag-and-layout-optimization
> 完成时间: 2026-04-12
> 辩论轮次: 6 轮（Layer 1: 2 轮, Layer 2: 1 轮, Layer 3: 1 轮, Layer 4: 2 轮）

---

## 辩论核心收获

### 1. 发现了 2 个 P0 级阻塞性 Bug

Debate 过程中通过代码审查发现了两个**当前就存在的功能断裂**：

- **MIME 不匹配**：`useDragAssets` 与 `useKeyframeDrop` 使用不同的 MIME 类型，导致拖拽到时间轴功能**从未正常工作过**
- **框选不可用**：`useRubberBandSelect` 的 `<button>` 排除逻辑与素材卡片的 `<button>` 元素冲突，导致框选**只能在 8px 间隙触发**

这两个 Bug 在 Debate 前未被识别，是 Reviewer 通过代码事实审查发现的。

### 2. 从「理想方案」回归到「最小修复」

| 辩论前预期 | 辩论后决策 | 变化原因 |
|-----------|-----------|---------|
| 引入 tauri-plugin-drag 实现原生拖出 | 推迟到 P2，MVP 用右键 Finder | 插件 14 个月无发版 + 与 HTML5 DnD 共存问题 |
| 实现 3 种密度档位 | 砍掉密度功能 | viewMode 已覆盖需求，维护成本 > 用户价值 |
| 构建 useGestureRouter 手势仲裁层 | 推迟到 P2，用 5px 死区最简策略 | 5 个 hook 统一重构范围太大 |
| 工期 1-2 天 → 5-8 天 → 2 天 | 最终 2 天 | 裁剪高风险项后，剩余全部是前端 UI 改动 |

### 3. Reviewer 的关键贡献

| 挑战 | 层级 | 影响 |
|------|------|------|
| WKWebView 拖出是技术死胡同 | L3 | 避免了投入 4-6 天到不可行方案 |
| UI 密度与拖拽外发应解耦 | L2 | 独立 workstream，避免简单任务被拖慢 |
| 密度档位是伪需求 | L2 | 砍掉不必要的功能，减少维护成本 |
| MIME 断裂 + 框选 Bug | L3 | 发现了比新功能更紧急的存量问题 |
| 替代方案成本效益 | L2 | 右键 Finder 以 10% 成本覆盖核心场景 |

### 4. Proposer 的关键贡献

| 论点 | 价值 |
|------|------|
| 「进得来、出不去」数据孤岛定义 | 精准命名了核心问题 |
| tauri-plugin-drag 技术调研 | 证明了 P2 存在可行路径 |
| Phase 0 + Phase 1 递进策略 | 零风险保底 + 后续增强 |
| 三通道隔离架构 | 清晰的拖拽路由模型 |
| 网格缩略图 convertFileSrc 方案 | 30 分钟低成本实现 |

---

## 最终交付物

- [x] PRD: `sessions/drag-and-layout-optimization/prd/drag-and-layout-optimization-prd-v1.md`
- [x] Debate Log: `sessions/drag-and-layout-optimization/debate/session_001/debate_log.md`
- [x] Debate Conclusions: 本文件
- [x] Session Context: `sessions/drag-and-layout-optimization/session_context.md`

---

## 下一步

当 PM 确认 PRD 可以开始编码时，启动 Conductor 流程：
1. 读取 PRD 中的 Conductor 桥接摘要
2. 创建 `progress.md`
3. 按 P0 → P1 顺序执行 task
