# PRD — NoteCapt Desktop v1.2

> 版本：v1.0 | 日期：2026-05-09 | 状态：已确认，进入开发

---

## 1. 项目概述

**背景**：NoteCapt Desktop 已具备录音文件自动转录功能（A-path），但当前使用 macOS 原生 SFSpeechRecognizer，普通话识别质量差、速度慢，严重影响核心使用体验。同时悬浮窗（Dropzone）存在关闭按钮无效的交互 bug。

**目标**：以科大讯飞非实时语音转写 WebAPI 替换 A-path ASR，提升转录质量与速度；修复悬浮窗关闭按钮。

**不做**：B-path（Timeline 手动转录）替换、悬浮窗其他交互优化、实时流式转录、Toast 通知。

---

## 2. 用户与核心场景

**用户**：使用 NoteCapt Desktop 录制并整理课堂/会议录音的个人用户。

**核心场景**：
1. 用户将录音文件（MP3/M4A/WAV，时长 1–1.5 小时）拖入 Dropzone
2. 系统自动入库，后台发起讯飞非实时转录任务（异步）
3. 资产卡片 extraction badge 显示「运行中」状态，用户继续其他操作
4. 转录完成后 badge 变为「完成」，用户打开文件即可查看转录结果
5. 用户需要关闭悬浮窗时，点击 X 按钮，窗口可靠消失

---

## 3. 功能需求

### F1 — 科大讯飞非实时 ASR 接入（P0）

| 子项 | 描述 |
|------|------|
| F1.1 认证 | Rust 侧实现 HMAC-SHA256 签名，拼接 Authorization header（讯飞标准鉴权格式） |
| F1.2 任务提交 | 读取原始音频文件字节，base64 编码，POST 至讯飞非实时转写接口，获取 taskId |
| F1.3 轮询 | tokio 异步轮询（建议间隔 10s，最大等待 30 分钟，超时返回错误） |
| F1.4 结果解析 | 解析讯飞 JSON 响应，提取拼接后的纯文本，写入现有 SQLite extraction 表 |
| F1.5 凭据管理 | APPID/APIKey/APISecret 仅在 Rust AppState 持有，绝不传递给前端 |
| F1.6 格式支持 | 支持 MP3、M4A、WAV、FLAC（对齐现有 `can_handle` mime 列表） |
| F1.7 降级处理 | API 失败（网络错误/鉴权错误/超时）时，badge 显示 error，错误信息写入 DB |

### F2 — 悬浮窗关闭 bug 修复（P0）

| 子项 | 描述 |
|------|------|
| F2.1 根因定位 | 检查 `tauri.conf.json` 中 dropzone window 的 `closable` 配置；检查 `void win.close()` 是否静默吞掉错误 |
| F2.2 可靠关闭 | 确保点击 X 按钮在 macOS 生产环境下窗口可靠消失；移除 `void`，改用 `.catch` 记录错误 |
| F2.3 范围限制 | 只改 `DropzoneApp.tsx` 关闭相关逻辑，不触碰其他子组件 |

---

## 4. 非功能需求

| 维度 | 要求 |
|------|------|
| 性能 | 1.5 小时 MP3（约 100MB）文件读取+上传超时设置 ≥ 120s；轮询总等待 ≤ 30 分钟 |
| 安全 | APPID/APIKey/APISecret 仅在 Rust 层持有，不出现在任何 IPC 返回值、日志、前端代码中 |
| 可维护性 | 新 ASR 实现放在独立文件 `audio_asr_iflytek.rs`，与现有 `asr_ffi.rs` 并列，通过注册机制切换 |
| 兼容性 | 不影响现有 extraction badge UI、Timeline 面板、其他 extractor |

---

## 5. 技术约束

- 框架：Tauri v2，Rust + React 19
- HTTP 客户端：reqwest 0.12（已有，不新增 HTTP 库）
- 异步运行时：tokio 1.x（已有）
- 加密：sha2 0.10（已有）；需新增 hmac + base64 crate（标准库补充）
- 存储：SQLite，写入现有 extraction_results 表
- 音频：直接读原始文件字节进行 base64 编码，无需格式转换

---

## 6. 分期计划

| 阶段 | 内容 | 状态 |
|------|------|------|
| v1.2 | F1（讯飞 ASR A-path）+ F2（悬浮窗关闭 bug） | 本次全部交付 |
| v1.3+ | B-path ASR 替换、悬浮窗交互优化 | 待定 |

---

## 7. Conductor 桥接摘要

### 核心功能清单（带优先级）

| 功能 | 优先级 | 核心用户场景 | 关键约束 |
|------|--------|-------------|----------|
| 讯飞 ASR A-path 替换 | P0 | 拖入录音→自动转录→badge 完成 | 凭据仅 Rust 持有；轮询 ≤30 分钟 |
| 悬浮窗关闭 bug 修复 | P0 | 点 X 按钮→窗口消失 | 只改 DropzoneApp.tsx |

### 不可妥协的技术底线

1. API 凭据（APPID/APIKey/APISecret）只在 Rust 层持有，不出现在任何 IPC 返回值、日志、前端文件
2. 转录失败时 badge 显示 error，不静默失败
3. 悬浮窗关闭必须在 macOS 生产环境下可靠触发

### 已识别的高风险项

| 风险 | 来源 | 状态 | 缓解策略 |
|------|------|------|----------|
| 讯飞 HMAC-SHA256 签名格式错误导致静默 401 | Round 2 Reviewer | 待处理 | Dev 严格对照讯飞官方文档实现，加单元测试 |
| 大文件 base64 内存峰值（1.5h MP3 ≈ 133MB encoded） | Round 2 Reviewer | 已评估可接受 | reqwest 设置足够超时；后续可优化为流式 |
| 悬浮窗 closable 配置问题 | Round 2 Reviewer | 待诊断 | task_003 首先检查 tauri.conf.json |

### MVP 边界声明

- **做什么**：讯飞 ASR A-path 替换（入库自动转录）；悬浮窗关闭 bug 修复
- **不做什么**：B-path（Timeline）ASR 替换（下一版本）；悬浮窗其他交互优化（用户确认 out-of-scope）；Toast 通知（用户确认不需要）；实时流式转录

### Debate 中未达成共识的争议

无。所有关键决策已明确。
