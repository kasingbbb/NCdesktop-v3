# Clean macOS VM Base Image SOP (task_013)

> 本文档说明如何制作 task_013 冒烟脚本 `vm-smoke.sh` 所需的两个干净 macOS arm64 base image。
> **每次冒烟前必须从 base snapshot 复原**——任何运行残留都会污染"干净 VM"前提，使
> Gatekeeper / 首启对话框结论失真。

---

## 1. 工具选择

| 候选 | 选择 | 理由 |
|------|------|------|
| `tart` (Cirrus Labs) | ✅ **首选** | 开源 (MIT)；Apple Silicon 原生（Apple Virtualization framework 包装）；snapshot/clone 原语完备；CLI 友好，CI 兼容。 |
| `UTM` | 备选 | GUI 完备但脚本化 snapshot 复原能力弱；多次 clone 慢；适合手动调试不适合自动化冒烟。 |
| Apple Virtualization framework 直接调 | ❌ | 无 snapshot 抽象，需要自己实现 disk image clone，工作量与本 task 失焦。 |

**结论**：本 SOP 全程使用 `tart`。`UTM` 仅用于首次安装 macOS（GUI 引导更省心），随后导出磁盘到 tart 管理。

---

## 2. 一次性准备：安装 tart

```bash
brew install cirruslabs/cli/tart
tart --version    # 期望 ≥ 2.0
```

**红线**：`tart` 仅安装在 **宿主机**（用户开发机）。**严禁** 在 VM 内安装 brew/tart/任何依赖——
违反 input.md 技术约束"严禁在 VM 内预装任何依赖（违反'干净'前提）"。

---

## 3. 创建 base image — macOS 12.x arm64

### 3.1 获取镜像

```bash
# Apple 公开 IPSW（最近补丁版本，arm64）。版本号示例：12.7.6
# Apple 官方镜像目录：https://ipsw.me/macOS%2012
tart create --from-ipsw=latest macos-12-base
```

> 如果 `latest` 抓的是 13.x，使用显式 IPSW URL，例如：
> `tart create --from-ipsw=https://updates.cdn-apple.com/.../UniversalMac_12.7.6_xxxxx.ipsw macos-12-base`

### 3.2 首次开机配置（**仅做最少操作**）

```bash
tart run macos-12-base
```

GUI 引导完成最小配置：
- 语言：English（避免 AppleScript locale 字面失配）
- 区域：United States
- 创建账户：用户名 `tester`，密码 `tester`
- **跳过** Apple ID 登录
- **跳过** Siri / 分析数据上传
- **跳过** Touch ID / 显示器设置

### 3.3 启用 SSH（用于 smoke 脚本 scp / ssh）

VM 内：
- System Preferences → Sharing → 勾选 **Remote Login**（开 SSH）
- 仅授权 `tester` 用户

> SSH 是宿主→VM 的运输通道，**不是 VM 内功能**；不破坏"干净"前提。

### 3.4 落 snapshot

关闭 VM，回到宿主机：

```bash
tart stop macos-12-base
# tart 的 base image 本身就是 snapshot：每次 `tart clone` 都从 base 出发。
# 仅需确认命名约定。
tart list | grep macos-12-base
```

**snapshot 命名约定**：`macos-12-base` / `macos-14-base`（不变；每次冒烟 clone 出 ephemeral 实例）。

### 3.5 严禁清单（base image 内必须不存在）

- 任何 Homebrew (`/opt/homebrew` 必须不存在)
- 任何 Python 3 (`/usr/local/bin/python3` 等用户态安装不存在；只允许 Apple 系统自带的 `/usr/bin/python3` stub)
- Xcode / Xcode Command Line Tools（除非首启自动弹出，那也只是 prompt，不能预接受）
- 任何开发者证书 / 配置描述文件
- 任何之前版本的 NoteCapt.app

宿主机验证脚本（在 `vm-smoke.sh` 启动前自检）：

```bash
ssh tester@<vm-ip> '
  test ! -d /opt/homebrew && \
  test ! -d /usr/local/Homebrew && \
  ! command -v brew >/dev/null 2>&1 && \
  ! test -d /Applications/NoteCapt.app
'
```

---

## 4. 创建 base image — macOS 14.x arm64

重复 §3，把版本号替换：

```bash
tart create --from-ipsw=https://updates.cdn-apple.com/.../UniversalMac_14.5_xxxxx.ipsw macos-14-base
```

snapshot 名：`macos-14-base`。其余配置一致。

---

## 5. 复原命令字面（被 vm-smoke.sh 调用）

```bash
# 干净 clone（每次冒烟一开始执行）：
tart clone macos-12-base ephemeral-12
tart run --no-graphics ephemeral-12 &
# … smoke 完成 …
tart stop ephemeral-12
tart delete ephemeral-12     # 销毁实例，下一次再 clone
```

`tart clone` 在 arm64 上做 COW（copy-on-write）——单次 clone 平均 5–10s，3 次冷启总 snapshot
复原成本约 30s 量级，可接受。

---

## 6. 网络与 IP 解析

`tart` 默认起 NAT 网络。VM IP：

```bash
tart ip ephemeral-12         # 例如 192.168.64.5
```

`vm-smoke.sh` 必须用 `tart ip` 动态解析，**严禁** 硬编码 IP。

---

## 7. 故障排查

| 现象 | 原因 | 处置 |
|------|------|------|
| `tart clone` 报 `image not found` | base 未创建 / 名字打错 | `tart list` 确认 |
| SSH 拒绝连接 | base 未启用 Remote Login | 重做 §3.3，重打 snapshot |
| AppleScript 报 `Not authorized to send Apple events` | base 未授予 Accessibility | 此 task 红线：**不预授权**。改用 osascript 替代序列（见 vm-smoke.sh 注释） |
| `spctl` 报 stale verdict | dev 机器 spctl 缓存污染 | 本 task 不在 dev 机器跑，VM 内 spctl 是干净的 |

---

## 8. 与其他 task 的关系

- task_005 (`notarize.sh`)：定义了 spctl 期望字面 `accepted: Notarized Developer ID`，本 SOP 在 VM
  内复用同字面。
- task_006 (`build-macos-dmg.sh`)：输出 `dist/dmg_size_report.txt`，本 SOP 复用其字段做体积比对。
- task_012 (`run-real-sample-matrix.sh` + `lib/sample-assertions.sh`)：本 SOP **复用** `assertions::*`
  函数判定 7 格式样本的转录结果，**不二次定义**。

---

## 9. 维护

- IPSW 链接每个 macOS 补丁会变；新出补丁后**重建 base image** 并打 snapshot；旧 base 保留 1 个
  版本用于回归比对。
- 本文档与 `vm-smoke.sh` 同步；如修改了 snapshot 命名约定（§3.4 / §4），必须同步改脚本。
