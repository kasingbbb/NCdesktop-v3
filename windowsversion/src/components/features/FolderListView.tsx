/**
 * PR-3 task_010: 列表视图（虚拟滚动 v2 接入 react-virtuoso）
 * MVP 骨架：图标/名称/分类/标签/大小/修改时间
 */
import { useEffect, useState } from "react";
import * as cmd from "../../lib/tauri-commands";

interface Props {
  projectId: string;
  categorySlug: string | null;
}

export function FolderListView({ projectId, categorySlug }: Props) {
  const [items, setItems] = useState<cmd.AssetView[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void cmd
      .listWorkspaceAssets({ projectId, categorySlug })
      .then((r) => setItems(r.items))
      .catch((e) => setError(String(e)));
  }, [projectId, categorySlug]);

  if (error) return <div>读取失败：{error}</div>;
  if (items.length === 0) return <EmptyImportCTAStub categorySlug={categorySlug} />;

  return (
    <table style={{ width: "100%", fontSize: 13, borderCollapse: "collapse" }}>
      <thead>
        <tr style={{ opacity: 0.6 }}>
          <th align="left">名称</th>
          <th align="left">分类</th>
          <th align="left">标签</th>
          <th align="right">大小</th>
          <th align="left">修改时间</th>
        </tr>
      </thead>
      <tbody>
        {items.map((a) => (
          <tr key={a.id}>
            <td>{iconFor(a.iconHint)} {a.name}</td>
            <td>{a.categorySlug ?? "—"}</td>
            <td>{a.tags.slice(0, 3).join(" · ")}</td>
            <td align="right">{formatSize(a.sizeBytes)}</td>
            <td>{a.updatedAt}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function iconFor(h: cmd.AssetView["iconHint"]) {
  return { image: "🖼", video: "🎬", audio: "🎵", pdf: "📄", office: "📘", text: "📝", unknown: "📦" }[h];
}
function formatSize(n: number) {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / 1024 / 1024).toFixed(1)} MB`;
}

function EmptyImportCTAStub({ categorySlug }: { categorySlug: string | null }) {
  return (
    <div style={{ padding: 24, textAlign: "center", opacity: 0.7 }}>
      此分类「{categorySlug ?? "全部"}」暂无文件。拖入文件可直接归入此分类。
    </div>
  );
}
