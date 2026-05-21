/**
 * PR-1 task_004: AppMode 横幅
 * - Normal：不渲染
 * - Degraded：黄条提示
 * - ReadOnly：红条提示，提示用户写操作不可用
 */
import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useUIStore, type AppMode } from "../../stores/uiStore";

export function AppModeBanner() {
  const appMode = useUIStore((s) => s.appMode);
  const setAppMode = useUIStore((s) => s.setAppMode);

  useEffect(() => {
    invoke<AppMode>("get_app_mode")
      .then((m) => setAppMode(m))
      .catch((e) => console.warn("[AppModeBanner] get_app_mode 失败:", e));
  }, [setAppMode]);

  if (!appMode || appMode.kind === "normal") return null;

  if (appMode.kind === "degraded") {
    return (
      <div
        role="alert"
        style={{
          padding: "8px 16px",
          background: "rgba(255, 200, 0, 0.15)",
          color: "#a76a00",
          borderBottom: "1px solid rgba(255, 200, 0, 0.4)",
          fontSize: 13,
        }}
      >
        ⚠️ 启动自愈完成，但有 {appMode.failed_count} 条数据无法归类（{appMode.reason}）。
        受影响资产已归入「未归类」，可正常使用。
      </div>
    );
  }

  // read_only
  return (
    <div
      role="alert"
      style={{
        padding: "8px 16px",
        background: "rgba(220, 60, 60, 0.18)",
        color: "#a01818",
        borderBottom: "1px solid rgba(220, 60, 60, 0.5)",
        fontSize: 13,
      }}
    >
      🔒 当前为只读安全模式（{appMode.reason}），导入与编辑功能已禁用，仅可查阅与导出。
    </div>
  );
}
