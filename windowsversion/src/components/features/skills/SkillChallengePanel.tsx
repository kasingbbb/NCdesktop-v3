/**
 * SkillChallengePanel — 技能验证面板（Step 10 / K8）
 *
 * 宪章 K8：使用开放式场景题，不用选择题。
 * 流程：
 *   1. 调用 skillGenerateChallenge → 获取情景题
 *   2. 用户输入答案
 *   3. 调用 skillEvaluateAnswer → 获取评分与反馈
 *   4. qualityScore >= 0.7 → 技能被标记 verified
 */

import { useEffect, useRef, useState } from "react";
import {
  CheckCircle2,
  ChevronRight,
  Loader2,
  RotateCcw,
  Send,
  XCircle,
} from "lucide-react";
import type { Skill, SkillChallenge, SkillEvaluation } from "../../../lib/tauri-commands";
import {
  skillEvaluateAnswer,
  skillGenerateChallenge,
} from "../../../lib/tauri-commands";
import "./SkillChallengePanel.css";

// ─── Props ────────────────────────────────────────────────────────────────────

interface Props {
  skill: Skill;
  onClose: () => void;
  onVerified: () => void;
}

// ─── 主组件 ───────────────────────────────────────────────────────────────────

export function SkillChallengePanel({ skill, onClose, onVerified }: Props) {
  const [phase, setPhase] = useState<"loading" | "answering" | "evaluating" | "result">(
    "loading"
  );
  const [challenge, setChallenge] = useState<SkillChallenge | null>(null);
  const [answer, setAnswer] = useState("");
  const [evaluation, setEvaluation] = useState<SkillEvaluation | null>(null);
  const [error, setError] = useState<string | null>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // ── 加载题目 ────────────────────────────────────────────────────────────────
  const loadChallenge = async () => {
    setPhase("loading");
    setError(null);
    setAnswer("");
    setEvaluation(null);
    try {
      const c = await skillGenerateChallenge(skill.id);
      setChallenge(c);
      setPhase("answering");
      // 聚焦输入框
      setTimeout(() => textareaRef.current?.focus(), 80);
    } catch (e) {
      setError(String(e));
      setPhase("answering"); // 降级：让用户看到错误而不是永久 loading
    }
  };

  useEffect(() => {
    loadChallenge();
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [skill.id]);

  // ── 提交答案 ────────────────────────────────────────────────────────────────
  const handleSubmit = async () => {
    if (!answer.trim() || !challenge) return;
    setPhase("evaluating");
    setError(null);
    try {
      const ev = await skillEvaluateAnswer(skill.id, answer);
      setEvaluation(ev);
      setPhase("result");
      if (ev.statusTransition === "verified" || ev.qualityScore >= 0.75) {
        // 稍等让用户看到结果再回调
        setTimeout(onVerified, 1800);
      }
    } catch (e) {
      setError(String(e));
      setPhase("answering");
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      handleSubmit();
    }
  };

  // ─── 渲染 ──────────────────────────────────────────────────────────────────

  return (
    <div className="scp-root">
      {/* 标题栏 */}
      <div className="scp-header">
        <span className="scp-title">技能验证</span>
        <button className="scp-close" onClick={onClose} title="关闭">
          ✕
        </button>
      </div>

      {/* 加载题目 */}
      {phase === "loading" && (
        <div className="scp-loading">
          <Loader2 size={20} className="scp-spin" />
          <span>正在生成情景题…</span>
        </div>
      )}

      {/* 错误 */}
      {error && phase !== "loading" && (
        <div className="scp-error">
          <XCircle size={14} />
          <span>{error}</span>
        </div>
      )}

      {/* 答题阶段 */}
      {(phase === "answering" || phase === "evaluating") && challenge && (
        <>
          {/* 情景题 */}
          <div className="scp-question-wrap">
            <div className="scp-question-label">场景题</div>
            <p className="scp-question">{challenge.question}</p>
            {challenge.scenario && (
              <p className="scp-context">{challenge.scenario}</p>
            )}
            {challenge.evaluationHints.length > 0 && (
              <div className="scp-keypoints">
                <span className="scp-keypoints-label">考察点：</span>
                {challenge.evaluationHints.map((kp: string, i: number) => (
                  <span key={i} className="scp-kp-chip">{kp}</span>
                ))}
              </div>
            )}
          </div>

          {/* 答案输入 */}
          <div className="scp-answer-wrap">
            <label className="scp-answer-label">
              你的回答
              <span className="scp-answer-hint">⌘↵ 提交</span>
            </label>
            <textarea
              ref={textareaRef}
              className="scp-answer-textarea"
              value={answer}
              onChange={(e) => setAnswer(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="根据你对这个技能的理解，写出你的答案……"
              disabled={phase === "evaluating"}
              rows={6}
            />
          </div>

          {/* 提交按钮 */}
          <div className="scp-actions">
            <button
              className="scp-submit-btn"
              onClick={handleSubmit}
              disabled={!answer.trim() || phase === "evaluating"}
            >
              {phase === "evaluating" ? (
                <>
                  <Loader2 size={14} className="scp-spin" />
                  评估中…
                </>
              ) : (
                <>
                  <Send size={14} />
                  提交回答
                  <ChevronRight size={14} />
                </>
              )}
            </button>
          </div>
        </>
      )}

      {/* 结果阶段 */}
      {phase === "result" && evaluation && (
        <EvaluationResult
          evaluation={evaluation}
          onRetry={() => loadChallenge()}
          onClose={onClose}
        />
      )}
    </div>
  );
}

// ─── EvaluationResult ─────────────────────────────────────────────────────────

interface ResultProps {
  evaluation: SkillEvaluation;
  onRetry: () => void;
  onClose: () => void;
}

function EvaluationResult({ evaluation, onRetry, onClose }: ResultProps) {
  const score = Math.round(evaluation.qualityScore * 100);
  const passed = evaluation.statusTransition === "verified" || evaluation.qualityScore >= 0.75;

  return (
    <div className="scp-result">
      {/* 分数环 */}
      <div className={`scp-score-ring ${passed ? "scp-score-pass" : "scp-score-fail"}`}>
        <span className="scp-score-num">{score}</span>
        <span className="scp-score-unit">分</span>
      </div>

      {/* 通过 / 未通过 */}
      {passed ? (
        <div className="scp-pass-badge">
          <CheckCircle2 size={16} />
          <span>技能验证通过</span>
        </div>
      ) : (
        <div className="scp-fail-badge">
          <XCircle size={16} />
          <span>还需再练习</span>
        </div>
      )}

      {/* 反馈文字 */}
      <p className="scp-feedback">{evaluation.feedback}</p>

      {/* 覆盖 / 遗漏 */}
      {evaluation.coveredPoints.length > 0 && (
        <div className="scp-points-section">
          <div className="scp-points-label scp-covered-label">已覆盖</div>
          <ul className="scp-points-list">
            {evaluation.coveredPoints.map((p, i) => (
              <li key={i} className="scp-point-item scp-covered">✓ {p}</li>
            ))}
          </ul>
        </div>
      )}

      {evaluation.missedPoints.length > 0 && (
        <div className="scp-points-section">
          <div className="scp-points-label scp-missed-label">待加强</div>
          <ul className="scp-points-list">
            {evaluation.missedPoints.map((p, i) => (
              <li key={i} className="scp-point-item scp-missed">+ {p}</li>
            ))}
          </ul>
        </div>
      )}

      {/* 操作按钮 */}
      <div className="scp-result-actions">
        {!passed && (
          <button className="scp-retry-btn" onClick={onRetry}>
            <RotateCcw size={13} />
            再试一题
          </button>
        )}
        <button className="scp-done-btn" onClick={onClose}>
          完成
        </button>
      </div>
    </div>
  );
}
