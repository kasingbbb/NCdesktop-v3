# Session 启动协议（Bootstrap Protocol）

## 概述

每次启动新项目（或新 session）前，PM（人类）必须与 Conductor 共同完成本清单。本清单的产出物是一份填写完毕的 `session_context.md`，它将作为所有角色 Prompt 的**唯一领域上下文来源**。

> **核心原则**：Harness 框架与具体项目领域解耦。所有领域相关信息通过 `session_context.md` 注入，Prompt 本身不硬编码任何项目细节。

---

## 启动流程

```
Step 1: PM 填写 session_context.md（基于 session_context.template.md）
   │
Step 2: 复杂度评估 → 决定流程路径
   │
   ├─ S（简单）：跳过 Debate，Architect → Dev 循环
   ├─ M（中等）：2 层 Debate（问题定义 + 策略），标准 Conductor
   └─ L（复杂）：4 层完整 Debate，完整 Conductor + Architecture Guard
   │
Step 3: 创建 session 目录结构
   │
Step 4: 启动对应流程
```

---

## Step 1：填写 Session Context

1. 复制 `harness-kit/core/session_context.template.md` 到 `sessions/<session_name>/session_context.md`
2. 逐项填写，**不得跳过必填项**

---

## Step 2：复杂度评估

由 PM 根据以下矩阵判定复杂度等级：

| 维度 | S（简单） | M（中等） | L（复杂） |
|------|-----------|-----------|-----------|
| 预估 task 数 | 1-3 | 4-8 | >8 |
| 技术不确定性 | 低（全用熟悉技术） | 中（有 1-2 个待验证points） | 高（核心方案未定） |
| 安全敏感度 | 低（无用户数据） | 中（有用户数据但非金融级） | 高（涉及认证/支付/隐私） |
| 用户可见 UI | 无/极简 | 有但非核心 | 是产品核心 |
| 需要 PRD | 否 | 轻量 | 完整 |

**判定规则**：任何维度达到 L 级别 → 整体为 L；多数维度为 M → 整体为 M；否则为 S。

---

## Step 3：创建 Session 目录

```
sessions/<session_name>/
├── session_context.md          # Step 1 的产出
├── debate/                     # 仅 M/L 复杂度
│   └── session_001/
│       ├── debate_log.md
│       └── debate_conclusions.md
├── conductor/
│   ├── progress.md
│   └── tasks/
│       └── task_001_architect/
└── prd/                        # 仅 M/L 复杂度
    └── <project>_prd_v1.md
```

---

## Step 4：启动

### S（简单）
```
读取 session_context.md → 启动 Conductor（INIT → ARCHITECTURE → TASK_START 循环）
```

### M（中等）
```
读取 session_context.md → 启动 Debate（Layer 1 + Layer 4）→ 产出轻量 PRD → 启动 Conductor
```

### L（复杂）
```
读取 session_context.md → 启动 Debate（Layer 1-4 完整）→ 产出完整 PRD → 启动 Conductor（含 Architecture Guard）
```

---

## 质量闸门

启动 Conductor 之前，必须满足：
- [ ] `session_context.md` 所有必填字段已填写
- [ ] 复杂度等级已判定
- [ ] PRD 已产出（M/L），或已确认跳过（S）
- [ ] PM 确认可以开始编码
