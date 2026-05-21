import { FolderPlus } from "lucide-react";

export function EmptyState() {
  return (
    <div className="flex-1 flex flex-col items-center justify-center p-[var(--space-8)] h-full">
      <div 
        className="w-24 h-24 rounded-full flex items-center justify-center mb-[var(--space-4)]"
        style={{ background: "var(--surface-secondary)", border: "1px solid var(--border-primary)" }}
      >
        <FolderPlus size={40} style={{ color: "var(--text-tertiary)" }} />
      </div>
      <h2 
        className="text-[var(--text-xl)] font-semibold mb-[var(--space-2)]"
        style={{ color: "var(--text-secondary)" }}
      >
        Welcome to NoteCapt
      </h2>
      <p 
        className="text-[var(--text-sm)] mb-[var(--space-4)] max-w-sm text-center"
        style={{ color: "var(--text-tertiary)" }}
      >
        Your knowledge library is empty. Import a session from your TF card or create a new project manually.
      </p>
      <button 
        className="btn-glass px-[var(--space-4)] py-[var(--space-2)] rounded-[var(--radius-md)] flex items-center gap-2"
        style={{ background: "var(--brand-navy)", color: "#ffffff" }}
      >
        <FolderPlus size={16} />
        <span>New Project</span>
      </button>
    </div>
  );
}
