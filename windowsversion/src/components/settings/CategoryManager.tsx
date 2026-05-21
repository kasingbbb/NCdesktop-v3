/**
 * PR-3 task_012: 分类管理 UI（平铺 CRUD）
 */
import { useEffect, useState } from "react";
import { useCategoryStore } from "../../stores/categoryStore";

interface Props {
  libraryId: string;
}

export function CategoryManager({ libraryId }: Props) {
  const setLibrary = useCategoryStore((s) => s.setLibrary);
  const fetch = useCategoryStore((s) => s.fetch);
  const categories = useCategoryStore((s) => s.categories);
  const create = useCategoryStore((s) => s.create);
  const rename = useCategoryStore((s) => s.rename);
  const setDisabled = useCategoryStore((s) => s.setDisabled);
  const remove = useCategoryStore((s) => s.remove);
  const error = useCategoryStore((s) => s.error);

  const [draftSlug, setDraftSlug] = useState("");
  const [draftLabel, setDraftLabel] = useState("");

  useEffect(() => {
    setLibrary(libraryId);
    void fetch(true);
  }, [libraryId, setLibrary, fetch]);

  return (
    <div style={{ padding: 16, fontSize: 13 }}>
      <h3>分类管理</h3>
      {error && <div style={{ color: "#a01818" }}>{error}</div>}

      <table style={{ width: "100%", borderCollapse: "collapse", marginTop: 8 }}>
        <thead>
          <tr style={{ opacity: 0.6 }}>
            <th align="left">slug</th>
            <th align="left">显示名</th>
            <th align="left">类型</th>
            <th align="left">操作</th>
          </tr>
        </thead>
        <tbody>
          {categories.map((c) => (
            <tr key={c.slug}>
              <td>{c.slug}</td>
              <td>
                <input
                  defaultValue={c.label}
                  onBlur={(e) => {
                    if (e.target.value !== c.label) void rename(c.slug, e.target.value);
                  }}
                />
              </td>
              <td>{c.isBuiltin ? "内置" : "自定义"}</td>
              <td>
                <button onClick={() => void setDisabled(c.slug, !c.isDisabled)}>
                  {c.isDisabled ? "启用" : "停用"}
                </button>
                {!c.isBuiltin && (
                  <button
                    onClick={() => {
                      if (confirm(`确认删除 ${c.slug}？仅在引用计数=0 时允许`)) {
                        void remove(c.slug);
                      }
                    }}
                    style={{ marginLeft: 6, color: "#a01818" }}
                  >
                    删除
                  </button>
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>

      <div style={{ marginTop: 16, paddingTop: 12, borderTop: "1px solid rgba(255,255,255,0.1)" }}>
        <h4>新增分类</h4>
        <input placeholder="slug（如 course）" value={draftSlug} onChange={(e) => setDraftSlug(e.target.value)} />
        <input placeholder="显示名（如 课程）" value={draftLabel} onChange={(e) => setDraftLabel(e.target.value)} style={{ marginLeft: 6 }} />
        <button
          style={{ marginLeft: 6 }}
          onClick={() => {
            if (!draftSlug.trim() || !draftLabel.trim()) return;
            void create(draftSlug.trim(), draftLabel.trim());
            setDraftSlug("");
            setDraftLabel("");
          }}
        >
          创建
        </button>
      </div>
    </div>
  );
}
