# Task 交付 — task_005

## 实现摘要
`workspace.rs` 增 `ProjectFolderRoot` newtype + `ProjectFolderScope<'a>` + `assert_scope` 谓词；不透明 root 仅由 assert_scope 产出，`join_relative` 二次防御 `..` / 绝对路径。**未改动外部命令签名**，task_006 接入。

## 修改
| 文件 | 类型 | 说明 |
|------|------|------|
| `src-tauri/src/workspace.rs` | 修改 | +120 行 类型与 6 单测 |

## 测试 / 结果
```
cargo test --lib workspace → 6 passed
cargo test --lib           → 97 passed; 0 failed
```

## 自测矩阵
| 类型 | 场景 | 状态 |
|------|------|------|
| ✅ | None / "__ROOT__" / 正常子目录 | PASS |
| ❌ | `..` 拒绝 | PASS |
| ❌ | 绝对路径拒绝 | PASS |
| ❌ | 空 project_id 拒绝 | PASS |
| ⚠️ | 软链接陷阱 | 未测；workspace_root 用 `~/Downloads/...` 已是固定层级，realistic 攻击面低 |

## 已知局限
- `assert_scope` 不查 DB 验证 project_id 是否真存在；调用方（dropzone）已在更上层校验
- 软链接绕过未测试

## Reviewer 关注
- `join_relative` 是否真的对所有写盘路径生效（task_006 接入时验证）
