import { create } from "zustand";
import * as cmd from "../lib/tauri-commands";
import type { UsbCardScan } from "../lib/tauri-commands";
import { logger } from "../utils/logger";

/**
 * U盘裸图片自动导入：检测 → 确认 → 导入 的前端状态机。
 *
 * 触发源（两条，靠 `present()` 的幂等去重避免重复弹窗）：
 *  1. 后端卷监听器 emit 的 `usb-card-detected` 事件（插拔/重连）；
 *  2. 窗口挂载时 `scanUsbCardNow()` 主动扫一次（兜底启动时卡已在）。
 *
 * 确认后复用现成 `import_drop_paths` 管线（落库 + 提取 → MD → 工作区），
 * 导入成功再 `markCardImported(hashes)` 落去重集合，使同一批图片重连不再重复提示。
 */
interface UsbImportStore {
  /** 当前待确认的扫描结果；null 表示无弹窗。 */
  pending: UsbCardScan | null;
  /** 导入进行中（确认后到完成）。 */
  isImporting: boolean;
  /** 最近一次导入结果（成功提示用）。 */
  lastResult: { imported: number; failed: number } | null;
  /** 最近一次错误文案。 */
  error: string | null;

  /** 收到一次扫描结果：有新文件且当前空闲才弹窗（幂等去重）。 */
  present: (scan: UsbCardScan) => void;
  /** 关闭弹窗（忽略本次，不写去重集——下次重连仍会提示）。 */
  dismiss: () => void;
  /** 确认导入：调 import_drop_paths + markCardImported。 */
  confirmImport: () => Promise<void>;
  /** 手动重扫目标卷（窗口聚焦/手动触发）。present 幂等去重，不会重复弹窗。 */
  rescan: () => Promise<void>;
}

export const useUsbImportStore = create<UsbImportStore>((set, get) => ({
  pending: null,
  isImporting: false,
  lastResult: null,
  error: null,

  present: (scan) => {
    const { pending, isImporting } = get();
    // 导入中 / 已有待确认弹窗 → 忽略（防 事件+主动扫描 双触发同一批图片重复弹窗）。
    if (isImporting || pending) return;
    if (!scan.newFiles || scan.newFiles.length === 0) return;
    logger.info("usbImport", "检测到新图片，弹确认框", {
      device: scan.deviceName,
      count: scan.newFiles.length,
    });
    set({ pending: scan, error: null, lastResult: null });
  },

  dismiss: () => set({ pending: null, error: null }),

  rescan: async () => {
    // 已在弹窗/导入中就别打扰；否则主动扫一次（覆盖"设备已挂载、新增了文件"或
    // "通过软件重新接入"等不产生挂载边沿的场景）。
    const { pending, isImporting } = get();
    if (pending || isImporting) return;
    try {
      const scan = await cmd.scanUsbCardNow();
      if (scan) get().present(scan);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      logger.warn("usbImport", "rescan 失败", { err: msg });
    }
  },

  confirmImport: async () => {
    const scan = get().pending;
    if (!scan) return;
    set({ isImporting: true, error: null });
    try {
      const paths = scan.newFiles.map((f) => f.path);
      const summary = await cmd.importDropPaths(paths);
      // 去重集合**只**记录成功导入到工作区的文件（修复：旧实现把失败项也标记成
      // "已导入"，导致复制失败的文件每次重新接入都被永久跳过、再也读不进来）。
      // 判定：原始路径出现在 summary.failures 文案里的视为导入失败，不入去重集。
      const succeededHashes = scan.newFiles
        .filter((f) => !summary.failures.some((msg) => msg.includes(f.path)))
        .map((f) => f.hash);
      if (succeededHashes.length > 0) {
        await cmd.markCardImported(succeededHashes);
      }
      logger.info("usbImport", "导入完成", {
        imported: summary.created.length,
        failed: summary.failures.length,
      });
      set({
        isImporting: false,
        pending: null,
        lastResult: {
          imported: summary.created.length,
          failed: summary.failures.length,
        },
      });
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      logger.error("usbImport", "导入失败", { err: msg });
      // 保留 pending，让用户可重试或关闭。
      set({ isImporting: false, error: msg });
    }
  },
}));
