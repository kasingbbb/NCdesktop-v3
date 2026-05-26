# task_013 artifacts/

> 本目录归档由 `scripts/vm-smoke.sh` 实跑产出的报告与截屏。脚本编写期为占位空目录，
> 实跑由用户在本地宿主机（装有 `tart` + `sshpass`）触发，回写到此。

## 期望内容（用户实跑后）

```
artifacts/
├── README.md                              ← 本文件
├── vm_smoke_report.macos-12-base.json     ← 3 次冷启 macOS 12 聚合
├── vm_smoke_report.macos-14-base.json     ← 3 次冷启 macOS 14 聚合
├── spctl/                                 ← 每次冷启 spctl 输出
│   ├── spctl_ephemeral-macos-12-base-1-*.txt
│   ├── …
├── per-iter/                              ← 每次冷启细粒度 JSON
│   ├── iter_1_ephemeral-macos-12-base-1-*.json
│   ├── …
└── screenshots/                           ← AppleScript 截屏（首启对话框存在/不存在证据）
    ├── 12-cold1-mount.png
    ├── 12-cold1-launched.png
    └── …
```

## 实跑命令

```bash
cd NCdesktop
# 先确保 DMG + 样本就绪（task_006 + task_012 产物）
scripts/vm-smoke.sh dist/NoteCapt-arm64.dmg macos-12-base ./samples-decrypted
scripts/vm-smoke.sh dist/NoteCapt-arm64.dmg macos-14-base ./samples-decrypted
# 报告会写入 dist/vm-smoke/，手动拷贝到此目录归档：
cp dist/vm-smoke/vm_smoke_report.json artifacts/vm_smoke_report.macos-12-base.json
```

## CI 限制（AC-6）

CI runner 无 macOS arm64 nested virtualization 能力，此 task 必须在用户机器手动执行。
output.md 标 `PENDING-USER-MACHINE` 直至用户实跑回报。
