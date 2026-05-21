# Scorecard — task_004

审查前验证 ✅

## AC
1 startup.rs+AppMode ✅ / 2 三档判定 ✅ / 3 manage 注入 ✅ / 4 get_app_mode 命令 ✅ / 5 banner+uiStore ✅ / 6 ReadOnly 允许查阅 ✅（守卫只在写命令调用） / 7 单测 ✅

## 评分
- 功能 30% × 4 = 1.2（前端未端到端实测，扣 1）
- UX 25% × 4 = 1.0（banner 文案 OK，未实测）
- 安全 15% × 5 = 0.75
- 架构 10% × 5 = 0.5
- 测试 10% × 4 = 0.4
- 维护 10% × 5 = 0.5
**综合 4.35 / 5 → PASS**

## 问题
无 BLOCKER / 无 MAJOR
- MINOR：前端 banner 端到端未实测（task_017 兜底）
- MINOR：Degraded 集成测试缺

## 通行下游
PR-1 全部 DONE。task_005-012 可启动；task_013-016 已可启动（与 PR-1 并行通道）。
