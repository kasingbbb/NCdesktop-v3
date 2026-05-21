import { Settings, Box } from "lucide-react";
import { SidebarItem } from "./SidebarItem";
import { useSyncStore } from "../../stores/syncStore";
import { invoke } from "@tauri-apps/api/core";

interface SidebarFooterProps {
  onSettingsOpen?: () => void;
}

export function SidebarFooter({ onSettingsOpen }: SidebarFooterProps) {
  const isTFCardConnected = useSyncStore((state) => state.isTFCardConnected);

  return (
    <div
      data-testid="sidebar-footer"
      className="px-[var(--space-3)] py-[var(--space-3)] border-t flex items-center gap-[var(--space-2)]"
      style={{ borderColor: "var(--border-primary)" }}
    >
      <div className="flex-1 min-w-0">
        <SidebarItem
          icon={<Settings size={16} />}
          label="设置"
          onClick={onSettingsOpen}
        />
        <SidebarItem
          icon={<Box size={16} />}
          label="悬浮导入"
          onClick={() => {
            invoke("toggle_dropzone_window").catch(console.error);
          }}
        />
      </div>
      {isTFCardConnected ? (
        <span
          data-testid="sidebar-footer-tf-badge"
          className="text-[10px] uppercase tracking-wide px-1.5 py-0.5 rounded font-semibold"
          style={{
            background: "var(--surface-tertiary)",
            color: "var(--text-secondary)",
          }}
          title="TF 卡已连接"
        >
          TF
        </span>
      ) : (
        <span
          data-testid="sidebar-footer-tf-dot"
          className="w-1.5 h-1.5 rounded-full shrink-0"
          style={{ background: "var(--text-tertiary)" }}
          title="未插入 TF 卡"
          aria-label="未插入 TF 卡"
        />
      )}
    </div>
  );
}
