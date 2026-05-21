# Code Review — 文件转换 v1.1 补救

**日期**：2026-05-12
**审查范围**：本轮补救实际变更（T-11 / T-12 / T-13 / T-14 / T-15）；T-01~T-10 在核验阶段确认上轮已完成，本次未触及。

## 审计范围

| 文件 | 变更类型 | 说明 |
|---|---|---|
| `src-tauri/build.rs` | 改写 | 新增 macOS Swift bridge 编译与链接逻辑 |
| `src-tauri/src/lib.rs` | 修改 | 启用 `pub mod macos`（macOS-only） |
| `src-tauri/src/extraction/extractors/mod.rs` | 修改 | macOS 默认 ASR 切换至本地 `audio_asr`；非 macOS 保持 iflytek |
| `src-tauri/src/commands/dropzone.rs` | 修改 | `path_asset_meta` 增加 docx/pptx 分支 |
| `src/types/asset.ts` | 修改 | Asset 增加 `sourceAssetId?`；AssetType 增加 `docx`/`pptx` |
| `src/App.tsx` | 修改 | 新增 `notecapt/asset-converted` 监听 |
| `src/components/features/AssetListView.tsx` | 修改 | 衍生 .md 渲染「转换自 xxx」 |

## 发现

### 🔴 高风险
（无）

### 🟡 中风险

1. **`src-tauri/build.rs:46-66`** — swiftc / xcrun 不可用时 `expect()` 直接 panic
   - 影响：在没有 Xcode CLT 的 macOS 机器上 `cargo build` 报错信息已带中文提示，开发者体验可接受；但 CI 非 macOS 节点不会触发（`#[cfg(target_os = "macos")]` 已护住）。
   - 建议：当前实现可接受；若未来增加 Linux CI 跨平台编译，需评估 cross-compile 路径。

2. **`src-tauri/build.rs:74-78`** — 链接路径硬编码 `xcrun --find swiftc`
   - 影响：依赖 Xcode Command Line Tools 路径稳定，xcode-select 切换 toolchain 时构建路径会随之变更（实际行为正确）。
   - 建议：保留现状；已通过 `cargo:rerun-if-changed=build.rs` 在 build.rs 自身变化时重跑。

3. **`src/components/features/AssetListView.tsx:578` 起的 IIFE** — `sourceAssetId` 反查依赖当前 `displayAssets` 在内存中存在
   - 影响：当筛选/排序后原件不在视图集合时，会回退到默认文案「来源：1 个原件」，原件名将看不到。
   - 建议：补救方案 T-15 验收标准为「可视区分原件与衍生 MD」，当前实现满足；如需跨筛选稳定显示，下个迭代可在 store 维持全量 id→name 映射。

### 🟢 低风险

1. **`src-tauri/src/extraction/extractors/mod.rs`** — 非 macOS 平台 `audio_asr_iflytek` 仍被注册为模块；macOS 上则未被 `vec!` 引用，会产生「模块未使用」类警告（非阻断）。
   - 建议：当前保留 iflytek 模块作为回滚保险，符合补救方案对 ASR 切换策略的注释意图。

2. **`src-tauri/build.rs` FRAMEWORKS 列表** — 当前包含 `Vision/PDFKit/ImageIO` 是因为同步编译 `ocr_bridge.swift`（虽 image_ocr / pdf_scan 提取器仍禁用）。
   - 建议：保留，否则 Swift 端编译会因 import 失败而崩；若决定彻底剥离 OCR，需要同时移除 ocr_bridge.swift 编译与 ocr_ffi.rs 模块声明。

## 架构一致性

- [x] 目录结构一致（未引入计划外文件）
- [x] API 路径一致（事件名 `notecapt/asset-converted` 与后端 emit 一致）
- [x] 数据模型一致（Asset.sourceAssetId 与后端 camelCase 序列化匹配）
- [x] 无计划外依赖（Swift 静态库由 xcrun/swiftc 工具链产出，未新增 Cargo dependency）

## 自动化校验结果

| 项 | 命令 | 结果 |
|---|---|---|
| Rust 库编译 | `cargo check` | ✅ 通过（4 条 pre-existing warning） |
| Rust 测试编译 | `cargo test --lib --no-run` | ✅ 通过 |
| 提取管线测试 | `cargo test --lib extraction` | ✅ 45 passed / 0 failed |
| 全量 lib 测试 | `cargo test --lib` | ⚠️ 68 passed / 12 failed |
| TypeScript 类型检查 | `npx tsc --noEmit` | ✅ EXIT=0 |

**12 个失败测试明细**：全部属于 `db::knowledge` 与 `db::co_occurrence` 模块，错误为 `no such table: concepts`，是知识进化系统的测试 fixture 未建表问题，**与本次文件转换 v1.1 补救完全无关**。建议作为单独 issue 追踪（属另一条 work stream）。

## 小结

补救方案中 T-11 ~ T-15 五项全部实现并通过编译/类型校验/相关单元测试。T-12 在执行中触发的 Swift FFI 构建子工程（build.rs、ncdesktop_bridges 静态库、toolchain 兼容库 link path）按用户决策选项 B 完整落地，PRD §4「无网络上传」硬约束在 macOS 上现已通过本地 SFSpeechRecognizer 满足。整体代码质量合格，无阻断性问题，可进入验收。
