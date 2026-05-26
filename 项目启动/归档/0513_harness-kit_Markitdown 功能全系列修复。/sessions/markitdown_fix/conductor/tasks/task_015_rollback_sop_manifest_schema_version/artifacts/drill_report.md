# Rollback Drill Report — task_015 AC-5

> **状态**：PENDING-USER-MACHINE — 演练计划已就绪，实测步骤需在干净 macOS VM 上执行后填写。

---

## 演练计划

### 目标
验证从 `dist/archive/<N-1>/` 取回的历史 DMG 在干净环境中仍可：
1. 通过 Gatekeeper（`spctl --assess` PASS）；
2. 保留 notarization stapled 状态（`stapler validate` PASS）；
3. 转录至少 1 个真实样本成功（与 task_012 真实样本矩阵复用 1 个最小用例）。

### 前置条件
- 一台干净 macOS VM（基线见 `scripts/vm-base-image.md`，task_013）；
- 网络隔离开启（验证完全离线可启动）；
- `dist/archive/` 中至少存在 N-1 子目录（由 `archive-dmg.sh` 在上一次发布后产出）。

### 预期步骤

| # | 操作 | 预期结果 | 验证命令 |
|---|------|---------|---------|
| 1 | 从 `dist/archive/<N-1>/` 取回 DMG + sha256 + archive_report.txt | sha256 一致 | `shasum -a 256 -c <N-1>.sha256` |
| 2 | 在干净 VM 挂载 DMG | DMG 挂载成功，无 quarantine 报错 | `hdiutil attach <DMG>` |
| 3 | 拖入 Applications | 复制完成 | `ls /Applications/NCdesktop.app` |
| 4 | Gatekeeper 验证 | accepted source=Notarized Developer ID | `spctl --assess --type execute -vv /Applications/NCdesktop.app` |
| 5 | Stapler 验证 | The validate action worked! | `stapler validate /Applications/NCdesktop.app` |
| 6 | 冷启动应用 | UI 出现，无 manifest 自检失败横幅 | 人工目视 |
| 7 | 运行 `scripts/vm-smoke.sh`（task_013） | 7/7 格式样本 PASS | 见 vm-smoke.sh 输出 |
| 8 | 转录 1 个最小真实样本 | 输出非空 md 文件 | `run-real-sample-matrix.sh` 单 case |

### 通过判据
- 步骤 4/5 必须严格 PASS（否则 = 历史 DMG notarization 已失效，违反 input.md 技术约束）；
- 步骤 7 必须 ≥ 95% 通过率（与 task_012 门禁一致）；
- 步骤 8 至少 1 例 PASS。

### 失败回退
- 若 Gatekeeper / stapler 失败 → 标记该 N-1 不可用，回退到 N-2（archive-dmg.sh 保留双层即为此场景设计）；
- 若 2 层都失败 → 立即重新签名+公证，临时下线下载页。

---

## 实测执行

> 以下字段在干净 VM 完成演练后由 Tech Lead 填入。

- **演练日期（UTC）**：`PENDING-USER-MACHINE`
- **演练人**：`PENDING-USER-MACHINE`
- **VM 基线**：`PENDING-USER-MACHINE`（macOS 版本 / 芯片）
- **测试的 N-1 tag**：`PENDING-USER-MACHINE`
- **DMG sha256**：`PENDING-USER-MACHINE`

### 步骤实测结果

| # | 步骤 | 结果 | 实际输出 / 备注 |
|---|------|------|-----------------|
| 1 | sha256 校验 | PENDING | |
| 2 | DMG 挂载 | PENDING | |
| 3 | 复制到 Applications | PENDING | |
| 4 | spctl --assess | PENDING | |
| 5 | stapler validate | PENDING | |
| 6 | 冷启动 | PENDING | |
| 7 | vm-smoke.sh 7 格式 | PENDING | |
| 8 | 单样本转录 | PENDING | |

### 总体结论
`PENDING-USER-MACHINE`

### 4 小时窗口测量
| 阶段 | 实测耗时 | 目标 | 是否达标 |
|------|---------|------|---------|
| 镜像取回 → 签名验证 | PENDING | ≤ 1h | PENDING |
| 推广 → 用户（模拟） | PENDING | ≤ 2h | PENDING |
