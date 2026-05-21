/**
 * PR-3 task_009: 纵向分类侧边栏（feature flag `workspace_view_v2` 控制）
 * MVP 骨架：列表 + 计数 + 启停折叠分组
 */
import { useEffect } from "react";
import { useCategoryStore } from "../../stores/categoryStore";

interface Props {
  libraryId: string;
}

export function WorkspaceCategorySidebar({ libraryId }: Props) {
  const setLibrary = useCategoryStore((s) => s.setLibrary);
  const fetch = useCategoryStore((s) => s.fetch);
  const categories = useCategoryStore((s) => s.categories);
  const activeSlug = useCategoryStore((s) => s.activeSlug);
  const setActive = useCategoryStore((s) => s.setActive);

  useEffect(() => {
    setLibrary(libraryId);
    void fetch();
  }, [libraryId, setLibrary, fetch]);

  const enabled = categories.filter((c) => !c.isDisabled);

  return (
    <nav style={{ padding: 8, fontSize: 13 }}>
      <div style={{ opacity: 0.6, marginBottom: 6 }}>分类</div>
      {enabled.map((c) => (
        <button
          key={c.slug}
          onClick={() => setActive(c.slug)}
          style={{
            display: "block",
            width: "100%",
            textAlign: "left",
            padding: "6px 8px",
            background: activeSlug === c.slug ? "rgba(120,80,200,0.15)" : "transparent",
            border: "none",
            borderRadius: 4,
            cursor: "pointer",
            color: "inherit",
          }}
        >
          {c.isBuiltin ? "⚙ " : "📁 "}
          {c.label}
        </button>
      ))}
    </nav>
  );
}
