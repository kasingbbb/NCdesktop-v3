# Debate Log — workspace_unified_md / session_001

主持人：Host
辩题：悬浮窗导入后，工作区如何呈现"唯一、MD 化、可拖拽"的资产？

---

## Layer 1 — 问题定义

### Round 1（Host 发问）

**Host 思考过程：**
1. Proposer 核心主张：尚未提出，本轮为开场提问。
2. Reviewer 核心挑战：尚未提出。
3. 交锋焦点：尚未形成。
4. 充分性判断：层级刚启动，需先把"问题"本身定义清楚，避免直接跳到方案。

**Host 判断：**
当前 Layer 1 为开场状态，需要 Proposer 首先把"问题"以及"成功定义"讲清楚再进入方案讨论。

**本轮问题给 Proposer：**

请你在不预设具体技术方案的前提下，先用 Trade-off 协议回答以下 4 个子问题，作为 Layer 1 的奠基：

1. **问题的本质**：用户报告"工作区出现两个条目"是表象。请定义"工作区资产"在产品语义上**应当是什么**？它是文件，还是"经过整理的可被 AI 消费的知识单元"？
2. **系统边界**：哪些内容属于"工作区资产" scope，哪些不属于？源文件（音频/图像原件）保留在哪？是否仍可被用户访问？
3. **关键约束**（请把 session_context.md 中的硬约束逐条映射到本辩题上）：
   - 工作区呈现唯一性
   - 元数据一致性
   - 失败可恢复
   - 拖拽外发即 MD
   - 源文件不丢
4. **成功标准**：本次迭代完成后，**可被用户和测试同时验证**的成功信号是什么？请给出 3–5 条可度量的判定条件（例：导入 5 个混合格式 → 工作区精确显示 5 条 MD 资产；对任一条目重命名后，拖拽 outbound 的文件名与列表显示一致 …）。

— 等待 Proposer 回应 —

---

### Round 1（Proposer 回应 — 摘要存档）

> 完整原文见 records/，此处保留 Host 综合裁决所需的核心要点。

**Proposer 提议**：把"工作区资产"框定为 **CKA（Consumable Knowledge Artifact）**——具备稳定逻辑身份的知识单元，物理层绑定"源载荷 + MD 消费载荷"。列表基数 = CKA 行数；rename/tag/delete 命令以 `asset_id` 为唯一目标参数；outbound payload 来自 CKA 的"当前 MD 载荷路径"。

**Trade-off Matrix（问题框定）**：A. CKA（选定） vs B. 文件中心 vs C. 通用知识单元（YAGNI）。

**Scope 划分**：CKA 元数据 + MD 消费载荷 + 转化作业状态 → in；源载荷 → 受管目录、可访问但不作为主条目；中间产物 / 孤儿 → out。

**成功标准（5 条，happy-path 为主）**：唯一性、元数据一致性、状态可见、失败降级、源不丢。

**脆弱性自述**：1-to-N 转化（音频按章节拆篇）、多模态非 MD 下游、协作同步、NSPasteboard 行为差异、历史脏数据 migration。

---

### Round 1（Reviewer 预先挑战 — 摘要存档）

**挑战 1（L3，根本）**：CKA 双层模型把现实压扁。真实世界是 **Asset（逻辑身份）/ Primary Rendition（MD 主呈现）/ Source Material（原物料）** 三层。

> Reviewer 论据：音频 ASR 错字率非零，用户**几乎一定**会回看原音频；图片 OCR 同理。源不是异常路径上的访问，是常规路径。Host 子问题 1 的"文件 vs 知识单元"二分法已暗中压扁这一层。

**挑战 2（L2，结构）**：Layer 1 子问题缺席 **时间维度** 与 **降级路径**：

- 时间：转化中态 / 已完成态 / 失败态都属于"工作区资产"，但底线 4「拖拽外发即 MD」只在已完成态下有定义；底线 1「唯一性」在转化中态也面临"显示 0/5 还是 5/5 占位"的语义未决。
- 降级：markitdown / 讯飞 ASR / 离线场景下成功标准是什么？schema 是否允许 `primary_path = source_path` 临时态？拖出去是 `.mp3` 还是 `.md.placeholder`？

**Reviewer 出层条件**：Round 1 必须覆盖以下 4 点中至少 3 点，否则 Layer 1 不可推进：
1. 显式提出三层模型（Asset / Primary Rendition / Source Material）并指明用户心智承载层。
2. 把底线 1/4 显式条件化为"已完成态强约束、其他态弱约束"。
3. 至少 2 条 sad-path 可度量成功标准。
4. 显式回答"用户在 Finder 里重命名源音频"这类**带外修改**算不算 in-scope。

---

## Host 综合裁决（Layer 1 Round 1）

**Host 思考过程：**
1. Proposer 核心主张：CKA 双层模型 + 5 条 happy-path 成功标准 + 命令以 asset_id 为唯一标识。
2. Reviewer 核心挑战：CKA 是压扁后的两层；底线必须分态条件化；缺 sad-path 度量；缺带外修改 scope 判定。
3. 交锋焦点：**定义级别分歧**——(a) 资产层级 2 vs 3；(b) 5 条底线是原子约束 vs 状态条件化约束。
4. 充分性判断：
   - 核心问题定义：❌ 部分（层级数与底线状态条件未定）
   - Reviewer 质疑正面回应：❌（预先挑战，Proposer 尚未回应）
   - 可入档结论：✅ 部分（Trade-off Matrix、Scope 初稿、5 条 happy-path 成功标准）
   - 论证追踪表无 ❓ 核心待定：❌（层级数 + 底线分态 双 ❓）

**Host 判断：Layer 1 不可出层，进入 Round 2。**

---

## 论证追踪表（Layer 1 Round 1 末状态）

| 论点 | 提出方 | 层级 | 状态 | 备注 |
|---|---|---|---|---|
| 工作区资产应以"逻辑资产 + 多载荷"建模（拒绝物理替换 b 和 UI 折叠 c） | Proposer | L1 | ✅ 已验证 | Reviewer 隐含同意，仅对层级数有异议 |
| 资产层级应为 2 层（CKA + 双载荷） | Proposer | L1 | ❓ 待定 | Reviewer 主张 3 层；Round 2 强制 Proposer 回应 |
| 资产层级应为 3 层（Asset / Primary Rendition / Source Material） | Reviewer | L1 | ❓ 待定 | Round 2 决议 |
| 5 条底线为原子约束 | （Host 列法）Proposer 默认接受 | L1 | ❌ 已推翻倾向 | Reviewer 论据强：底线 4 在转化中态确无定义 |
| 5 条底线需按 happy/中间/sad 三态条件化 | Reviewer | L1 | ⏸️ 倾向接受 | Round 2 让 Proposer 给出条件化表述 |
| 命令以 asset_id 为唯一标识、禁止接受文件路径 | Proposer | L1 | ✅ 已验证 | 无异议 |
| outbound payload 由后端命令返回 MD 路径 | Proposer | L1 | ✅ 已验证 | 已完成态下；中间/失败态待 Round 2 决议 |
| 工作区列表基数 = 逻辑资产行数 | Proposer | L1 | ✅ 已验证 | 已完成态下；中间/失败态待 Round 2 |
| Scope：源载荷在受管目录、可达、不作为主条目 | Proposer | L1 | ⏸️ 待 Reviewer 接受 | 若三层模型获采纳，源载荷需有独立"原物料视图"入口 |
| 5 条 sad-path 成功标准缺位 | Reviewer | L1 | ❓ 待定 | Round 2 要求 Proposer 补齐 2+ 条 |
| 带外修改（Finder 改源文件名）的 scope 判定 | Reviewer | L1 | ❓ 待定 | Round 2 决议 |
| Trade-off 排除"物理替换"(方案 b)与"UI 折叠"(方案 c) | Proposer | L1 | ✅ 已验证 | 与底线 5（源不丢）、底线 2（一致性）直接冲突 |
| 一对多转化（音频按章节拆 N 篇）属于本期 scope | Proposer（脆弱性声明） | L1 | ⏸️ out-of-scope（待 Round 2 确认） | 倾向移到 P2 |

---

## Round 2（Host 追问 — 给 Proposer）

**Host 思考过程：**
1. Proposer 主张 = CKA 双层；Reviewer 挑战 = 三层 + 条件化 + sad-path + 带外。
2. 交锋焦点：定义层级数 + 约束状态化。
3. 充分性：本轮目标是把 4 个 ❓ 收敛为 ✅ 或 ⏸️。
4. 决定：追问，不返回上一层。

**给 Proposer 的 4 个追问（必须逐条回应，不可绕开）：**

**Q1（层级数决议）** —— Reviewer 主张 Asset / Primary Rendition / Source Material 三层，其论据是"音频/图片场景下源回看是高频路径"。请正面表态：
- 你是否承认源载荷应当被建模为**与 MD 平级的命名物**（而非附属字段）？
- 如承认，请把"CKA"重新拆为三层并标注用户心智承载层（默认 Asset 层）；
- 如不承认，请给出反例证据，说明为何把源载荷做成 Asset 的字段不会在重命名/替换源/重新转化场景下导致信息丢失。

**Q2（底线条件化）** —— 请把 5 条底线分别按 **{完成态 / 转化中态 / 失败态 / 离线态}** 四态写出条件化版本。例如：
- 底线 1（唯一性）：完成态 = 列表 = 资产行数；转化中态 = ？（占位/不显示/进度行）；失败态 = ？；离线态 = ？
- 底线 4（拖拽即 MD）：完成态 = MD 路径；其他态 = ？（禁用拖拽 / 拖源 / 拖占位 / 拖错误说明文本）

**Q3（sad-path 度量）** —— 至少补 2 条 sad-path 可度量成功标准。建议方向：
- 离线导入 N 个音频，UI 在 X 秒内呈现 N 条"待转化"状态资产，可重命名/打标签/不可拖出；
- 转化失败资产可被重试，重试期间不产生重复条目。

**Q4（带外修改 scope）** —— 用户在 Finder 里直接重命名了源音频文件、或移动到回收站，本期是否 in-scope？请明确：
- in-scope：给出检测机制（文件监听？启动期扫描？）和处理策略；
- out-of-scope：明确写入"已知局限"，并说明用户感知后果（资产指向不存在的源 → UI 如何提示）。

— Round 2 已完成（详见下） —

---

### Round 2（Proposer 回应摘要）

- **Q1**：接受三层模型 **Asset / Primary Rendition / Source Material**，心智层 = Asset，命令唯一标识 = `asset_id`。
- **Q2 4 态矩阵**：定义 {done / converting / failed / offline}；非 done 态默认禁用 outbound 拖拽；rename/tag 任态均生效。
- **Q3 sad-path**：新增 S6（离线批量导入 3s 内 N 条占位）、S7（失败重试无重复条目）、S8（source 失联降级仍可拖 rendition）。
- **Q4 带外修改**：rendition 带外 = out-of-scope；source 带外 = in-scope（启动扫描 + 惰性校验，不用 fsnotify）。

### Round 2（Reviewer 裁定摘要）

- 承认 Round 1 出层 4 条件已达成；
- 新生 4 个挑战，其中 3 个需 minimal Round 3 表态：A（Asset:Source 1:1 与 Source id 暴露）/ B（rendition 落盘名策略）/ C（多选混合态拖拽）；
- 挑战 D（10 万级规模性能）转 ⏸️。

### Round 3（Proposer 收尾决议）

- **A**：采纳锁定 **1:1**，Source 不暴露独立 id；MVP 阶段 Source 作为 Asset 的内嵌属性。
- **B**：采纳 **`{asset_id}.{ext}`** 派生磁盘名；用户可见名只活在 DB（display_name）。
- **C**：采纳"多选含一条非 done 即整体禁用 outbound + toast 提示"。

---

## ✅ Layer 1 共识（Problem Definition）

1. **资产模型** —— 三层：**Asset（逻辑身份，承载用户心智 + rename/tag/delete）/ Primary Rendition（MD 主呈现，outbound 载荷）/ Source Material（原物料，可回看）**。MVP 阶段 Asset:Source 锁定 **1:1**，Source **不**暴露独立 id 给前端；rename/tag/delete 命令的唯一目标参数为 `asset_id`。

2. **磁盘命名** —— rendition / source 的物理文件名一律由 `asset_id` 派生（`{asset_id}.md`、`{asset_id}.{原扩展名}`）；用户可见名只活在 DB 的 `display_name`，重命名零 IO。

3. **状态机（四态）** —— `{done, converting, failed, offline}`，矩阵化条件约束如下：

| 底线 | done | converting | failed | offline |
|---|---|---|---|---|
| 唯一性 | 列表行=Asset 行 | 同左+"转化中"徽标 | 同左+"失败可重试" | 同左+"离线待转化" |
| 元数据一致性 | rename/tag 实时生效 | 实时生效，落盘时按 DB 当前名 | 同左 | 同左 |
| 失败可恢复 | N/A | 可取消 | 显式重试入口，错误可查 | 依赖恢复自动入队 |
| 拖拽即 MD | MD 路径 | **禁用** | **禁用**（默认） | **禁用** |
| 源不丢 | source 可达 | 同左 | 同左 | 同左 |

4. **拖拽 outbound 规则** —— 单选非 done 态：禁用 + hover 提示；多选含任一非 done：**整体禁用 + toast**；done 态 payload 一律来自 rendition 的 MD 路径（由后端命令派发，前端禁止自行拼路径）。

5. **带外修改 scope** —— rendition 带外 = **out-of-scope**（已知局限）；source 带外 = **in-scope**，采用启动期扫描 + 惰性校验；source 丢失时 Asset 标记 `source-missing` 子状态，rendition 不受影响仍可拖出。

6. **成功标准（8 条 = 5 happy + 3 sad）**：
   - S1 唯一性：5 混合格式 → 5 条 MD 资产
   - S2 元数据一致：rename → 列表 / outbound 落盘名 / DB display_name 三处一致
   - S3 三态可见：done / converting / failed 在 UI 自动化中可断言
   - S4 失败降级：markitdown 失败下资产仍存在且可 rename/tag
   - S5 源不丢：删除 Asset 时源 + rendition 同清，受管目录无孤儿
   - S6 离线批量：N 文件 3s 内 N 条 offline 占位资产
   - S7 失败重试无重复：连击 5 次重试，行数恒等于导入数
   - S8 source 失联：rendition 仍可拖出，"查看原文件"置灰提示

7. **scope 边界**：
   - **In**：源载荷与 MD rendition 双重落地、列表唯一性、状态机四态、outbound MD-only、source 失联降级、启动扫描。
   - **Out（本期）**：1-to-N 拆篇（音频按章节）、rendition 内容多版本、source 多版本演进、fsnotify 实时监听、rendition 带外检测、多模态非 MD outbound、协作/同步、10 万级规模性能承诺（PRD 标 ≤1 万）。

---

## 论证追踪表（Layer 1 终态）

| 论点 | 提出方 | 状态 | 备注 |
|---|---|---|---|
| 逻辑资产 + 多载荷（拒绝物理替换 / UI 折叠） | Proposer | ✅ | |
| **三层模型 Asset / Primary Rendition / Source Material** | Reviewer | ✅ | Round 2 Proposer 接受 |
| 命令以 `asset_id` 唯一标识 | Proposer | ✅ | |
| 5 条底线状态条件化（4 态矩阵） | Reviewer | ✅ | Round 2 Proposer 给出矩阵 |
| 非 done 态默认禁用 outbound | Proposer | ✅ | |
| Asset:Source MVP 锁 1:1 + Source 无前端 id | Reviewer | ✅ | Round 3 决议 |
| rendition 磁盘名由 asset_id 派生 | Reviewer | ✅ | Round 3 决议 |
| 多选混合态整体禁用 + toast | Reviewer | ✅ | Round 3 决议 |
| source 带外 in-scope（启动扫描+惰性） | Proposer | ✅ | |
| rendition 带外 out-of-scope | Proposer | ✅ | 写入已知局限 |
| 5 条 happy + 3 条 sad 成功标准 | Proposer | ✅ | |
| 1-to-N 拆篇 | Proposer 自述脆弱 | ⏸️ | out-of-scope（本期） |
| rendition 多版本 | Reviewer 隐含 | ⏸️ | out-of-scope |
| source 多版本 | Reviewer 隐含 | ⏸️ | out-of-scope |
| fsnotify 长驻监听 | Reviewer | ❌ | 已推翻（ROI 低） |
| 10 万级 asset 性能承诺 | Reviewer 挑战 D | ⏸️ | PRD 已知局限：目标≤1 万 |
| 多模态非 MD outbound | Proposer 脆弱性 | ⏸️ | 未来再议 |

## 层间过渡验证

- [x] Layer 1 无 ❓ 待定核心定义
- [x] 所有 ⏸️ 搁置已明确 out-of-scope 或写入已知局限
- [x] Layer 1 共识可作为 Layer 4 策略讨论基础
- [x] 论证追踪表已更新

— Layer 1 出层，进入 Layer 4（策略 + MVP 边界） —

---

## Layer 4 — 策略

### Round 1（Host 发问）

**Host 思考过程：**
1. Proposer 主张：尚未提出 Layer 4 策略，待发起。
2. Reviewer 挑战：尚未发声。
3. 交锋焦点：将围绕"哪些 in-scope 必须进 P0、哪些可延 P1/P2，以及回溯校验是否覆盖 Layer 1 所有核心问题"展开。
4. 充分性判断：开场。

**Host 判断**：进入 Layer 4 第一轮，要求 Proposer 一次性产出 MVP 范围 + P0/P1/P2 分期 + Scope 裁剪原则 + 回溯校验映射表。

**给 Proposer 的问题：**
基于 Layer 1 共识（三层模型 / 四态矩阵 / 8 条成功标准 / scope 边界），请给出：
1. **MVP 范围**：哪些功能模块在 P0（必须进首版），哪些 P1（次版），哪些 P2（远期/搁置）？
2. **Scope 裁剪原则**：当工期不够时，从 P0→P1 的裁剪顺序与判据？
3. **回溯校验映射表**：把每个 MVP 功能映射回 Layer 1 共识的核心问题，确保没有遗漏也没有偷塞。
4. **关键技术决策点**：3–5 个会显著影响后续 Architect 工作的硬决策（例：是否复用现有 `conversion_meta.rs` / 是否引入新的 `assets_v2` 表 / migration 兼容方式）。

---

### Round 1（Proposer 提案摘要）

- **P0**（必交付）：M1 折叠视图 / M2 四态聚合 / M3 命令 asset_id 化 / M4 outbound MD payload / M5 失败重试 / M6 删除级联 / M7 启动期 source 扫描
- **P1**：M8 取消转化 / M9 离线检测自动入队 / M10 转化日志面板 / M11 多选混合态禁用 toast
- **P2 / out**：M12–M15（拆篇 / rendition 多版本 / fsnotify / 10 万级性能 / 协作同步 / 非 MD outbound）
- **关键决策 A–E**：A 复用 `assets.source_asset_id` 不新建表 / B 状态由 `pipeline_tasks` 派生 / C 零新增 migration / D outbound = hardlink + Tauri dragDrop API / E 离线被动检测
- **裁剪原则**：保 S1/S2，可裁 S3 完整性、S6/S7、UI 聚合

### Round 1（Reviewer 裁定摘要）

- 同意决策 B/E
- 决策 A：要求二选一兜底（SQL VIEW vs 唯一 list API）
- 决策 D：三个未验证假设需收敛（跨卷 EXDEV / sanitize / 双 representation）
- 决策 C：`source_missing` 落点需明确
- P0 漏掉：原子导入事务 / 批量进度聚合
- 本轮裁定：需 Round 2

### Round 2（Proposer 收尾决议）

| 决议 | 选择 |
|---|---|
| **R-1 查询兜底** | `db/asset.rs` 唯一 list API（`list_root_assets()` / `list_assets_filtered()`），禁止其他 caller 拼 SQL |
| **R-2(a) EXDEV** | 降级到 **copy**（非 symlink） |
| **R-2(b) sanitize** | `/`、`\` → `_`；控制字符删除；emoji/CJK 保留；Windows 保留字/尾随 `.`/空格 → 追加 `_`；UTF-8 200 字节截断 + `_<asset_id前8位>` |
| **R-2(c) Pasteboard** | 同时提供 **file + text 双 representation**（NSFilenamesPboardType + NSStringPboardType） |
| **R-3 source_missing** | 写入 **P1 已知局限**，不动 schema；UI 层 try-open 失败 toast |
| **R-4(a) 原子导入事务** | **采纳为 P0**（M0：insert asset(pending) → enqueue conversion 两阶段） |
| **R-4(b) 批量进度 UI** | **不采纳为 P0，降 P1**；P0 用 scheduler 日志 + 单条 toast 验证闭环 |

---

## ✅ Layer 4 共识（策略）

- **MVP P0**：M0 原子导入事务 + M1–M7（折叠视图 / 四态聚合 / 命令 asset_id 化 / outbound MD payload / 失败重试 / 删除级联 / 启动期 source 扫描）
- **P1**：M8–M11 + 批量进度 UI + source_missing 持久化
- **P2 / out**：M12–M15（拆篇 / 多版本 / fsnotify / 10 万级 / 协作 / 非 MD）
- **零新增 migration**（V8 schema 已足够）
- **唯一查询入口**：`db/asset.rs::list_root_assets()` —— 任何工作区列表必走此 API
- **outbound 实现**：hardlink 优先、跨卷 fallback copy；sanitize 规则已固化；file+text 双 representation
- **裁剪保护线**：S1（唯一性）/ S2（元数据一致）不可裁

## 回溯校验映射表（最终）

| MVP 功能 | 对应 Layer 1 共识 | 优先级 |
|---|---|---|
| M0 原子导入事务 | 共识 1（三层模型）+ S6/S7（导入即可见） | P0 |
| M1 折叠视图（list_root_assets） | 共识 1 + S1 唯一性 | P0 |
| M2 四态聚合 | 共识 3（四态矩阵）+ S3 | P0 |
| M3 命令 asset_id 化 | 共识 1 + S2 元数据一致 | P0 |
| M4 outbound MD payload（hardlink+双 representation） | 共识 4 + 底线 4 + S2 | P0 |
| M5 失败重试入口 | 共识 3.failed + S4/S7 | P0 |
| M6 删除级联 | 底线 5 + S5 | P0 |
| M7 启动期 source 扫描 | 共识 5 + S8 | P0 |
| M8 取消转化 | 共识 3.converting | P1 |
| M9 离线自动入队 | 共识 3.offline + S6 | P1 |
| M10 转化日志面板 | 体验/可观测（新增——已论证：S4 失败可查需要） | P1 |
| M11 多选混合态禁用 + toast | Layer 1 Round 3 C | P1 |
| 批量进度聚合 UI | 体验优化 | P1 |
| source_missing 持久化 | 共识 5（升级） | P1 |
| 拆篇 / 多版本 / fsnotify / 10 万级 / 协作 / 非 MD | — | P2/out |

**反向自检**：无 P0 偏离 Layer 1 共识；M10 在表中显式标注"新增需要论证"已论证；裁剪原则与 S1/S2 保护线匹配。

— Layer 4 出层；Debate 完成；进入 PRD 撰写 —


