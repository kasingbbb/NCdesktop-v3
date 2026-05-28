# Task 输入 — task_009_ux_review

## 目标

对"用户自定义 Prompt 功能"的最终 UI/UX 与文案做一次正式评审，识别影响专家用户使用流畅度与系统稳定性认知的问题，产出 fix list 供 task_007 二轮修复（或直接在本 task 给出可执行 patch 建议）。

## 前置条件

- 依赖 task：`task_007_dev_frontend_ui` 必须 DONE 且 task_008 至少 AC-2 PASS（可见可交互）
- 必须先存在的文件/接口：
  - 可运行的 NCdesktop dev 启动（`pnpm tauri dev` 可打开设置 → Prompt 自定义）

## 验收标准（Acceptance Criteria）

1. **AC-1（评审范围清单）** — 至少检查以下 6 个维度，每项产出"通过/有发现"判断 + 必要时具体描述：

   | 维度 | 评审问题 |
   |---|---|
   | 信息架构 | Tab 入口位置是否符合"高级功能"定位（不应过于醒目误导普通用户）？4 个 module 折叠子项顺序与命名是否清晰？ |
   | 文案完整性 | PRD § 3.1 的 3 行说明是否落地？"已自定义/默认"状态指示是否一目了然？保存/恢复按钮文案是否符合 NCdesktop 既有风格？ |
   | 错误提示 | 缺占位符 / 字节超限 / 保存失败 / 调用前字符数超限的提示文案是否中文化、有可操作建议？是否避免技术堆栈泄露？ |
   | 状态变化反馈 | 保存成功是否有 toast/即时反馈？恢复默认后是否有明确的 textarea 内容更新动画/瞬时高亮？ |
   | 可达性 | textarea 是否支持 Tab / Shift-Tab？折叠条是否可键盘展开？按钮 disabled 时是否有 `aria-disabled` 与 tooltip 解释？ |
   | R4 风险（PRD 4 module ↔ 后端 3 调用链）落地 | UI 是否需要在"PARA 分组"折叠条加一句说明"此 Prompt 与『文件打标签』在同一次 LLM 调用中合并使用"以避免用户疑惑？ |

2. **AC-2（输出 fix list）** — 评审产出 `sessions/custom_prompt_v1/conductor/tasks/task_009_ux_review/findings.md`，按以下格式：

   ```markdown
   # UX 评审发现 — task_009

   ## BLOCKER（必须修复）
   - [ ] 问题：...
     - 位置：PromptCustomizationPanel.tsx:Lxx
     - 建议：...

   ## MAJOR（强烈建议修复）
   - [ ] ...

   ## MINOR（可选修复）
   - [ ] ...
   ```

3. **AC-3（决议 R4 文案问题）** — 评审必须给出 R4（§Architect output.md）的明确决议：
   - 方案 A：维持"看起来 4 个独立 module"的体验抽象，不向用户透露后端合并细节
   - 方案 B：在 PARA / tagging 折叠条加 1 行说明"此 Prompt 与 X 在同一次分类调用中生效"
   - 方案 C：其他
   该决议作为关键决策记录到 `progress.md`

4. **AC-4（无代码变更）** — 本 task 不直接改代码（除非发现 BLOCKER 级别可在 < 30 行内修复的 micro fix，可直接 patch 并标注）；具体 fix 由 task_007 二轮（或独立的 task_007_round2）承接

## 技术约束

- **评审范围限于 UI/UX**：不评审后端代码质量（那是 Reviewer 责任）
- **评审依据**：PRD v1.1 § 3 + session_context.md § 4（质量偏好中"用户体验要求：高"权重最高）
- **不引入新需求**：评审不能借机要求功能扩展（如"加上 diff 预览"），那是 PRD P2 范围

## 参考文件

**必读**：
- PRD § 3.1（UI 草图）+ § 3.3（功能行为表）
- Architect output.md `§ R4`
- session_context.md `§ 4`（用户体验权重）
- task_007 input.md（理解原始 AC）

**代码参考**：
- `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src/components/settings/PromptCustomizationPanel.tsx`（task_007 产物）
- `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src/components/features/SettingsPanel.tsx`

## 预估影响范围

- **新建文件**：
  - `sessions/custom_prompt_v1/conductor/tasks/task_009_ux_review/findings.md`
- **修改文件**：原则上无；如发现 < 30 行的 micro fix 可直接 patch `PromptCustomizationPanel.tsx`，但必须在 findings.md 注明
- **预估变更**：评审文档 ~200 行；如有 micro fix < 30 行
