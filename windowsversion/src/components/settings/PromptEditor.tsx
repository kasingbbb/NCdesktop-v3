/**
 * PR-4 task_014/015/016: Prompt 三段编辑器
 * 骨架：tabs (classify/naming/tagging) + user/output textareas + dry-run + reset
 * 完整 UX（占位符 chip / 红下划线）留 task_017
 */
import { useEffect, useState } from "react";
import { usePromptStore } from "../../stores/promptStore";
import type { PromptInfo } from "../../lib/tauri-commands";

const KINDS: PromptInfo["kind"][] = ["classify", "naming", "tagging"];

export function PromptEditor() {
  const [kind, setKind] = useState<PromptInfo["kind"]>("classify");
  const load = usePromptStore((s) => s.load);
  const drafts = usePromptStore((s) => s.drafts);
  const update = usePromptStore((s) => s.updateDraft);
  const save = usePromptStore((s) => s.save);
  const test = usePromptStore((s) => s.testDryRun);
  const reset = usePromptStore((s) => s.reset);
  const dryRun = usePromptStore((s) => s.dryRun[kind]);

  useEffect(() => {
    void load(kind);
  }, [kind, load]);

  const draft = drafts[kind];
  const userPlaceholderOk = draft.user.includes("{content}");

  return (
    <div style={{ padding: 16, fontSize: 13 }}>
      <div style={{ marginBottom: 12 }}>
        {KINDS.map((k) => (
          <button
            key={k}
            onClick={() => setKind(k)}
            style={{
              padding: "6px 10px",
              marginRight: 6,
              background: k === kind ? "rgba(120,80,200,0.2)" : "transparent",
              border: "1px solid rgba(255,255,255,0.1)",
              borderRadius: 4,
              cursor: "pointer",
              color: "inherit",
            }}
          >
            {k}
          </button>
        ))}
      </div>

      <label>User 段（必含 `{"{content}"}`）</label>
      <textarea
        value={draft.user}
        onChange={(e) => update(kind, "user", e.target.value)}
        rows={12}
        style={{ width: "100%", padding: 8, fontFamily: "monospace", fontSize: 12 }}
      />
      {!userPlaceholderOk && (
        <div style={{ color: "#a01818", marginTop: 4 }}>
          ⚠️ 请在 user 段使用 {"{content}"}；保存按钮已禁用
        </div>
      )}

      <div style={{ marginTop: 12, display: "flex", gap: 8 }}>
        <button onClick={() => void test(kind)}>测试</button>
        <button disabled={!userPlaceholderOk} onClick={() => void save(kind, "user")}>
          保存
        </button>
        <button onClick={() => void reset(kind, "user")}>恢复默认</button>
      </div>

      {dryRun && (
        <div style={{ marginTop: 8, padding: 8, background: "rgba(255,255,255,0.05)", fontSize: 12 }}>
          schema_ok={String(dryRun.schemaOk)} · online_ok={String(dryRun.onlineOk)} · offline_only={String(dryRun.offlineOnly)}
          {dryRun.error && <div style={{ color: "#a01818" }}>{dryRun.error}</div>}
        </div>
      )}
    </div>
  );
}
