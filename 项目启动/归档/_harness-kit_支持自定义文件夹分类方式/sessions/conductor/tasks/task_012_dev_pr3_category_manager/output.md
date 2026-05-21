# Task 交付 — task_012

## 实现摘要
后端 `commands/categories.rs`：6 命令（list/create/rename/set_disabled/delete/add_alias）+ slug 白名单（`[a-z0-9一-龥_-]` 1-32）+ 保留字拒绝（`__uncategorized__/__archived__/other`）+ 删除前置（builtin=0 AND ref=0）+ 3 单测；前端 `CategoryManager.tsx` 平铺 CRUD + 创建 form。

## 文件
- `src-tauri/src/commands/categories.rs`（新，260 行）
- `src/components/settings/CategoryManager.tsx`（新）
- `src/lib/tauri-commands.ts`（封装）
- `src-tauri/src/commands/mod.rs` + `lib.rs`（注册）

## 测试
```
cargo test --lib categories → 3 passed
cargo test --lib            → 116 passed
npx tsc --noEmit            → 0 errors
```

## 自测
- ✅ slug 白名单（含 CJK / 长度 / 保留字）
- ✅ 删除被引用计数挡住
- ✅ 删除内置 PARA 被 is_builtin 挡住
- ⚠️ rename slug + alias 升级路径：MVP 仅改 label；slug 变更未做（v2）

**PASS** 4.7/5
