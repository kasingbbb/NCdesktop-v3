# Session Context — 项目上下文配置

> **使用方法**：复制本文件到 `sessions/<session_name>/session_context.md`，填写所有标注为 `[必填]` 的字段。所有角色 Prompt 将从此文件读取项目特定信息。

---

## 1. 项目信息 [必填]

- **项目名称**：notecapt desktop v1.2 版本迭代
- **一句话描述**：将原来的原始转录工具升级为 ai 赋能的录音转文本工具+悬浮窗弹窗 bug 修改
- **项目类型**：MacOS Desktop App
- **复杂度等级**：M（技术不确定性中等：科大讯飞 WebAPI 集成为新外部依赖；用户可见 UI 是产品核心功能；预估 task 数 4-6）

---

## 2. 技术上下文 [必填]

- **主语言**：Rust（后端逻辑/命令层）+ TypeScript/React（前端 UI）
- **框架/运行时**：Tauri v2（Rust + React 19 + Vite 6）；状态管理 Zustand；样式 Tailwind CSS 4
- **数据库**：SQLite（rusqlite，FTS5 全文检索）
- **关键外部依赖**：
  - 当前 ASR：macOS SFSpeechRecognizer（通过 Swift C FFI 调用，见 `src-tauri/src/macos/asr_ffi.rs`）
  - 目标 ASR：科大讯飞非实时语音转写 WebAPI（`https://office-api-ist-dx.iflyaisol.com`）
  - APPID: `6b22481d` / APIKey: `05c5027bf1c45c067a7c78d7f3c11243` / APISecret: `OTNjODViOTczODdiOWYwYmZkZTRkMzVk`
  - 悬浮窗（DropzoneApp）：`@tauri-apps/api` window API，见 `src/components/features/dropzone/DropzoneApp.tsx`
- **现有代码库**：改造现有代码
- **目标部署环境**：本地（macOS DMG 分发）

---

## 3. 关键约束 [必填]

- **安全性要求**：中 — 科大讯飞 API Key/Secret 不得硬编码在前端，必须经由 Rust 命令层调用，绝不暴露给前端 JS 环境
- **性能要求**：高 — 转录速度是核心体验指标；科大讯飞 WebAPI 需在用户可感知的合理时间内完成（目标 <1 分钟/5 分钟录音）
- **用户体验要求**：高 — 转录是产品核心功能；悬浮窗关闭 bug 是严重交互障碍，必须修复
- **可维护性要求**：中 — ASR 实现应封装在独立模块（`extraction/extractors/audio_asr.rs`），便于未来再次替换
- **不可妥协的底线**：
  1. API 凭据（APPID/APIKey/APISecret）只在 Rust 后端持有，不得出现在前端代码或日志
  2. 转录功能降级时（网络不可用/API 报错）应有明确的用户反馈，不能静默失败
  3. 悬浮窗关闭功能必须在 macOS 生产环境下可靠触发

---

## 4. 质量偏好（影响 Reviewer 评分权重）

| 维度 | 权重 | 说明 |
|------|------|------|
| 功能正确性 | 35% | 转录准确性和速度是核心，bug 修复必须可靠 |
| 安全性 | 20% | API 凭据安全是硬约束 |
| 代码质量 | 15% | 遵循现有 Rust/TS 模块化风格 |
| 测试覆盖 | 10% | 至少单元测试覆盖 Rust 侧 API 调用逻辑 |
| 架构一致性 | 10% | 与现有 extraction/extractor 模式保持一致 |
| 可维护性 | 10% | 后续可轻松替换 ASR 提供方 |

> 权重总和必须为 100%。根据项目特性调整：安全敏感项目提高安全性权重，内部工具可降低 UX 权重。

---

## 5. 领域特定代码规范 [按需填写]

```
- Rust：所有 Tauri command 函数加 #[tauri::command]，错误返回 String（tauri serialize 友好）
- Rust：新 ASR 模块放在 src-tauri/src/extraction/extractors/audio_asr_iflytek.rs，与现有 audio_asr.rs 并列
- Rust：HTTP 请求使用已有 reqwest 0.12 crate（项目已引入），不新增 HTTP 库
- Rust：API 凭据通过 AppState 或函数参数传递，禁止 lazy_static / 全局变量硬编码
- TypeScript：前端调用新 ASR 走现有 src/lib/tauri-commands.ts 模式封装
- 错误处理：网络/API 错误通过 Toast 通知用户（现有 ToastContainer 组件），不 console.error 静默
- 悬浮窗 bug 修复：只改 DropzoneApp.tsx，不触碰其他 dropzone 子组件
```

---

## 6. 领域特定审查重点 [按需填写]

```
- 科大讯飞鉴权：WebAPI 使用 HMAC-SHA256 签名（date + host + path），检查签名生成是否正确
- API 凭据：确认 APPID/APIKey/APISecret 不出现在任何前端文件、日志输出、或 Tauri IPC 返回值中
- 科大讯飞非实时转写接口：确认 audio/pcm 或 audio/mpeg 格式与 API 要求一致；base64 编码正确
- 悬浮窗关闭：确认 win.close() 调用时机是否与 Tauri v2 window API 生命周期兼容
- 降级处理：当讯飞 API 不可用时，是否有明确 fallback（报错提示，不崩溃）
- 文件大小/时长限制：讯飞非实时接口对单文件大小有限制，需检查是否做了预处理或分片
```

---

## 7. 角色专业背景补充 [按需填写]

- **Proposer 应具备的专业知识**：
  - 科大讯飞非实时语音转写 WebAPI 接口规范（HMAC-SHA256 鉴权、文件上传格式）
  - Tauri v2 命令系统与 IPC 机制
  - Rust reqwest 异步 HTTP 客户端使用
  - macOS 悬浮窗（child window）生命周期与 Tauri Window API
- **Reviewer 应重点关注的风险域**：
  - API 凭据泄露（前端/日志暴露）
  - 讯飞 API 鉴权签名错误（导致静默失败）
  - 悬浮窗关闭在 macOS 系统下的兼容性问题
  - 大文件/长音频转录的内存与超时处理

---

## 8. 文件路径约定 [必填]

- **PRD 路径**：`product/prd/`
- **源码路径**：`product/src/`
- **Session 记录路径**：`sessions/`
- **进度文件**：`sessions/conductor/progress.md`
- **架构方案存放**：`sessions/conductor/tasks/task_001_architect/output.md`

---

## 9. 辩题概述（仅 M/L 复杂度） [按需填写]

- **核心辩题**：如何以最小风险将 macOS 原生 ASR 替换为科大讯飞 WebAPI，同时确保凭据安全、降级可靠、转录体验显著提升？
- **辩论偏好**：
  - 重点辩论层：问题定义 + 策略（重点关注方案落地细节）
  - 最关心的维度：性能（转录速度/准确性提升）+ 安全（凭据保护）
