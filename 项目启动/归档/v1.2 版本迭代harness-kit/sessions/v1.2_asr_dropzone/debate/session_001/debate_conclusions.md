# Debate 结论 — v1.2 ASR & Dropzone

> 日期：2026-05-09 | 复杂度：M | 层次：Layer 1 + Layer 4

## Layer 1 核心结论

1. A-path ASR（入库自动转录）替换为科大讯飞非实时 WebAPI
2. B-path（Timeline 面板）不在本次范围
3. 悬浮窗只修关闭 bug，其他交互 out-of-scope
4. 状态反馈：沿用现有 extraction badge，不加 Toast
5. 音频规模：1–1.5 小时，压缩格式 50–120MB，单次上传可行

## Layer 4 策略决策

- 全部 P0 功能在 v1.2 一次交付
- 轮询方案：tokio sleep(10s)，最大 30 分钟
- 凭据：Rust AppState 持有，编译期或配置文件加载
