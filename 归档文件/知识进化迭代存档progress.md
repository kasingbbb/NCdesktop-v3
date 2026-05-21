---
name: 知识进化系统迭代进度
description: NCdesktop 知识进化系统（12步开发计划）当前实现进度和未完成任务
type: project
originSessionId: 1f2b41d3-91cc-4e6d-a12d-4cf62fbe22f5
---

## 已完成步骤（Step 1-3, 5-11）

### Step 1: 知识单元数据模型 ✅
- 数据库迁移 V7（4张新表）：`knowledge_units`, `understanding_snapshots`, `asset_inferences`, `voice_memos`
- Rust CRUD：`src-tauri/src/db/knowledge_units.rs`
- Tauri 命令：`src-tauri/src/commands/knowledge_units.rs`（17个命令，ku_* 前缀）
- 已注册：`src-tauri/src/db/mod.rs`, `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs`
- TypeScript 类型：`src/types/knowledge-units.ts`
- IPC 桥接：追加到 `src/lib/tauri-commands.ts` 末尾

### Step 2: 知识合成管道 ✅
- Rust 合成命令：`src-tauri/src/commands/knowledge_synthesis.rs`
- 命令名：`synthesize_knowledge_units(library_id, force)`
- 事件名：`notecapt/knowledge-synthesis-progress`
- TypeScript IPC：`synthesizeKnowledgeUnits()` in `tauri-commands.ts`

### Step 3: 信号推断引擎 ✅
- Rust 命令：`src-tauri/src/commands/asset_inference.rs`
  - `infer_asset_context`, `infer_library_assets`, `get_asset_inference_result`
- 时间聚类（±2h）+ Jaccard 关键词相似度 + 置信度三档分类
- 事件：`notecapt/inference-low-confidence`
- 前端 Toast：`src/components/features/today/InferenceToast.tsx`（InferenceToastHost）

### Step 5: 知识库 UI 重设计 ✅
- `src/components/features/knowledge/KnowledgeLibraryView.tsx`（两列布局）
- `src/components/features/knowledge/KnowledgeUnitDetailPanel.tsx`（右侧详情面板）
- Zustand store：`src/stores/knowledgeUnitsStore.ts`
- 五级状态图标（○ ◔ ◑ ◕ ●），宪章 K3 推荐行动卡

### Step 6: 知识详情页 渐进式交互 ✅
- `src/components/KnowledgeUnit/KnowledgeUnitPage.tsx`（四步渐进式学习）
- Rust 流式命令：`src-tauri/src/commands/knowledge_unit_learning.rs`
  - `ku_generate_summary`, `ku_generate_explanation`, `ku_validate_explanation`, `ku_check_staleness`
- 事件名：`notecapt/ku-summary-chunk`, `ku-explanation-chunk`, `ku-mirror-chunk`

### Step 7: 今天视图 ✅
- `src/components/features/today/TodayView.tsx`（SM-2 日视图）
- 主卡、次要列表（最多7条）、统计行
- urgencyScore() 排序：状态权重 + 逾期天数加权

### Step 8: 知识进化追踪 ✅
- `ku_check_staleness` Rust 命令（新素材加入 → staleness 检测）
- KnowledgeUnitPage 集成 staleness 橙色警告横幅 + 一键重新生成
- 镜子核对完成后自动创建 UnderstandingSnapshot + SM-2 复习调度

### Step 9: 知识图谱可视化 ✅
- Rust 命令：`src-tauri/src/commands/knowledge_graph.rs`（`get_knowledge_graph`）
- 前端：`src/components/features/knowledge/KnowledgeGraphView.tsx`
  - 纯 Canvas + Verlet 物理仿真（无第三方图形库）
  - 拖拽 / 平移 / 缩放 / Tooltip / 课程分组晕圈 / 跨域虚线边
- 集成到 KnowledgeLibraryView：列表/图谱切换按钮

### Step 10: 技能形成系统 ✅
- DB V8 迁移：`skills` 表
- Rust 9个命令：`src-tauri/src/commands/skills.rs`
- 前端：`SkillsView.tsx`, `SkillChallengePanel.tsx`（开放式情景题，宪章 K8）
- 导航接入：Sidebar 新增「今日复习 / 知识库 / 技能」三项

### Step 11: 技能封装与 MCP 导出 ✅
- `src-tauri/src/mcp/server.rs`：纯 tokio TCP HTTP/1.1 MCP 服务器（无额外依赖）
  - JSON-RPC 2.0：`initialize`, `tools/list`, `tools/call`, `ping`
  - `tools/list` → verified 技能作为 MCP Tool
  - `tools/call` → 加载 KU 内容 + LLM 回答用户查询
- `src-tauri/src/commands/skill_mcp.rs`（5个命令）
- 前端：`SkillMcpPanel.tsx`（服务器控制 + 配置复制 + JSON 下载）

---

## 未完成 / 暂停步骤

### Step 4: 语音备注采集与分类 ⏸️（尚未开始）
**宪章工期**：4-5天，P1

**待实现内容**（对照宪章 Step 4）：
1. 应用内悬浮录音按钮（快按开始 / 松开停止）
2. Whisper 转录集成（项目已有音频能力可复用）
3. LLM 语音分类（4类：`supplementary` / `standalone` / `question` / `connection`）
4. 后处理逻辑：
   - `supplementary` → merge 进最相关 KU
   - `standalone` → 创建新 KU 草稿
   - `question` → 在对应 KU 上标记 ❓
   - `connection` → 跨域推送给关联 KU
5. 今天视图的处理结果通知

**现有基础**（Step 1 已完成的部分）：
- `voice_memos` 表已建（DB V7 迁移）
- 已注册命令：`ku_create_voice_memo`, `ku_classify_voice_memo`,
  `ku_get_unarchived_voice_memos`, `ku_get_voice_memos_for_unit`
- Rust CRUD：`src-tauri/src/db/knowledge_units.rs`（voice memo 部分）

**下次继续起点**：实现录音悬浮按钮（类似已有 Dropzone 窗口的方式），复用
`src-tauri/src/audio/` 已有音频采集能力，接入 Whisper 转录后触发分类管道。

---

### Step 12: 对外服务基础设施 ⏸️（主动跳过，待后续决策）
**宪章工期**：1-2周，P2

**待实现内容**：
- 访问控制（API Key 管理）
- 云端 API 端点（Skill Package 上传 / 分享）
- Cognitive Twin 基础版

**跳过原因**：需要外部服务器基础设施，超出桌面端范围，待产品方向确认后再做。

---

## 关键文件索引

| 文件 | 用途 |
|------|------|
| `src/types/knowledge-units.ts` | 全量 TS 类型 + SM-2 工具函数 |
| `src-tauri/src/db/knowledge_units.rs` | Rust DB CRUD（含 voice memo） |
| `src-tauri/src/commands/knowledge_units.rs` | 17个 CRUD 命令 |
| `src-tauri/src/commands/knowledge_synthesis.rs` | LLM 聚类合成 |
| `src-tauri/src/commands/knowledge_unit_learning.rs` | LLM 学习流式命令 |
| `src-tauri/src/commands/asset_inference.rs` | 信号推断引擎 |
| `src-tauri/src/commands/knowledge_graph.rs` | 知识图谱数据 |
| `src-tauri/src/commands/skills.rs` | 技能 CRUD + 验证 |
| `src-tauri/src/commands/skill_mcp.rs` | MCP 导出命令 |
| `src-tauri/src/mcp/server.rs` | MCP HTTP 服务器 |
| `src/stores/knowledgeUnitsStore.ts` | Zustand 状态管理 |
| `src/components/features/knowledge/KnowledgeLibraryView.tsx` | 知识库主视图（含图谱切换） |
| `src/components/KnowledgeUnit/KnowledgeUnitPage.tsx` | 渐进式学习页（Step 6+8） |
| `src/components/features/today/TodayView.tsx` | 今天视图（Step 7） |
| `src/components/features/skills/SkillsView.tsx` | 技能视图（Step 10） |
| `src/components/features/skills/SkillMcpPanel.tsx` | MCP 导出面板（Step 11） |
| `src/lib/tauri-commands.ts` | 全量 IPC 桥接 |

**Why:** 依据《notecapt 知识进化功能迭代宪章v1.0.md》第八部分 12步开发计划
**How to apply:** 下次继续时优先从 **Step 4（语音备注采集）** 开始；Step 12 等产品方向确认后再做。
