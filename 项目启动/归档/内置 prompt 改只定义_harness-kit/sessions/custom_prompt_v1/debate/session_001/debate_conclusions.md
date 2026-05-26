# Debate 结论摘要 — custom_prompt_v1

> **辩题**：NCdesktop 用户自定义 Prompt 系统设计
> **完成时间**：2026-05-15
> **Debate 状态**：四层完整 Debate 已关闭

---

## 核心结论

1. **问题本质**：内置知识管理策略与用户个人心智模型的错位（策略个性化问题）
2. **自定义形态**：参数调整为主（MVP）+ 受限模板覆写为辅（V1），排除从零编写
3. **四模块分三层渐进开放**：Tagging/PARA → Concept Extraction → Knowledge Aggregation
4. **安全架构**：角色隔离（用户配置放 user message，不注入 system prompt）
5. **数据先行**：P0 采集全模块修正率数据，P1 用数据决策

## MVP 定义

Tagging 模块参数配置（7 参数含补充指令）+ 全模块修正率采集 + 三层注入防护 + Token 预算框架

## 分期

- P0 (W1-3): MVP — Tagging 参数 + 数据采集
- P1 (W4-5): MVP+ — 受限自由文本过渡 + 数据驱动扩展
- P2 (W6-9): V1 — 模板覆写 + 词库 + 版本迁移

## PRD 路径

`sessions/custom_prompt_v1/prd/custom_prompt_prd_v1.md`
