# PRD v1 — 转录完整性 & 知识关联体验提升

## 1. 项目概述
修复 NCdesktop 当前两类用户痛点：
- **转录不完整**：部分文件类型（尤其 `.md` 源文件、不可解析格式、空抽取）不在工作区生成派生 `.md`，导致工作区视图缺失。
- **知识体验割裂**：转录完成后必须手动触发概念抽取，且每次全量重扫，用户编辑保护未文档化；视角/案例稀疏。

## 2. 用户与场景
- 单用户（PM 即作者，本地使用）
- 场景 1：拖入任意类型文件 → 期望工作区出现一一对应的 `.md`（哪怕是占位/失败说明）
- 场景 2：导入新素材后 → 概念库自动增量更新，已编辑的概念保持不变
- 场景 3：派生 `.md` 被 NotebookLM 导入时 → 内嵌的 tag front-matter 提供结构化提示

## 3. 功能需求

### P0
| ID | 功能 | 验收 |
|---|---|---|
| F-1 | `.md` 源文件复制 + 安全重命名进工作区 | 拖入乱码命名 .md，工作区出现 `<assetId>_<safeName>.md` |
| F-2 | 不可解析 / 失败 / 空抽取 → 占位 `.md` | 内含 `## 转录失败` 段，注明 reason + mime + 时间 |
| F-3 | 派生版本化 `_versions/<asset_id>/v{N}.md` | 重抽取 N 次后，工作区根目录始终是 latest，`_versions/` 含全部历史 |
| F-4 | DB 加 `derivative_version` 字段 | migration 可前进、不破坏存量数据 |
| F-5 | `propagate_tags_to_derivative` 实现 | 原文件 tag 同步到派生 asset |
| F-6 | 派生 `.md` 顶部写 YAML front-matter | `tags / source_asset_id / version / extracted_at` |
| F-7 | 转录完成事件 → 自动入抽取队列 | 拖入 → 等待 → 概念页自动出现新概念 |
| F-8 | 概念抽取按 asset_id 增量 | 第二次拖入新文件，仅对新 asset 跑 LLM |
| F-9 | `user_edited=true` 概念重扫时跳过 | 编辑后再触发抽取，定义不变 |
| F-10 | 同概念多源视角去重合并 | source_asset_ids 数组追加，不产生重复 viewpoint |
| F-11 | 后端 stub 验证报告 | 明确 `synthesize_viewpoints` / `generate_extensions` / `concept_relations` 是否真写库；缺失则补齐 |

### P1（next iteration）
- 素材更新 → summary/explanation 过期提示
- 概念列表虚拟化
- 跨 library 关联

### 不做
- 多设备同步、远程 LLM 切换、UI 改版

## 4. 非功能需求
- 性能：单文件转录后 → 概念出现 < 30s（典型 PDF）
- 数据安全：任何写操作不删除/覆盖用户编辑内容
- 可观测：所有占位 `.md` 在 UI 工作区视图标识为「⚠️ 失败/占位」

## 5. 技术约束
见 session_context.md 第 3、5、6 节。

## 6. 分期
- **P0**：本次 session 全部交付
- **P1**：下一 session
- 时间盒：无（质量优先）

---

## Conductor 桥接摘要

### 核心功能清单（带优先级）

| 功能 | 优先级 | 核心场景 | 关键约束 |
|---|---|---|---|
| F-1 .md 源进工作区 | P0 | 拖入用户自写 .md | safeName 重命名 |
| F-2 占位 .md | P0 | 拖入 zip / 损坏 / 空抽取 | 必须含 reason |
| F-3 派生版本化 | P0 | 重抽取保留历史 | `_versions/` 子目录 |
| F-4 derivative_version 字段 | P0 | DB 演进 | 向前迁移 |
| F-5 tag 元数据传播 | P0 | 打 tag 后抽取 | propagate 实现 |
| F-6 tag 内嵌 front-matter | P0 | NotebookLM 可读 | YAML 头部 |
| F-7 转录→抽取自动链路 | P0 | 端到端无需手点 | 失败重试 |
| F-8 增量抽取 | P0 | 二次导入只跑新 asset | 跳过已抽取 |
| F-9 user_edited 保护 | P0 | 编辑后重扫 | 不覆盖 |
| F-10 视角去重合并 | P0 | 同概念多源 | source_asset_ids 追加 |
| F-11 后端 stub 验证 | P0 | 视角/扩展/关系真实写库 | 缺失则补齐 |

### 不可妥协的技术底线
1. `user_edited=true` 概念绝不被自动覆盖
2. 重抽取绝不删除旧派生 `.md`
3. 工作区中每个原文件 100% 有可点击 `.md` 邻居
4. DB migration 无 destructive drop

### 已识别高风险项

| 风险 | 来源 | 状态 | 缓解 |
|---|---|---|---|
| 后端 stub 实现状态未知（F-11） | Layer 2 探索 | 待验证 | Task 1 先做实测 |
| 增量逻辑误判 → 漏跑 | Layer 2 I-01 | 待定 | 用 source_asset_id+content_hash 做指纹 |
| 用户编辑被覆盖（一旦发生不可逆） | Layer 1 底线 | 待实现 | 跳过 + 可选 diff UI |
| `_versions/` 目录无限增长 | Layer 4 | 搁置 | P1 加保留策略 |

### MVP 边界声明

**做什么**：F-1 ~ F-11 全部 P0
**不做什么**：
- 失效传播 UI 提示（推 P1，本次不影响功能正确性）
- 概念列表虚拟化（推 P1，仅性能优化）
- 跨 library 关联（推 P1，新增功能非修复）
- UI 视觉改版（out-of-scope，与本次主题无关）

### Debate 中未达成共识的争议
**无**。所有决策点已闭环。

### Architect 需做的明确选择
1. 占位 `.md` 的格式：纯 markdown 还是 YAML+正文？→ 建议 YAML front-matter + `## 失败原因` 段
2. `_versions/` 是物理目录还是 DB 虚拟视图？→ 建议物理目录（用户可直接在 Finder 看到历史）
3. 增量抽取的指纹算法：mtime 还是 content_hash？→ 建议 content_hash（防止 mtime 被工具误改）
