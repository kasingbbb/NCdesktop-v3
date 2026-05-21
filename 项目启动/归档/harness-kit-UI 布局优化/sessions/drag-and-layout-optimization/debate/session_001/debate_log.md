# Debate Log — 拖拽外发与中栏布局优化

> Session: drag-and-layout-optimization
> 开始时间: 2026-04-12

---

## Round 1 - Layer 1（问题定义）

### Host 开场

本轮辩题：**如何为 NCdesktop 的多模态素材管理增加「拖拽外发到系统和外部应用」能力，并优化三栏布局中栏的文件呈现密度。**

Host 向 Proposer 提出 5 个核心问题：核心问题定义、系统边界、关键约束、用户画像、成功定义。

### Proposer 首轮发言

- **核心问题**：NCdesktop 是「进得来、出不去」的数据孤岛
- **系统边界**：MVP 仅覆盖 Finder/原生 App，排除浏览器网页上传
- **关键约束**：不可修改原件（copy 语义）、WKWebView 限制、拖拽延迟<100ms
- **用户画像**：学生/知识工作者，习惯 macOS 原生拖拽范式
- **成功定义**：单/多选可拖到 Finder、延迟<100ms、≥50 文件不卡顿

### Reviewer 首轮挑战

- **L3**：WKWebView 拖出可能是技术死胡同（需 Spike 验证）
- **L2**：UI 密度和拖拽外发应解耦为独立 workstream
- **L2**：现有两套 MIME 技术债需处理
- **L2**：替代方案（Finder 中显示等）可能更经济
- **L1**：成功定义缺少失败场景

---

## Round 2 - Layer 1（续）

### Proposer 回应

1. **技术可行性**：发现 `tauri-plugin-drag`（CrabNebula 官方维护）可 0.5-1d 集成，绕过 Web DataTransfer 限制
2. **解耦**：完全同意 WS-A（UI 密度）/ WS-B（拖出）独立 workstream
3. **MIME 统一**：废弃 `application/x-asset-id`，统一为 `application/notecapt-assets`；外发走原生通道不冲突
4. **失败场景**：补充了文件不存在 toast、大文件无延迟（文件引用）等
5. **替代方案**：Phase 0（Finder 中显示，0.5h）+ Phase 1（原生拖出，0.5-1d）递进策略

### Reviewer 追问

1. **L2 插件风险**：14 个月无发版、5 个开放 Bug（#76 路径崩溃高风险）、macOS 26 兼容性零验证
2. **L2 手势冲突**：mousedown+mousemove 同时触发框选和拖拽，需手势仲裁器
3. **L2 内外路由**：同一 onDragStart 如何同时支持内部时间轴和外部 Finder？
4. **L2 成本合理性**："Finder 中显示"0.5d 覆盖 100%，为什么原生拖拽值得 5-8d？

---

## Layer 1 共识

### 核心问题定义
NCdesktop 的知识资产存在「进得来、出不去」的数据孤岛问题。用户完成素材整理后，无法通过拖拽将多模态文件直接送达外部应用。同时，三栏布局中栏的文件呈现密度过低，影响了「快速浏览-批量选取」的操作效率。

### 系统边界
- **In Scope**：拖出到 macOS Finder / 原生 App（如 Keynote、邮件、微信、备忘录）
- **Out of Scope**：拖出到浏览器网页上传区（如 ChatGPT）—— 由已有 LLM 桥接功能或「Finder 中显示」承接
- **独立 Workstream**：
  - WS-A：UI 密度优化（纯前端 CSS，0.5-1d）
  - WS-B：文件拖出（tauri-plugin-drag + Spike，1-2d）

### 关键约束
| 约束 | 类型 |
|------|------|
| 不可修改原始文件（copy 语义） | 🔴 硬约束 |
| 拖拽启动延迟 < 100ms | 🔴 硬约束 |
| 保留已有拖入导入功能 | 🔴 硬约束 |
| 打包体积 < 15MB | 🔴 硬约束 |
| 框选与拖拽手势不冲突 | 🔴 硬约束 |
| 已有选中集逻辑复用 | 🟡 软约束 |
| 内部 MIME 统一（技术债清理） | 🟡 软约束（但纳入范围） |

### 用户画像
大学生、研究生、教师、独立研究者。技术水平中低，习惯 macOS 原生交互范式。核心场景：课后选中关键素材 → 拖出到 Keynote/微信/邮件制作复习材料或分享。

### 成功定义
- 单选/多选素材可拖拽到 Finder 并产生正确文件引用
- 拖拽启动延迟 < 100ms
- ≥ 50 个文件批量拖出不卡顿
- 原件安全（不被移动/删除/改名）
- 内部拖拽功能不受影响
- 拖出失败有明确的用户反馈（toast 提示）
- "Finder 中显示"作为保底方案 Phase 0 优先交付

---

## 论证追踪表

| 论点 | 提出方 | 层级 | 状态 | 备注 |
|------|--------|------|------|------|
| NCdesktop 是数据孤岛 | Proposer | L1 | ✅ 已验证 | 双方共识 |
| MVP 仅覆盖 Finder/原生 App | Proposer | L1 | ✅ 已验证 | Reviewer 同意，浏览器场景由 LLM 桥接承接 |
| WKWebView Web API 拖出不可行 | Reviewer | L1 | ✅ 已验证 | Proposer 承认，改用 tauri-plugin-drag |
| tauri-plugin-drag 可行 | Proposer | L1 | ⏸️ 搁置 | 需 0.5d Spike 验证，不阻塞 L1 推进 |
| UI 密度与拖拽外发应解耦 | Reviewer | L1 | ✅ 已验证 | 双方共识：WS-A / WS-B 独立 |
| 现有 MIME 技术债需统一 | Reviewer | L1 | ✅ 已验证 | 纳入 WS-B 范围，统一为 notecapt-assets |
| Phase 0（Finder 中显示）是保底方案 | 双方 | L1 | ✅ 已验证 | 零风险止血 |
| 框选与拖拽手势冲突 | Reviewer | L1→L2 | ❓ 待定 | 核心问题，推入 Layer 2 详细讨论 |
| 内外拖拽路由机制 | Reviewer | L1→L2 | ❓ 待定 | 推入 Layer 2 讨论 |
| 原生拖拽 vs Finder 中显示的成本合理性 | Reviewer | L1 | ⏸️ 搁置 | Proposer 的 Phase 0+1 策略已化解为递进关系 |

---

## 层间过渡验证

- [x] 当前层无 ❓ 待定的**核心定义**（手势冲突和路由是设计问题，非定义问题）
- [x] 所有 ⏸️ 搁置项已标注为 "Spike 验证" 或 "推入 Layer 2"
- [x] 本层共识可被直接引用为 Layer 2 讨论基础
- [x] 论证追踪表已更新

---

## Round 3 - Layer 2（理想态）

*（进入理想态讨论）*

---
