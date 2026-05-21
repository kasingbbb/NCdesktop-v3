/**
 * PR-3 task_011: 三段面包屑 Library > Project > WorkspaceView/...
 */
interface Props {
  libraryName: string;
  projectName: string;
  categorySlug: string | null;
  onNavigateProject?: () => void;
  onNavigateLibrary?: () => void;
}

export function Breadcrumb({ libraryName, projectName, categorySlug, onNavigateProject, onNavigateLibrary }: Props) {
  return (
    <nav style={{ padding: "6px 12px", fontSize: 12, opacity: 0.85 }}>
      <Link onClick={onNavigateLibrary}>{libraryName}</Link>
      <Sep />
      <Link onClick={onNavigateProject}>{projectName}</Link>
      {categorySlug && (
        <>
          <Sep />
          <span>{categorySlug}</span>
        </>
      )}
    </nav>
  );
}
function Link({ children, onClick }: { children: React.ReactNode; onClick?: () => void }) {
  return (
    <button onClick={onClick} style={{ background: "none", border: "none", cursor: onClick ? "pointer" : "default", color: "inherit" }}>
      {children}
    </button>
  );
}
function Sep() {
  return <span style={{ margin: "0 6px", opacity: 0.5 }}>›</span>;
}
