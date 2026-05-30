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

  confirmImport: async () => {
    const scan = get().pending;
    if (!scan) return;
    set({ isImporting: true, error: null });
    try {
      const paths = scan.newFiles.map((f) => f.path);
      const summary = await cmd.importDropPaths(paths);
      // 导入成功 → 落去重集合（即便部分失败也标记，避免失败项每次重连反复弹；
      // 失败项会进 summary.failures，由现有提取管线的重试/自愈兜底）。
      await cmd.markCardImported(scan.newFiles.map((f) => f.hash));
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
