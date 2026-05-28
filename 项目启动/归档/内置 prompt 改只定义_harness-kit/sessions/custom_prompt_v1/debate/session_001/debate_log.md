# Debate Log — custom_prompt_v1

> **辩题**：如何设计一套用户可自定义的 Prompt 系统，使其在 NCdesktop 内置 Prompt 之上实现个性化知识管理？
> **复杂度**：L（完整 4 层 Debate）
> **启动时间**：2026-05-15

---

## Layer 1: 问题定义（已关闭）

### Round 1
- **Proposer**：核心问题是"内置知识管理策略与用户个人心智模型的错位"，是策略个性化问题而非 Prompt 编辑器问题。提出 overlay 合并模式，四模块统一自定义。
- **Reviewer**：提出四个 L2+ 挑战——① 自定义定义模糊 ② 四模块不应同等对待 ③ 链式依赖风险 ④ 可能是内置不够好而非需要自定义。

### Round 2
- **Proposer 修正**：① 采用"参数调整 + 受限模板覆写"混合形态，排除从零编写 ② 三层分级开放（Tagging/PARA 即时开放，Extraction 谨慎开放，Aggregation 仅参数） ③ 引入接口契约层做断路器 ④ 承认"优化内置"是 P0 前置工作。
- **Reviewer**：承认 ①② 已关闭，③ 需补回退 UX，④ "参数"和"模板插槽"需具体化。

### Round 3
- **Proposer 落地**：① 回退 UX 方案（Toast + 常驻状态指示） ② Tagging 参数清单（7 个参数） ③ 界面 Wireframe（三层结构） ④ 其他模块参数思路。
- **Reviewer 最终审视**：四项挑战均已充分回应，Layer 1 可以关闭。

### Layer 1 共识
1. 核心问题：内置策略与用户个人策略的错位（策略个性化问题）
2. 系统边界：参数调整 + 受限模板覆写，排除从零编写/marketplace/模型切换
3. 四模块分三层渐进开放
4. 接口契约层 + 断路器 + 显性回退 UX
5. 界面三层结构：基础表单 → 词库管理 → 折叠式 Prompt 模板
6. 成功标准：15% MAU 使用率、手动修正率降 30%、配置完成率 70%、LLM 失败率 <2%

### 搁置项
- 参数间交互约束（→ Layer 2）
- Prompt 模板安全边界/注入防护（→ Layer 2）
- 各模块参数完整枚举（→ Layer 2）

---

## Layer 2: 理想态（已关闭）

### Round 1
- **Proposer**：完整用户旅程（结果卡片入口 → 引导式配置 → 预览确认）、SQLite 存储方案（user_prompt_config 表）、三层 overlay 合并策略、slot_schema 版本化、注入防护（白名单+长度限制）、参数交互约束（声明式规则引擎）。
- **Reviewer**：提出三个硬问题——① [L3] Slot 版本稳定性是最大炸弹 ② [L2] Prompt 注入在 slot 场景下的特殊性 ③ [L2] Token 预算零和博弈。

### Round 2
- **Proposer 回应**：① Slot schema 版本化（deprecated+successor 链，不做自动迁移，提供 diff 视图） ② 结构隔离为主（XML 包裹+指令锚定）+内容审查为辅 ③ 四级 token 预算分配（P0 系统指令 > P1 文件内容 60% > P2 用户自定义 2000 token > P3 历史对话）。预览改为差异标注模式。
- **Reviewer 最终审视**：三项挑战均已回应，注入防护残余风险为已知局限非设计缺陷。Layer 2 可以关闭。

### Layer 2 共识
1. 用户旅程：结果卡片入口 → 引导式配置 → 差异标注预览 → 保存 → 一键回退
2. 存储：SQLite user_prompt_config 表
3. 合并：三层 overlay（system_prompt 不可覆写 + builtin slot 可覆写 + user_config）
4. 版本管理：slot_schema 表，deprecated+successor，diff 视图
5. 注入防护：XML 结构隔离+指令锚定+内容审查（残余风险已知）
6. Token 预算：四级分配，文件内容保底 60%

---

## Layer 3: 差距分析（已关闭）

### Round 1
- **Proposer**：六个结构性 Gap（存储层/合并引擎/Token 预算/前端界面/注入防护/版本迁移），MVP 收窄为"参数调整+硬限制"，模板覆写推后。
- **Reviewer**：三个挑战——① [L2] Slot 化改造前置成本被低估 ② [L2] MVP 应收窄到单模块+无预览 ③ [L3] 注入防护不可推迟。

### Round 2
- **Proposer 回应**：① MVP 纯参数不涉及 slot 化（消除前置成本风险） ② 接受收窄至 Tagging 单模块 ③ 升级注入防护至三层（正则+角色隔离+长度限），实现成本 2-3 小时。
- **Reviewer 最终审视**：三项挑战均已解决。Layer 3 可以关闭。

### Layer 3 共识
1. MVP：单模块 Tagging + 参数配置 + 升级注入防护 + 无预览
2. PARA 为 MVP+（间隔一周）
3. 关键架构决策：用户配置值放 user message，永远不注入 system prompt
4. 注入防护三层：正则+角色隔离+长度限

---

## Layer 4: 策略（已关闭）

### Round 1
- **Proposer**：P0/P1/P2 三期分阶计划（3+2+4=9 人周），回溯校验映射表，裁剪原则。
- **Reviewer**：三个挑战——① [L2] 回溯校验语义滑坡（参数微调≠策略个性化） ② [L1] P0→P2 跃迁缺乏过渡 ③ [L1] 验证入口选择。

### Round 2
- **Proposer 修正**：① 接受加"补充指令"字段（200字自由文本） ② 接受 P1 做受限自由文本过渡 ③ 不接受从修正率最高模块切入，但补充全模块修正率数据采集。
- **Reviewer 最终审视**：三项挑战均已解决，数据先行策略合理。Layer 4 可以关闭。

### Layer 4 共识
1. MVP 最终版：Tagging 参数配置 + 补充指令字段 + 全模块修正率采集
2. P1：受限自由文本（500字+引导+校验+diff预览）+ 数据驱动选择下一模块
3. P2：模板覆写 + 词库 + 版本迁移 + 差异预览
4. 回溯校验通过：无核心问题被裁掉，无范围外功能被偷加

---

## Debate 总结

- **总轮次**：10 轮（L1×3 + L2×2 + L3×2 + L4×2 + 最终审视×1）
- **核心成果**：从模糊的"让用户自定义 Prompt"收敛为可执行的三期交付计划
- **关键转折点**：
  - L1R2：Reviewer 迫使 Proposer 区分"参数调整"和"从零编写"
  - L3R1：Reviewer 指出 MVP 不需要 slot 化改造（纯参数即可）
  - L4R1：Reviewer 发现 MVP 的验证断层（参数微调≠策略个性化），催生"补充指令"字段
- **PRD 已产出**：`sessions/custom_prompt_v1/prd/custom_prompt_prd_v1.md`
