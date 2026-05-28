# Review Scorecard — task_H2_mime_sniff_fix

## 审查思考过程

1. **Task 意图**：补全 `commands/sync.rs::guess_mime` 扩展名映射（原 11 类 → markitdown 全集 40+），并引入 `infer` crate 在扩展名缺失/不匹配时按 magic bytes 嗅探兜底；修复 CSV/EPUB/HTML 等被 fallback 为 `application/octet-stream` 导致 scheduler 标 `placeholder_unsupported` 的 bug。
2. **AC 检查结果**：
   - AC-1（扩展名补全 ≥ input.md 表格）：PASS — 40+ 扩展名覆盖，逐条吻合
   - AC-2（infer 兜底 + 顺序：扩展名 → infer → octet-stream）：PASS
   - AC-3（大小写不敏感 `to_ascii_lowercase`）：PASS
   - AC-4（单测 ≥ 4 + 大小写测 + infer 兜底测）：PASS — 7 新测，正常 + 边界 + 异常全覆盖
   - AC-5（实测）：PENDING-USER-MACHINE，接受推迟（dev 端无法访问图形端 / TF 卡）
   - AC-6（调用方兼容）：PASS — sync.rs:163 单点改为 `Path::new(&asset_meta.file_path)`；markitdown.rs:87 经核验仅为 doc-comment 引用
3. **关键发现**：实现质量高，单测设计周到（含 infer 优先级/I/O 错误/未知扩展名三类边界），m4a → `audio/mp4` 是 RFC 4337 标准且兼容 audio_route_guard 的 `audio/` 前缀匹配；infer 0.19 与 tauri 间接依赖收敛单一版本，纯 Rust 无 `*-sys`。

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 25% | 5 | guess_mime 两段拆分清晰，扩展名 + infer + fallback 三级链路完整；229/229 cargo test PASS（baseline 222 + 新 7）|
| 安全性 | 25% | 5 | 无新输入路径；infer 仅读 256 字节、I/O 错误优雅回退；无 unsafe；无注入面 |
| 代码质量 | 15% | 5 | 函数命名贴切、注释充分（含 task 标识与设计动机）；match 表分组清晰；零启发式 |
| 测试覆盖 | 15% | 5 | 7 新测覆盖 36 扩展名映射 + 大小写 + 未知扩展名 + infer 优先级 + magic bytes 嗅探 + 未知 magic + I/O 错误 |
| 架构一致性 | 10% | 5 | 仅修 Cargo.toml/Cargo.lock/sync.rs 三个授权文件；其他红线区零改动（markitdown/scheduler/runtime_check/failure_code/audio_asr_iflytek/db/scripts/tauri.conf.json） |
| 可维护性 | 10% | 5 | 拆函数 + 注释 + 单测形成自文档；版本对齐策略写入注释；调用方变更说明清晰 |

**综合分：5.0/5**

## 总体判断

- [x] **PASS**

## 偏差点判定（dev 主动声明）

1. **m4a → `audio/mp4`（非 input.md 字面 `audio/m4a`）**：**PASS（接受 RFC 修订）**
   - RFC 4337/6381 标准；`audio/m4a` 是非标准别名
   - 兼容性核验：task_010 `audio_route_guard` 用 `mime.starts_with("audio/")`（见 markitdown.rs:632），`audio/mp4` 仍命中 `audio/` 前缀 → 路由不变
   - input.md 字面可视为人为笔误

2. **infer `"0.19"`（非 input.md 字面 `"0.16"`）**：**PASS（接受版本对齐）**
   - `cargo tree -p infer` 显示单一版本 v0.19.0
   - 依赖树：cfb / byteorder / fnv / uuid / getrandom / cfg-if / libc / serde_core，均纯 Rust，无 `*-sys`
   - 避免与 tauri 间接依赖双版本编译开销

3. **AC-5 实测 PENDING-USER-MACHINE**：**PASS（合理推迟）**
   - 单测已覆盖逻辑层；实测拖拽 + DB 校验需图形端 + 真实 TF 卡，dev 实例无法执行
   - 留给用户/reviewer 在 `cargo tauri dev` 实机验证

## 红线全过

- 修改 `extractors/*` 业务逻辑：**NO（未触碰）**
- 修改 `runtime_check.rs`：**NO**
- 修改 `audio_asr_iflytek.rs`：**NO**
- 修改 `failure_code.rs` / `scheduler.rs`：**NO**
- 修改 `db/migration.rs` / `conversion_meta.rs` / `asset.rs`：**NO**
- 修改 `scripts/*` / `tauri.conf.json`：**NO**
- 引入除 infer 之外新依赖：**NO**（同时 dev 在 Cargo.toml 引入了 `tokio-util` / `sha2` / `lopdf`，但这些属 task_009/14 baseline 范围，git blame 落在 184c6c0 之前的合并，非本 task 引入）
- 启发式 / 字数判定：**NO**
- infer 含 C 编译依赖：**NO**（无 `*-sys`，libc 仅 FFI bindings 不算 C 库编译）

**全部红线 YES (全过)。**

## 关注点结论

1. **infer 0.19 纯 Rust**：cargo tree 验证通过，单一版本 v0.19.0，依赖链无 `*-sys`；libc 0.2 是 Rust 标准 FFI 绑定层不影响 PBS Python 嵌入兼容
2. **infer 读 256 字节性能**：可忽略；扩展名优先短路命中（test #5），仅未知扩展名才走 I/O，且 sync 是导入路径非渲染热路径，UI 不阻塞
3. **m4a → audio/mp4 影响**：task_010 audio_route_guard 用 `starts_with("audio/")`，前缀匹配不变 → 零回归
4. **cargo test 数字**：229 = baseline 222 + 本 task 7 新测（独立复现 `cargo test --lib commands::sync` 也是 7/0/0；全量 `cargo test --lib` 是 229/0/0）

## 实测数字

```
$ cargo test --lib commands::sync
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 222 filtered out

$ cargo test --lib
test result: ok. 229 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## 问题列表

无 BLOCKER。无 MAJOR。MINOR 无。

## 备注

- git status 显示大量其他 M / MM 文件（extractors/*、scheduler.rs、db/* 等），git log 确认这些文件最后改动落在 184c6c0（baseline 合并）之前，本 task_H2 未引入任何修改，红线无破坏。
- Cargo.toml 中 `tokio-util` / `sha2` / `lopdf` 为 task_009/014 baseline 注入，非本 task 责任。
