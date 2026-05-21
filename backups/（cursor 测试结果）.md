# NoteCapt（NCdesktop）自动化测试执行报告

**执行日期**：2026-03-26  
**应用根目录**：`项目启动/NCdesktop/`  
**依据文档**：`（cursor 测试用例）.md`

---

## 一、执行摘要

| 指标 | 结果 |
|------|------|
| `pnpm check`（`tsc --noEmit`） | **通过**（exit 0） |
| `pnpm lint` | **通过**（0 error，10 warnings） |
| `pnpm build`（`tsc -b && vite build`） | **通过**（exit 0） |
| `pnpm test`（Vitest） | **通过**：5 个文件，**12** 条用例全部通过 |
| `cargo test`（`src-tauri`） | **通过**：**5** 条单测全部通过 |

**未在本轮执行**（文档中为手动或需完整打包 / 真机）：

- `pnpm tauri:dev`、`pnpm tauri:build`（TC-ENV-01、TC-ENV-05）
- 主窗口、悬浮窗拖放、LLM、时间轴、TF 卡、搜索与系统集成等**手动用例**（见第三节映射表「未执行」列）

---

## 二、命令输出要点

### 2.1 前端检查与构建

- **Lint**：警告集中在 `ProjectListView.tsx`（TanStack Virtual 与 React Compiler）、`ExportPanel.tsx`、`useAudioPlayer.ts`（hooks 依赖），无 error。
- **生产构建**：Vite 已成功产出 `dist/`（含 `index.html` 与分包 JS/CSS）。

### 2.2 Vitest（`pnpm test`）

- **Runner**：Vitest v4.1.1  
- **通过的单测名称**（便于对照 PR / 回归）：

| 文件 | 用例名 |
|------|--------|
| `src/lib/__tests__/test-log.test.ts` | testLog > 输出带 TEST 前缀且不抛错 |
| `src/App.test.tsx` | App Component > renders AppLayout by default |
| `src/App.test.tsx` | App Component > renders DropzoneApp when pathname is /dropzone |
| `src/App.test.tsx` | App Component > logs mount event |
| `src/components/layout/AppLayout.test.tsx` | AppLayout Component > renders TitleBar, Sidebar, and ContentArea on wide screens |
| `src/components/layout/AppLayout.test.tsx` | AppLayout Component > changes layout to two-column when screen width strictly between 700 and 1200 |
| `src/components/layout/AppLayout.test.tsx` | AppLayout Component > hides sidebar on narrow screens (single-column) |
| `src/components/features/dropzone/DropzoneApp.test.tsx` | DropzoneApp Component > renders DropzoneIdle by default |
| `src/components/features/dropzone/DropzoneApp.test.tsx` | DropzoneApp Component > renders DropzoneAttract when phase is attract |
| `src/components/features/dropzone/DropzoneApp.test.tsx` | DropzoneApp Component > handles standard drag events by preventing default |
| `src/components/features/timeline/TimelineView.test.tsx` | TimelineView Component > renders timeline components when timeline data exists |
| `src/components/features/timeline/TimelineView.test.tsx` | TimelineView Component > logs keyframe click event |

### 2.3 Rust（`cd src-tauri && cargo test`）

| 测试名 | 结果 |
|--------|------|
| `llm::classify_parse::tests::parse_plain_json` | ok |
| `llm::classify_parse::tests::parse_markdown_fence` | ok |
| `llm::classify_parse::tests::parse_extracts_from_prefix_text` | ok |
| `llm::classify_parse::tests::parse_defaults_missing_fields` | ok |
| `db::tests::open_runs_migrations` | ok |

`main.rs` 与 doc-tests：0 测试（正常）。

---

## 三、与《测试用例总表》的映射

### 3.1 环境与构建（3.1 节）

| ID | 本轮自动化结论 |
|----|----------------|
| TC-ENV-02 | **通过**（`pnpm check`） |
| TC-ENV-03 | **通过**（`pnpm lint`，含可接受 warning） |
| TC-ENV-04 | **通过**（`pnpm build`） |
| TC-ENV-01 | **未执行**（需交互启动 `pnpm tauri:dev`） |
| TC-ENV-05 | **未执行**（未跑 `pnpm tauri:build`） |

### 3.2 Rust 单元测试（3.2 节）

| ID | 本轮结论 |
|----|----------|
| TC-RUST-01～05 | **全部通过**（`cargo test` 全绿） |

### 3.3 前端单元测试（3.3 节）

| ID | 说明 |
|----|------|
| TC-WEB-01 | **通过**（含 `test-log`；另有 App / AppLayout / DropzoneApp / TimelineView 等组件级测试，共 12 条，文档可后续扩展独立 TC 编号） |

### 3.4 其余章节（主窗口、悬浮窗、LLM、资产、TF、搜索等）

均为**手动**或 E2E 范围，**本轮未执行**，无自动化通过/失败判定。

---

## 四、为使 `pnpm build` 通过所做的代码调整（与测试可重复性相关）

1. **`tsconfig.app.json`**：增加 `exclude`，将 `*.test.ts(x)` 与 `__tests__` 排除在应用 `tsc -b` 工程外，避免 Vitest/jest-dom 类型与 `erasableSyntaxOnly` 约束冲突。  
2. **`src/utils/logger.ts`**：`debug` 使用 `import.meta.env.DEV`，避免在纯前端工程中出现未声明的 `process`。  
3. **`vite.config.ts`**：`defineConfig` 改为自 `vitest/config` 导入，使 `test` 配置字段类型合法。  
4. **`DropzoneApp.test.tsx`**：`LogicalSize` 桩类改为普通字段赋值，满足 `erasableSyntaxOnly`。

---

## 五、建议的下一轮手动冒烟（文档 第四节）

1. `pnpm tauri:dev`：主窗口 + 新建项目 + 悬浮窗拖 txt +（可选）LLM  
2. 按需执行 `pnpm tauri:build` 验证安装包产出  

---

*报告结束。*
