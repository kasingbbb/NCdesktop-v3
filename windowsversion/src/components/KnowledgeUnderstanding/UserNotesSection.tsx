/**
 * UserNotesSection — 「用你自己的话解释这个概念」区域
 *
 * 功能：
 *   - 自由文本 textarea（1s debounce 自动保存）
 *   - 「给我一个出发点」草稿生成（复用 explanation.essenceSentence）
 *   - 「和 AI 核对一下」触发镜子反馈
 */

import { useState, useRef, useCallback, useEffect } from "react";
import { Lightbulb, MessageCircle } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { useKnowledgeUnderstandingStore } from "../../stores/knowledgeUnderstandingStore";
import { MirrorFeedbackDisplay } from "./MirrorFeedbackDisplay";

interface UserNotesSectionProps {
  conceptId: string;
}

export function UserNotesSection({ conceptId }: UserNotesSectionProps) {
  const userNote = useKnowledgeUnderstandingStore((s) => s.userNote);
  const explanation = useKnowledgeUnderstandingStore((s) => s.explanation);
  const store = useKnowledgeUnderstandingStore;

  const [text, setText] = useState(userNote?.userExplanation ?? "");
  const [saveStatus, setSaveStatus] = useState<"idle" | "saving" | "saved" | "error">("idle");
  const [validating, setValidating] = useState(false);
  const saveTimerRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const saveStatusTimerRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  // 同步 store 中的 userNote 到本地 text
  useEffect(() => {
    if (userNote?.userExplanation && !text) {
      setText(userNote.userExplanation);
    }
  }, [userNote?.userExplanation]);

  // ── 1s debounce 自动保存 ──────────────────────────────────────────────────

  const doSave = useCallback(
    async (content: string) => {
      if (!content.trim()) return;
      setSaveStatus("saving");
      try {
        await invoke<void>("knowledge_save_user_note", {
          conceptId,
          userExplanation: content,
        });
        setSaveStatus("saved");
        clearTimeout(saveStatusTimerRef.current);
        saveStatusTimerRef.current = setTimeout(() => setSaveStatus("idle"), 2000);
      } catch {
        setSaveStatus("error");
      }
    },
    [conceptId]
  );

  const handleTextChange = (value: string) => {
    setText(value);
    clearTimeout(saveTimerRef.current);
    saveTimerRef.current = setTimeout(() => {
      void doSave(value);
    }, 1000);
  };

  // 组件卸载时清理定时器
  useEffect(() => {
    return () => {
      clearTimeout(saveTimerRef.current);
      clearTimeout(saveStatusTimerRef.current);
    };
  }, []);

  // ── 「给我一个出发点」────────────────────────────────────────────────────

  const handleStartingPoint = () => {
    const essenceSentence = explanation?.essenceSentence;
    if (!essenceSentence) return;

    if (text.trim()) {
      setText((prev) => prev + "\n\n" + essenceSentence);
    } else {
      setText(essenceSentence);
    }
  };

  // ── 「和 AI 核对一下」────────────────────────────────────────────────────

  const handleValidate = async () => {
    if (!text.trim() || validating) return;

    setValidating(true);
    store.getState().setMirrorFeedback(null);
    store.getState().setMirrorStatus("streaming");
    store.setState({ mirrorStreamBuffer: "" });

    try {
      // 先保存最新内容
      await invoke<void>("knowledge_save_user_note", {
        conceptId,
        userExplanation: text,
      });

      // 再触发验证
      await invoke<string>("knowledge_validate_explanation", {
        conceptId,
        userExplanation: text,
      });
    } catch {
      store.getState().setMirrorStatus("error");
    } finally {
      setValidating(false);
    }
  };

  const canValidate = text.trim().length > 0 && !validating;
  const hasStartingPoint = !!explanation?.essenceSentence;

  return (
    <section className="space-y-[var(--space-3)]">
      {/* 标题 */}
      <h3
        className="text-[var(--text-sm)] font-semibold"
        style={{ color: "var(--text-primary)" }}
      >
        用你自己的话解释这个概念
      </h3>

      {/* 文本输入区 */}
      <div className="space-y-[var(--space-2)]">
        <textarea
          value={text}
          onChange={(e) => handleTextChange(e.target.value)}
          placeholder="试试用自己的语言描述这个概念……"
          rows={5}
          className="w-full px-[var(--space-3)] py-[var(--space-3)] rounded-[var(--radius-md)] text-[var(--text-sm)] leading-relaxed resize-y outline-none transition-colors"
          style={{
            background: "var(--surface-secondary)",
            border: "1px solid var(--border-primary)",
            color: "var(--text-primary)",
            minHeight: 100,
          }}
        />

        {/* 保存状态 + 按钮行 */}
        <div className="flex items-center justify-between">
          <span
            className="text-[var(--text-xs)]"
            style={{ color: "var(--text-tertiary)" }}
          >
            {saveStatus === "saving" && "正在保存..."}
            {saveStatus === "saved" && "已保存"}
            {saveStatus === "error" && "保存失败"}
          </span>

          <div className="flex items-center gap-[var(--space-2)]">
            {/* 给我一个出发点 */}
            {hasStartingPoint && (
              <button
                type="button"
                onClick={handleStartingPoint}
                className="flex items-center gap-1 px-[var(--space-3)] py-[var(--space-1)] rounded-[var(--radius-md)] text-[var(--text-xs)] transition-colors"
                style={{
                  background: "var(--surface-secondary)",
                  border: "1px solid var(--border-primary)",
                  color: "var(--text-secondary)",
                }}
              >
                <Lightbulb size={11} />
                给我一个出发点
              </button>
            )}

            {/* 和 AI 核对一下 */}
            <button
              type="button"
              onClick={() => void handleValidate()}
              disabled={!canValidate}
              className="flex items-center gap-1 px-[var(--space-3)] py-[var(--space-1)] rounded-[var(--radius-md)] text-[var(--text-xs)] font-medium transition-colors"
              style={{
                background: canValidate ? "var(--brand-navy)" : "var(--surface-tertiary)",
                color: canValidate ? "#fff" : "var(--text-tertiary)",
                opacity: canValidate ? 1 : 0.6,
              }}
            >
              <MessageCircle size={11} />
              {validating ? "核对中..." : "和 AI 核对一下"}
            </button>
          </div>
        </div>
      </div>

      {/* 镜子反馈 */}
      <MirrorFeedbackDisplay />
    </section>
  );
}
