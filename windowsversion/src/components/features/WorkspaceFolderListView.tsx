/**
 * 工作区文件夹列表视图（task_007 T5a 骨架 + task_008 T5b inline 编辑）
 *
 * 替换原只读 `WorkspaceFolderStrip`：表格式 3 列（名称 / 项目数 / 修改时间）+
 * 36px 工具栏（新建 / 重命名 / 移到废纸篓）+ 右键菜单（root / ai_organized /
 * __ROOT__ / 空白处 4 形态）+ 键盘 handler 入口判定（Enter / ⌘⌫ 仅 root 触发）。
 *
 * T5b 范围（本次新增）：
 *  - inline 编辑状态机：mode = 'idle' | 'creating' | 'renaming'，从 uiStore 推导（互斥）
 *  - F1 幽灵新建行（不进 fs / 不发 IPC，Enter/blur 提交、Esc 取消）
 *  - F2 三入口重命名（右键 / Enter / 工具栏），同步乐观提交 + 失败回滚 + selection 冻结
 *  - F3 删除二次确认 modal（含 expected_count 文案，E_FOLDER_DIRTY 重弹）
 *  - 编辑期切走二次确认 modal「放弃新建『xxx』？」
 *
 * 拖拽 → T6；列宽拖动 / 排序 → PRD §3 P2 明示不做。
 *
 * ⚠️ 红线（ADR-007，底线 1）：
 *    所有写动作 handler 首行：if (selection.kind !== 'root') return;
 *    不依赖按钮/菜单项的 disabled 属性。
 *
 * ⚠️ ADR-010 边界：本期 `aggregateRow` 使用「前缀匹配（folder.relativePath + "/" + ...
 *    或 等于）」与后端 `LIKE :prefix || '/%' ESCAPE '\'` 等价；防 `100 / 100%` 前缀冲突。
 *    `__ROOT__` 仍按「不含 /」聚合。
 */
import { useCallback, useMemo, useRef, useState } from "react";
import type { Asset, WorkspaceFolderEntry } from "../../types";
import { useUIStore } from "../../stores/uiStore";
import {
  FolderListRow,
  type FolderListRowItem,
  type RowKind,
} from "./WorkspaceFolderListView/FolderListRow";
import { FolderListToolbar } from "./WorkspaceFolderListView/FolderListToolbar";
import {
  FolderContextMenu,
  type ContextMenuTarget,
} from "./WorkspaceFolderListView/FolderContextMenu";
import { InlineNameEditor } from "./WorkspaceFolderListView/InlineNameEditor";
import { DeleteConfirmModal } from "./WorkspaceFolderListView/DeleteConfirmModal";
import {
  createWorkspaceFolder,
  renameWorkspaceFolder,
  deleteWorkspaceFolder,
} from "../../lib/tauri-commands";
import { validateFolderNameSync } from "../../lib/folder-name-validate";
import { renderIpcError, isIpcError } from "../../lib/ipc-errors";

interface WorkspaceFolderListViewProps {
  projectId: string;
  /** 后端 `list_project_workspace_folders` 结果（含 `__ROOT__` 哨兵行） */
  folders: WorkspaceFolderEntry[];
  /** 项目内全量素材（用于前端聚合 count + 修改时间，ADR-010） */
  assets: Asset[];
  workspaceRoot: string;
  /** 在文件资源管理器中显示（reveal 失败由调用方处理；本期 reveal 成功路径） */
  onReveal: (relativePath: string) => void;
  /** 列表刷新（重命名 / 删除完成后调用方触发） */
  onRefresh: () => void;
}

/** 默认新建文件夹名（PRD §3 F1） */
const DEFAULT_NEW_NAME = "未命名文件夹";

/**
 * 把 filePath 归一为 workspace 相对正斜杠路径。
 * - 兼容 filePath 为绝对路径（剥 workspaceRoot 前缀）或已是相对路径两种情况。
 * - 返回不带前导 `/` 的相对路径；若 filePath 恰好等于 workspaceRoot 返回空串。
 */
export function relativeToWorkspace(
  filePath: string,
  workspaceRoot: string,
): string {
  const fp = filePath.replace(/\\/g, "/");
  const root = workspaceRoot.replace(/\\/g, "/").replace(/\/$/, "");
  let rel = fp;
  if (root && fp.startsWith(root + "/")) {
    rel = fp.slice(root.length + 1);
  } else if (root && fp === root) {
    return "";
  }
  return rel.replace(/^\/+/, "");
}

/**
 * 与后端 `count_folder_assets` 的 `LIKE :prefix || '/%' ESCAPE '\'` 等价的前端匹配：
 *  - `__ROOT__` 行 = workspace 相对路径不含 `/`（含空串排除）
 *  - 其余行 = `rel === folder.relativePath` 或 `rel.startsWith(folder.relativePath + "/")`
 *
 * 加 "/" 尾巴可防 `100` vs `100%` 前缀冲突（与 ADR-006 后端 SQL 一致）。
 */
export function matchesFolder(
  filePath: string,
  workspaceRoot: string,
  folder: WorkspaceFolderEntry,
): boolean {
  const rel = relativeToWorkspace(filePath, workspaceRoot);
  if (!rel) return false;
  if (folder.relativePath === "__ROOT__") {
    return !rel.includes("/");
  }
  return rel === folder.relativePath || rel.startsWith(folder.relativePath + "/");
}

/**
 * 聚合某行的 count 与 modifiedAt（ADR-010 前端聚合 O(N) 一次）。
 */
function aggregateRow(
  folder: WorkspaceFolderEntry,
  assets: Asset[],
  workspaceRoot: string,
): { count: number; modifiedAt: string | null } {
  let count = 0;
  let latest = "";
  for (const a of assets) {
    if (!matchesFolder(a.filePath, workspaceRoot, folder)) continue;
    count += 1;
    if (a.importedAt && a.importedAt > latest) latest = a.importedAt;
  }
  return { count, modifiedAt: latest || null };
}

function rowKindOf(f: WorkspaceFolderEntry): RowKind {
  if (f.relativePath === "__ROOT__") return "__ROOT__";
  if (f.kind === "ai_organized") return "ai_organized";
  if (f.kind === "root_import") return "root_import";
  return "root";
}

export function WorkspaceFolderListView({
  projectId,
  folders,
  assets,
  workspaceRoot,
  onReveal,
  onRefresh,
}: WorkspaceFolderListViewProps) {
  const selectedRel = useUIStore((s) => s.workspaceFolderRelativePath);
  const setSelectedRel = useUIStore((s) => s.setWorkspaceFolderRelativePath);
  // T5b uiStore 状态
  const editingFolderPath = useUIStore((s) => s.editingFolderPath);
  const pendingNewFolder = useUIStore((s) => s.pendingNewFolder);
  const pendingRenameIds = useUIStore((s) => s.pendingRenameIds);
  const startCreating = useUIStore((s) => s.startCreating);
  const cancelCreating = useUIStore((s) => s.cancelCreating);
  const startRenaming = useUIStore((s) => s.startRenaming);
  const finishRename = useUIStore((s) => s.finishRename);
  const addNotification = useUIStore((s) => s.addNotification);

  // 推导编辑状态机：creating / renaming / idle（互斥）
  const mode: "idle" | "creating" | "renaming" = pendingNewFolder
    ? "creating"
    : editingFolderPath !== null
      ? "renaming"
      : "idle";
  const isEditing = mode !== "idle";

  const [ctxMenu, setCtxMenu] = useState<
    | null
    | { x: number; y: number; target: ContextMenuTarget }
  >(null);

  // ── inline 编辑器本地受控值 ────────────────────────────────────────
  // creating 模式：输入框默认值「未命名文件夹」
  // renaming 模式：输入框默认值 = 当前 displayLabel
  const [editingValue, setEditingValue] = useState<string>("");
  const [editingError, setEditingError] = useState<string | null>(null);
  // submitting：IPC 进行中（阻挡 blur 二次提交）
  const submittingRef = useRef<boolean>(false);
  // 切走二次确认 modal：用户在编辑期点击其他行 / blur 时挂起的 action
  const [pendingDiscard, setPendingDiscard] = useState<null | {
    label: string;
    onConfirm: () => void;
  }>(null);

  // ── 删除二次确认 modal 状态 ────────────────────────────────────────
  const [deleteModal, setDeleteModal] = useState<null | {
    relativePath: string;
    displayName: string;
    expectedCount: number;
    dirtyPrev: number | null;
    busy: boolean;
  }>(null);

  const listRef = useRef<HTMLDivElement>(null);

  // 派生行数据
  const items: FolderListRowItem[] = useMemo(() => {
    return folders.map((f) => {
      const { count, modifiedAt } = aggregateRow(f, assets, workspaceRoot);
      return {
        relativePath: f.relativePath,
        displayLabel: f.displayLabel,
        kind: rowKindOf(f),
        count,
        modifiedAt,
      };
    });
  }, [folders, assets, workspaceRoot]);

  // 幽灵新建行：creating 模式下追加在列表末尾
  const itemsWithGhost: Array<FolderListRowItem & { ghost?: boolean }> = useMemo(() => {
    if (mode !== "creating") return items;
    return [
      ...items,
      {
        relativePath: "__GHOST_NEW__",
        displayLabel: editingValue || DEFAULT_NEW_NAME,
        kind: "root" as const,
        count: 0,
        modifiedAt: null,
        ghost: true,
      },
    ];
  }, [items, mode, editingValue]);

  // 当前选中行
  const selectedItem = useMemo(
    () => items.find((it) => it.relativePath === selectedRel) ?? null,
    [items, selectedRel],
  );
  const selectionKind: RowKind | null = selectedItem?.kind ?? null;

  // ── 编辑器同步校验 ────────────────────────────────────────────────
  const updateEditingValue = useCallback((next: string) => {
    setEditingValue(next);
    // 即时校验：失败把 reason 转中文（与 ipc-errors errorMessages 解耦，
    // 这里仅作行内 hint）
    const v = validateFolderNameSync(next);
    if (v.ok) {
      setEditingError(null);
    } else {
      const map: Record<string, string> = {
        has_slash: "名称不能包含 / \\ :",
        leading_dot: "名称不能以 . 开头",
        blank: "名称不能为空",
        too_long: "名称过长（超过 255 字节）",
        reserved: "「organized」是保留字",
      };
      setEditingError(map[v.reason] ?? "名称不合法");
    }
  }, []);

  // ── F1 提交新建 ────────────────────────────────────────────────────
  const submitCreate = useCallback(async () => {
    if (submittingRef.current) return;
    const name = editingValue.trim();
    const v = validateFolderNameSync(name);
    if (!v.ok) {
      // 保留编辑态 + 红框（updateEditingValue 已设 editingError）
      updateEditingValue(editingValue); // 触发 reason 同步
      return;
    }
    submittingRef.current = true;
    try {
      await createWorkspaceFolder(projectId, name);
      submittingRef.current = false;
      cancelCreating();
      setEditingValue("");
      setEditingError(null);
      onRefresh();
      // 新行选中：后端返回 entry.relativePath 即新行 rel；这里乐观以 name 作为
      // top-level relativePath（MVP 仅支持根级单层）
      setSelectedRel(name);
    } catch (e) {
      submittingRef.current = false;
      const msg = isIpcError(e) ? renderIpcError(e) : "创建失败";
      setEditingError(msg);
      addNotification({ type: "error", title: "新建文件夹失败", message: msg, duration: 4000 });
      // 保留编辑态 + 红框；不 cancelCreating
    }
  }, [editingValue, projectId, cancelCreating, onRefresh, setSelectedRel, updateEditingValue, addNotification]);

  const cancelCreate = useCallback(() => {
    submittingRef.current = false;
    setEditingValue("");
    setEditingError(null);
    cancelCreating();
  }, [cancelCreating]);

  // ── F2 提交重命名 ──────────────────────────────────────────────────
  const submitRename = useCallback(async () => {
    if (submittingRef.current) return;
    const oldRel = editingFolderPath;
    if (!oldRel) return;
    const newName = editingValue.trim();
    const oldName = items.find((it) => it.relativePath === oldRel)?.displayLabel ?? oldRel;
    if (newName === oldName) {
      // 无变化 → 直接退出
      finishRename(oldRel);
      setEditingValue("");
      setEditingError(null);
      return;
    }
    const v = validateFolderNameSync(newName);
    if (!v.ok) {
      updateEditingValue(editingValue);
      return;
    }
    submittingRef.current = true;
    try {
      const entry = await renameWorkspaceFolder(projectId, oldRel, newName);
      submittingRef.current = false;
      finishRename(oldRel);
      setEditingValue("");
      setEditingError(null);
      onRefresh();
      // 保持选中：把 selection 切到新 rel
      setSelectedRel(entry.relativePath);
    } catch (e) {
      submittingRef.current = false;
      const msg = isIpcError(e) ? renderIpcError(e) : "重命名失败";
      // 回滚名称：finishRename → 退出编辑态，UI 名称回原值
      finishRename(oldRel);
      setEditingValue("");
      setEditingError(null);
      // selection 回到该节点
      setSelectedRel(oldRel);
      addNotification({ type: "error", title: "重命名失败", message: msg, duration: 4000 });
    }
  }, [
    editingFolderPath,
    editingValue,
    items,
    projectId,
    finishRename,
    onRefresh,
    setSelectedRel,
    updateEditingValue,
    addNotification,
  ]);

  const cancelRename = useCallback(() => {
    submittingRef.current = false;
    const oldRel = editingFolderPath;
    if (oldRel) finishRename(oldRel);
    setEditingValue("");
    setEditingError(null);
  }, [editingFolderPath, finishRename]);

  // ── 写动作 handler（共享给工具栏 / 右键菜单 / 键盘）─────────────────
  // 入口判定不依赖 UI disabled（ADR-007 / 底线 1）。
  const handleRename = useCallback(() => {
    if (selectionKind !== "root") return; // ADR-007 入口判定
    if (!selectedRel) return;
    if (mode !== "idle") return; // 已在编辑 → 忽略
    const cur = items.find((it) => it.relativePath === selectedRel);
    if (!cur) return;
    setEditingValue(cur.displayLabel);
    setEditingError(null);
    startRenaming(selectedRel);
  }, [selectionKind, selectedRel, mode, items, startRenaming]);

  const handleDelete = useCallback(() => {
    if (selectionKind !== "root") return; // ADR-007 入口判定
    if (!selectedRel) return;
    if (mode !== "idle") return;
    const cur = items.find((it) => it.relativePath === selectedRel);
    if (!cur) return;
    setDeleteModal({
      relativePath: selectedRel,
      displayName: cur.displayLabel,
      expectedCount: cur.count, // ADR-010：前端聚合提供
      dirtyPrev: null,
      busy: false,
    });
  }, [selectionKind, selectedRel, mode, items]);

  const handleCreate = useCallback(() => {
    // 新建无需 selection 判定：永激活
    if (mode !== "idle") return;
    setEditingValue(DEFAULT_NEW_NAME);
    setEditingError(null);
    startCreating();
  }, [mode, startCreating]);

  // ── 删除 modal: 确认提交 ───────────────────────────────────────────
  const submitDelete = useCallback(async () => {
    if (!deleteModal) return;
    if (deleteModal.busy) return;
    setDeleteModal((s) => (s ? { ...s, busy: true } : s));
    const { relativePath, expectedCount } = deleteModal;
    try {
      await deleteWorkspaceFolder(
        projectId,
        relativePath,
        expectedCount > 0,
        expectedCount,
      );
      setDeleteModal(null);
      addNotification({ type: "success", title: "已移到废纸篓", message: "", duration: 2000 });
      onRefresh();
      // 选中回退：清掉对该 rel 的选中；若选中了它则回到 __ROOT__
      if (selectedRel === relativePath) setSelectedRel("__ROOT__");
    } catch (e) {
      if (isIpcError(e) && e.code === "E_FOLDER_DIRTY") {
        const details = (e.details ?? {}) as { now?: number };
        const now = typeof details.now === "number" ? details.now : expectedCount;
        // 重弹：dirtyPrev = 当前 expected；expectedCount = details.now
        setDeleteModal({
          relativePath,
          displayName: deleteModal.displayName,
          expectedCount: now,
          dirtyPrev: expectedCount,
          busy: false,
        });
        return;
      }
      // 其他错误：toast + 关闭 modal
      const msg = isIpcError(e) ? renderIpcError(e) : "删除失败";
      setDeleteModal(null);
      addNotification({ type: "error", title: "删除失败", message: msg, duration: 4000 });
    }
  }, [deleteModal, projectId, onRefresh, selectedRel, setSelectedRel, addNotification]);

  const handleRevealRow = useCallback(
    (relativePath: string) => {
      onReveal(relativePath);
    },
    [onReveal],
  );

  // ── 切走二次确认 ──────────────────────────────────────────────────
  /**
   * 在编辑期间执行某 action 前先弹「放弃新建『xxx』？」modal。
   * 仅 creating 用：rename 失败后不阻拦（产品 PRD 给出仅 creating 二次确认）。
   * @returns true 表示已挂起（调用方应直接 return），false 表示无需拦截
   */
  const guardSwitchAwayCreating = useCallback(
    (action: () => void): boolean => {
      if (mode !== "creating") return false;
      const label = editingValue.trim() || DEFAULT_NEW_NAME;
      setPendingDiscard({
        label,
        onConfirm: () => {
          cancelCreate();
          setPendingDiscard(null);
          action();
        },
      });
      return true;
    },
    [mode, editingValue, cancelCreate],
  );

  // 单击选中
  const handleSelect = useCallback(
    (rel: string) => {
      // selection 冻结：renaming 时不响应其他行点击（AC-3）
      if (mode === "renaming" && pendingRenameIds.size > 0) {
        return;
      }
      if (guardSwitchAwayCreating(() => setSelectedRel(rel))) return;
      setSelectedRel(rel);
    },
    [mode, pendingRenameIds, guardSwitchAwayCreating, setSelectedRel],
  );

  // 双击：切换筛选（PRD §3 verifications 4）—— 已选则取消，未选则选中
  const handleDoubleClick = useCallback(
    (rel: string) => {
      if (mode === "renaming" && pendingRenameIds.size > 0) return;
      if (guardSwitchAwayCreating(() => setSelectedRel(selectedRel === rel ? null : rel))) return;
      setSelectedRel(selectedRel === rel ? null : rel);
    },
    [selectedRel, mode, pendingRenameIds, guardSwitchAwayCreating, setSelectedRel],
  );

  // 右键菜单
  const handleContextMenuRow = useCallback(
    (e: React.MouseEvent, item: FolderListRowItem) => {
      e.preventDefault();
      if (isEditing) return; // 编辑期不弹右键
      // 右键即选中该行（与 Finder 行为一致），避免 handler 用错 selection
      setSelectedRel(item.relativePath);
      setCtxMenu({
        x: e.clientX,
        y: e.clientY,
        target: { kind: "row", rowKind: item.kind, relativePath: item.relativePath },
      });
    },
    [isEditing, setSelectedRel],
  );

  const handleContextMenuBlank = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      if (isEditing) return;
      setCtxMenu({ x: e.clientX, y: e.clientY, target: { kind: "blank" } });
    },
    [isEditing],
  );

  // 键盘：Enter / ⌘⌫ / ⌘⇧N + Up/Down 导航
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLDivElement>) => {
      // 编辑期把键盘交给 InlineNameEditor（已 stopPropagation），此处一律不响应
      if (isEditing) return;

      // 上下导航（无 a11y 强求，仅基本可用）
      if (e.key === "ArrowDown" || e.key === "ArrowUp") {
        if (items.length === 0) return;
        const curIdx = Math.max(
          0,
          items.findIndex((it) => it.relativePath === selectedRel),
        );
        const nextIdx =
          e.key === "ArrowDown"
            ? Math.min(items.length - 1, curIdx + 1)
            : Math.max(0, curIdx - 1);
        if (nextIdx !== curIdx) {
          e.preventDefault();
          setSelectedRel(items[nextIdx].relativePath);
        }
        return;
      }

      // ⌘⇧N 新建（P1 可选）
      if (e.key === "n" && e.metaKey && e.shiftKey) {
        e.preventDefault();
        handleCreate();
        return;
      }

      // Enter 进入重命名（仅 root；判定不依赖 UI disabled）
      if (e.key === "Enter") {
        if (selectionKind !== "root") return; // ADR-007 入口判定
        e.preventDefault();
        handleRename();
        return;
      }

      // ⌘⌫ 删除（仅 root）
      if (e.key === "Backspace" && e.metaKey) {
        if (selectionKind !== "root") return;
        e.preventDefault();
        handleDelete();
        return;
      }
    },
    [
      isEditing,
      items,
      selectedRel,
      selectionKind,
      setSelectedRel,
      handleCreate,
      handleRename,
      handleDelete,
    ],
  );

  return (
    <div
      ref={listRef}
      data-testid="workspace-folder-list-view"
      data-mode={mode}
      tabIndex={0}
      onKeyDown={handleKeyDown}
      onContextMenu={(e) => {
        // 仅当事件目标是容器自身（空白处）时触发空白菜单
        if (e.target === e.currentTarget) handleContextMenuBlank(e);
      }}
      className="shrink-0 mb-[var(--space-3)] rounded-[var(--radius-lg)] border overflow-hidden outline-none"
      style={{
        borderColor: "var(--border-primary)",
        background: "var(--surface-primary)",
      }}
    >
      <FolderListToolbar
        selectedKind={selectionKind}
        onCreate={handleCreate}
        onRename={handleRename}
        onDelete={handleDelete}
      />

      {/* 列头 */}
      <div
        role="row"
        data-testid="folder-list-header"
        className="flex items-center gap-2 px-2"
        style={{
          height: 24,
          background: "var(--surface-secondary)",
          borderBottom: "1px solid var(--border-primary)",
          fontWeight: 600,
          fontSize: 13,
          color: "var(--text-secondary)",
        }}
      >
        <div className="flex-1 min-w-0">名称</div>
        <div className="shrink-0 text-right" style={{ width: 56 }}>
          项目数
        </div>
        <div className="shrink-0 text-right" style={{ width: 92 }}>
          修改时间
        </div>
      </div>

      {/* 数据行 */}
      <div data-testid="folder-list-rows">
        {itemsWithGhost.map((item) => {
          const isRenamingRow =
            mode === "renaming" && editingFolderPath === item.relativePath;
          const isCreatingRow = item.ghost === true && mode === "creating";
          const renaming = pendingRenameIds.has(item.relativePath);

          const editor =
            isRenamingRow || isCreatingRow ? (
              <InlineNameEditor
                value={editingValue}
                onChange={updateEditingValue}
                onCommit={isCreatingRow ? submitCreate : submitRename}
                onCancel={isCreatingRow ? cancelCreate : cancelRename}
                invalid={!!editingError}
                error={editingError}
                testId={isCreatingRow ? "inline-editor-create" : "inline-editor-rename"}
              />
            ) : undefined;

          return (
            <FolderListRow
              key={item.relativePath}
              item={item}
              selected={selectedRel === item.relativePath}
              onSelect={() => handleSelect(item.relativePath)}
              onDoubleClick={() => handleDoubleClick(item.relativePath)}
              onContextMenu={(e) => handleContextMenuRow(e, item)}
              nameEditor={editor}
              pending={renaming}
              ghost={item.ghost}
            />
          );
        })}
      </div>

      {ctxMenu ? (
        <FolderContextMenu
          x={ctxMenu.x}
          y={ctxMenu.y}
          target={ctxMenu.target}
          onClose={() => setCtxMenu(null)}
          onRename={handleRename}
          onDelete={handleDelete}
          onReveal={() => {
            if (ctxMenu.target.kind === "row") {
              handleRevealRow(ctxMenu.target.relativePath);
            }
          }}
          onCreate={handleCreate}
        />
      ) : null}

      {/* 切走二次确认 modal（编辑期间点击其他行） */}
      {pendingDiscard ? (
        <div
          data-testid="folder-discard-modal"
          role="dialog"
          aria-modal="true"
          style={{
            position: "fixed",
            inset: 0,
            background: "rgba(0,0,0,0.35)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            zIndex: 9998,
          }}
          onClick={() => setPendingDiscard(null)}
        >
          <div
            onClick={(e) => e.stopPropagation()}
            style={{
              minWidth: 320,
              background: "var(--surface-primary)",
              border: "1px solid var(--border-primary)",
              borderRadius: 8,
              padding: 20,
              boxShadow: "0 8px 32px rgba(0,0,0,0.2)",
              color: "var(--text-primary)",
            }}
          >
            <div style={{ fontSize: 14, marginBottom: 16 }}>
              {`放弃新建「${pendingDiscard.label}」？`}
            </div>
            <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
              <button
                type="button"
                data-testid="folder-discard-cancel"
                onClick={() => setPendingDiscard(null)}
                className="text-[13px] px-3 py-1 rounded-[var(--radius-sm)]"
                style={{
                  border: "1px solid var(--border-primary)",
                  background: "var(--surface-primary)",
                  color: "var(--text-primary)",
                }}
              >
                继续编辑
              </button>
              <button
                type="button"
                data-testid="folder-discard-confirm"
                onClick={() => pendingDiscard.onConfirm()}
                className="text-[13px] px-3 py-1 rounded-[var(--radius-sm)]"
                style={{
                  background: "var(--accent-danger, #ef4444)",
                  color: "#fff",
                  border: "none",
                }}
              >
                放弃
              </button>
            </div>
          </div>
        </div>
      ) : null}

      {/* 删除二次确认 modal */}
      {deleteModal ? (
        <DeleteConfirmModal
          displayName={deleteModal.displayName}
          expectedCount={deleteModal.expectedCount}
          dirtyPrev={deleteModal.dirtyPrev}
          busy={deleteModal.busy}
          onCancel={() => setDeleteModal(null)}
          onConfirm={submitDelete}
        />
      ) : null}
    </div>
  );
}

// 兼容旧 export（T5a 单测可能 import 该函数）
export function firstSegmentRel(
  filePath: string,
  workspaceRoot: string,
): string | null {
  const rel = relativeToWorkspace(filePath, workspaceRoot);
  if (!rel) return null;
  const idx = rel.indexOf("/");
  if (idx < 0) return null;
  return rel.slice(0, idx);
}
