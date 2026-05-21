
interface SidebarItemProps {
  icon: React.ReactNode;
  label: string;
  active?: boolean;
  badge?: React.ReactNode;
  onClick?: () => void;
  className?: string;
}

export function SidebarItem({ icon, label, active = false, badge, onClick, className = "" }: SidebarItemProps) {
  return (
    <button
      className={`sidebar-item w-full text-left flex items-center mb-[1px] ${active ? "active" : ""} ${className}`}
      type="button"
      onClick={onClick}
    >
      <span className="sidebar-item-icon w-[16px] h-[16px] flex items-center justify-center shrink-0">{icon}</span>
      <span className="flex-1 truncate">{label}</span>
      {badge !== undefined && (
        <span
          className="text-[10px] ml-auto px-[6px] py-[1px] rounded-[var(--radius-full)]"
          style={{
            background: active ? "rgba(147,197,253,0.15)" : "rgba(255,255,255,0.1)",
            color: active ? "var(--sidebar-active-fg)" : "var(--sidebar-text-muted)",
          }}
        >
          {badge}
        </span>
      )}
    </button>
  );
}

interface SidebarSectionProps {
  title: string;
  children: React.ReactNode;
  action?: React.ReactNode;
  titleColor?: string;
}

export function SidebarSection({ title, children, action, titleColor }: SidebarSectionProps) {
  return (
    <div className="mb-[var(--space-1)]">
      <div className="flex items-center justify-between px-[14px] pt-[8px] pb-[4px]">
        <p
          className="text-[10px] font-bold uppercase tracking-[0.08em]"
          style={{ color: titleColor ?? "var(--sidebar-text-dim)" }}
        >
          {title}
        </p>
        {action && <div>{action}</div>}
      </div>
      {children}
    </div>
  );
}
