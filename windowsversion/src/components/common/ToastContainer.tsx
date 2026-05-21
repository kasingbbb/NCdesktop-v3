import { CheckCircle2, AlertCircle, XCircle, Info, X } from "lucide-react";
import { useUIStore } from "../../stores/uiStore";
import type { Notification } from "../../types/ui";

export function ToastContainer() {
  const notifications = useUIStore((s) => s.notifications);
  const removeNotification = useUIStore((s) => s.removeNotification);

  if (notifications.length === 0) return null;

  return (
    <div
      className="fixed bottom-5 right-5 z-[200] flex flex-col gap-2 pointer-events-none"
      aria-live="polite"
    >
      {notifications.map((n) => (
        <ToastItem
          key={n.id}
          notification={n}
          onDismiss={() => removeNotification(n.id)}
        />
      ))}
    </div>
  );
}

const TYPE_CONFIG: Record<
  Notification["type"],
  { icon: React.ReactNode; color: string; bg: string }
> = {
  success: {
    icon: <CheckCircle2 size={14} />,
    color: "var(--color-success)",
    bg: "rgba(52,199,89,.1)",
  },
  warning: {
    icon: <AlertCircle size={14} />,
    color: "var(--color-warning)",
    bg: "rgba(255,149,0,.1)",
  },
  error: {
    icon: <XCircle size={14} />,
    color: "var(--color-danger)",
    bg: "rgba(255,59,48,.1)",
  },
  info: {
    icon: <Info size={14} />,
    color: "var(--color-accent)",
    bg: "rgba(59,130,246,.1)",
  },
};

function ToastItem({
  notification,
  onDismiss,
}: {
  notification: Notification;
  onDismiss: () => void;
}) {
  const cfg = TYPE_CONFIG[notification.type];
  return (
    <div
      className="flex items-start gap-[var(--space-3)] px-[var(--space-3)] py-[var(--space-3)]
                 rounded-[var(--radius-lg)] border pointer-events-auto max-w-[300px]"
      style={{
        background: "var(--surface-elevated)",
        borderColor: "var(--border-primary)",
        boxShadow: "var(--shadow-md)",
        animation: "toastEnter var(--duration-normal) var(--ease-out-expo)",
      }}
    >
      <div
        className="w-[24px] h-[24px] rounded-full flex items-center justify-center shrink-0"
        style={{ background: cfg.bg, color: cfg.color }}
      >
        {cfg.icon}
      </div>
      <div className="flex-1 min-w-0">
        <p
          className="text-[var(--text-sm)] font-semibold"
          style={{ color: "var(--text-primary)" }}
        >
          {notification.title}
        </p>
        {notification.message && (
          <p
            className="text-[var(--text-xs)] mt-0.5 leading-relaxed"
            style={{ color: "var(--text-secondary)" }}
          >
            {notification.message}
          </p>
        )}
      </div>
      <button
        type="button"
        onClick={onDismiss}
        className="shrink-0 w-[18px] h-[18px] flex items-center justify-center
                   rounded transition-colors"
        style={{ color: "var(--text-tertiary)" }}
        onMouseEnter={(e) =>
          ((e.currentTarget as HTMLElement).style.background = "var(--surface-tertiary)")
        }
        onMouseLeave={(e) =>
          ((e.currentTarget as HTMLElement).style.background = "transparent")
        }
        aria-label="关闭通知"
      >
        <X size={11} />
      </button>
    </div>
  );
}
