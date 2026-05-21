/**
 * KnowledgeUnitPage — 知识单元完整学习页（Step 6）
 *
 * 渐进式交互：四步骤折叠/展开
 *   1. 摘要（你的文档怎么说）
 *   2. 理解框架（核心机制 + 典型场景 + 一句话精华）
 *   3. 用你自己的话说（笔记 + 出发点提示）
 *   4. 和 AI 核对（镜子反馈）
 *
 * + 底部：推荐行动卡（宪章 K3：只有一个主操作）
 * + 理解历史时间线（UnderstandingSnapshot）
 *
 * 约束：A1/A2/K3/K4/K5
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { AlertTriangle, ArrowLeft, BookOpen, ChevronDown, ChevronRight, Loader2, RotateCcw, Sparkles } from "lucide-react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { useKnowledgeUnitsStore } from "../../stores/knowledgeUnitsStore";
import {
  statusToIcon,
  STATUS_LABELS,
  addDays,
  calcNextReviewDays,
  type KnowledgeStatus,
  type KnowledgeExplanation,
  type MirrorFeedbackResult,
  type UnderstandingSnapshot,
} from "../../types/knowledge-units";
import * as cmd from "../../lib/tauri-commands";
import "./KnowledgeUnitPage.css";

// ─── 流式事件类型 ────────────────────────────────────────────────────────────

interface StreamChunk {
  knowledgeUnitId: string;
  chunk: string;
  isFinal: boolean;
}

// ─── 主组件 ───────────────────────────────────────────────────────────────────

interface Props {
  unitId: string;
  onBack: () => void;
}

export function KnowledgeUnitPage({ unitId, onBack }: Props) {
  const {
    unitDetail,
    snapshots,
    loadDetail,
    loadSnapshots,
    updateNote,
    updateStatus,
    updateReviewSchedule,
    createSnapshot,
  } = useKnowledgeUnitsStore();

  // ── 本地 UI 状态 ──────────────────────────────────────────────────────────
  const [summaryExpanded, setSummaryExpanded] = useState(true);
  const [explanationExpanded, setExplanationExpanded] = useState(false);
  const [noteExpanded, setNoteExpanded] = useState(false);
  const [historyExpanded, setHistoryExpanded] = useState(false);

  const [summaryStream, setSummaryStream] = useState("");
  const [summaryStatus, setSummaryStatus] = useState<"idle" | "streaming" | "done">("idle");

  const [explanationStatus, setExplanationStatus] = useState<"idle" | "streaming" | "done">("idle");
  const [parsedExplanation, setParsedExplanation] = useState<KnowledgeExplanation | null>(null);

  const [mirrorStatus, setMirrorStatus] = useState<"idle" | "streaming" | "done">("idle");
  const [mirrorFeedback, setMirrorFeedback] = useState<MirrorFeedbackResult | null>(null);

  const [noteText, setNoteText] = useState("");
  const [noteSaved, setNoteSaved] = useState(false);
  const noteSaveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Step 8: staleness 检测
  const [isStale, setIsStale] = useState(false);

  const unlistenRefs = useRef<UnlistenFn[]>([]);

  // ── 挂载：加载详情 + 注册流式事件 ────────────────────────────────────────

  useEffect(() => {
    loadDetail(unitId);
    loadSnapshots(unitId);
  }, [unitId, loadDetail, loadSnapshots]);

  // 详情加载完毕后同步本地状态
  useEffect(() => {
    if (!unitDetail || unitDetail.id !== unitId) return;

    setNoteText(unitDetail.userNote ?? "");

    if (unitDetail.summary) {
      setSummaryStatus("done");
    }
    if (unitDetail.explanation) {
      setExplanationStatus("done");
      try {
        const ex =
          typeof unitDetail.explanation === "string"
            ? (JSON.parse(unitDetail.explanation as unknown as string) as KnowledgeExplanation)
            : unitDetail.explanation;
        setParsedExplanation(ex);
      } catch {
        // leave null
      }
    }
    if (unitDetail.lastMirrorFeedback) {
      setMirrorStatus("done");
      try {
        const fb =
          typeof unitDetail.lastMirrorFeedback === "string"
            ? (JSON.parse(unitDetail.lastMirrorFeedback as unknown as string) as MirrorFeedbackResult)
            : unitDetail.lastMirrorFeedback;
        setMirrorFeedback(fb);
      } catch {
        // leave null
      }
    }
  }, [unitDetail, unitId]);

  // 注册流式事件
  useEffect(() => {
    const registerListeners = async () => {
      const unSummary = await listen<StreamChunk>(
        "notecapt/ku-summary-chunk",
        (e) => {
          if (e.payload.knowledgeUnitId !== unitId) return;
          setSummaryStream((s) => s + e.payload.chunk);
          if (e.payload.isFinal) {
            setSummaryStatus("done");
            loadDetail(unitId);
          }
        }
      );
      const unExplanation = await listen<StreamChunk>(
        "notecapt/ku-explanation-chunk",
        (e) => {
          if (e.payload.knowledgeUnitId !== unitId) return;
          if (e.payload.isFinal) {
            setExplanationStatus("done");
            loadDetail(unitId);
          }
        }
      );
      const unMirror = await listen<StreamChunk>(
        "notecapt/ku-mirror-chunk",
        (e) => {
          if (e.payload.knowledgeUnitId !== unitId) return;
          if (e.payload.isFinal) {
            setMirrorStatus("done");
            loadDetail(unitId);
          }
        }
      );
      unlistenRefs.current = [unSummary, unExplanation, unMirror];
    };
    registerListeners();
    return () => {
      unlistenRefs.current.forEach((fn) => fn());
    };
  }, [unitId, loadDetail]);

  // 无摘要时自动触发生成；有摘要时检查是否过期
  useEffect(() => {
    if (!unitDetail || unitDetail.id !== unitId) return;
    if (!unitDetail.summary && summaryStatus === "idle") {
      triggerSummary(false);
    } else if (unitDetail.summary) {
      // Step 8: 检查是否有新素材使摘要过期
      cmd.kuCheckStaleness(unitId).then(stale => setIsStale(stale)).catch(() => {});
    }
  }, [unitDetail?.id]); // eslint-disable-line react-hooks/exhaustive-deps

  // ── 动作 ──────────────────────────────────────────────────────────────────

  const triggerSummary = useCallback(
    (force: boolean) => {
      setSummaryStatus("streaming");
      setSummaryStream("");
      invoke("ku_generate_summary", { knowledgeUnitId: unitId, forceRegenerate: force }).catch(
        () => setSummaryStatus("idle")
      );
    },
    [unitId]
  );

  const triggerExplanation = useCallback(() => {
    setExplanationStatus("streaming");
    setExplanationExpanded(true);
    invoke("ku_generate_explanation", { knowledgeUnitId: unitId }).catch(
      () => setExplanationStatus("idle")
    );
  }, [unitId]);

  const triggerMirror = useCallback(() => {
    if (!noteText.trim()) return;
    setMirrorStatus("streaming");
    invoke("ku_validate_explanation", {
      knowledgeUnitId: unitId,
      userExplanation: noteText,
    }).catch(() => setMirrorStatus("idle"));
  }, [unitId, noteText]);

  // 完成镜子核对后：保存快照 + 更新复习调度
  useEffect(() => {
    if (mirrorStatus !== "done" || !mirrorFeedback || !unitDetail) return;

    // 计算新的 depthLevel
    const quality =
      (mirrorFeedback.coveredCount ?? 0) /
      Math.max((mirrorFeedback.coveredPoints?.length ?? 0) + (mirrorFeedback.additionalPerspectives?.length ?? 0), 1);

    const newDepth = Math.min(5, Math.max(1, (unitDetail.depthLevel ?? 1) + (quality >= 0.7 ? 1 : 0))) as 1 | 2 | 3 | 4 | 5;
    const reviewDays = calcNextReviewDays(newDepth, quality);
    const now = new Date().toISOString();
    const nextDue = addDays(now, reviewDays);

    // 保存快照
    createSnapshot({
      id: crypto.randomUUID(),
      knowledgeUnitId: unitId,
      userExplanation: noteText,
      mirrorCoveredCount: mirrorFeedback.coveredCount ?? 0,
      mirrorCoveredPoints: mirrorFeedback.coveredPoints ?? [],
      mirrorMissedAreas: (mirrorFeedback.additionalPerspectives ?? []).map(
        (p) => (typeof p === "string" ? p : p.text ?? "")
      ),
      depthLevelAtTime: newDepth,
      sourceAssetCountAtTime: unitDetail.sourceAssetIds.length,
      timestamp: now,
    });

    // 更新复习调度
    updateReviewSchedule(unitId, nextDue, newDepth);
    updateStatus(unitId, "validated");
  }, [mirrorStatus]); // eslint-disable-line react-hooks/exhaustive-deps

  // 笔记防抖自动保存
  const handleNoteChange = useCallback(
    (text: string) => {
      setNoteText(text);
      setNoteSaved(false);
      if (noteSaveTimer.current) clearTimeout(noteSaveTimer.current);
      noteSaveTimer.current = setTimeout(async () => {
        await updateNote(unitId, text);
        setNoteSaved(true);
        setTimeout(() => setNoteSaved(false), 2000);
      }, 1000);
    },
    [unitId, updateNote]
  );

  const markAsUnderstood = useCallback(() => {
    updateStatus(unitId, "understood");
    setExplanationExpanded(true);
  }, [unitId, updateStatus]);

  // ── 渲染 ──────────────────────────────────────────────────────────────────

  if (!unitDetail) {
    return (
      <div className="kup-loading">
        <Loader2 size={20} className="kup-spin" />
      </div>
    );
  }

  const status = unitDetail.status as KnowledgeStatus;
  const summaryText = summaryStatus === "streaming" ? summaryStream : (unitDetail.summary ?? summaryStream);
  const explanationReady = explanationStatus === "done" && parsedExplanation;

  return (
    <div className="kup-root">
      {/* 顶部导航 */}
      <div className="kup-topbar">
        <button className="kup-back-btn" onClick={onBack}>
          <ArrowLeft size={16} />
          <span>返回知识库</span>
        </button>
        <div className="kup-topbar-right">
          <span className={`kup-status-badge kup-status-${status}`}>
            {statusToIcon(status)} {STATUS_LABELS[status]}
          </span>
        </div>
      </div>

      {/* 来源声明横幅 */}
      <div className="kup-transparency-banner">
        <BookOpen size={12} />
        <span>以下内容基于你的文档生成</span>
      </div>

      {/* Step 8: 新素材过期警告 */}
      {isStale && (
        <div className="kup-staleness-banner">
          <AlertTriangle size={13} />
          <span>你添加了新素材，摘要可能不是最新的</span>
          <button
            className="kup-staleness-regen"
            onClick={() => {
              setIsStale(false);
              triggerSummary(true);
            }}
          >
            重新生成
          </button>
        </div>
      )}

      <div className="kup-scroll-area">
        {/* 知识标题 */}
        <div className="kup-hero">
          <h1 className="kup-title">{unitDetail.title}</h1>
          <p className="kup-core-insight">{unitDetail.coreInsight}</p>
          <div className="kup-hero-meta">
            {unitDetail.sourceAssetIds.length} 份素材
            {unitDetail.firstCapturedAt && (
              <> · {formatRelDate(unitDetail.firstCapturedAt)} 首次采集</>
            )}
            <span className="kup-depth-stars">
              {"★".repeat(unitDetail.depthLevel)}{"☆".repeat(5 - unitDetail.depthLevel)}
            </span>
          </div>
        </div>

        {/* ── 第一步：摘要 ────────────────────────────────────────────────────── */}
        <CollapsibleStep
          step={1}
          title="你的文档怎么说"
          done={!!unitDetail.summary}
          expanded={summaryExpanded}
          onToggle={() => setSummaryExpanded((v) => !v)}
        >
          {summaryStatus === "streaming" && (
            <div className="kup-streaming-indicator">
              <Loader2 size={13} className="kup-spin" />
              <span>生成中...</span>
            </div>
          )}
          {summaryText ? (
            <p className="kup-summary-text">{summaryText}</p>
          ) : summaryStatus === "idle" ? (
            <button className="kup-generate-btn" onClick={() => triggerSummary(false)}>
              <Sparkles size={14} />
              生成摘要
            </button>
          ) : null}
          {unitDetail.summary && (
            <button className="kup-regen-btn" onClick={() => triggerSummary(true)}>
              <RotateCcw size={11} />
              重新生成
            </button>
          )}
          {/* 读完后推进到下一步 */}
          {summaryText && status === "synthesized" && (
            <button className="kup-next-step-btn" onClick={markAsUnderstood}>
              读完了，下一步 →
            </button>
          )}
        </CollapsibleStep>

        {/* ── 第二步：理解框架 ──────────────────────────────────────────────── */}
        <CollapsibleStep
          step={2}
          title="理解框架"
          done={explanationStatus === "done"}
          expanded={explanationExpanded}
          onToggle={() => setExplanationExpanded((v) => !v)}
        >
          {explanationStatus === "idle" && (
            <button className="kup-generate-btn" onClick={triggerExplanation}>
              <Sparkles size={14} />
              生成理解框架
            </button>
          )}
          {explanationStatus === "streaming" && (
            <div className="kup-streaming-indicator">
              <Loader2 size={13} className="kup-spin" />
              <span>生成理解框架...</span>
            </div>
          )}
          {explanationReady && (
            <div className="kup-explanation">
              <div className="kup-expl-block">
                <div className="kup-expl-label">核心机制</div>
                <p className="kup-expl-text">{parsedExplanation.mechanism.text}</p>
                {parsedExplanation.mechanism.source && (
                  <span className="kup-source-tag">来源：{parsedExplanation.mechanism.source}</span>
                )}
              </div>
              {parsedExplanation.typicalScenarios.map((sc, i) => (
                <div key={i} className="kup-expl-block">
                  <div className="kup-expl-label">典型场景 {i + 1}</div>
                  <p className="kup-expl-text">{sc.text}</p>
                  {sc.source && <span className="kup-source-tag">来源：{sc.source}</span>}
                </div>
              ))}
              {parsedExplanation.commonMisconceptions && parsedExplanation.commonMisconceptions.length > 0 && (
                <div className="kup-expl-block kup-misconception">
                  <div className="kup-expl-label">常见误区</div>
                  {parsedExplanation.commonMisconceptions.map((m, i) => (
                    <p key={i} className="kup-expl-text">{m.text}</p>
                  ))}
                </div>
              )}
              <div className="kup-essence-box">
                <span className="kup-essence-label">一句话精华</span>
                <span className="kup-essence-text">{parsedExplanation.essenceSentence}</span>
              </div>
            </div>
          )}
        </CollapsibleStep>

        {/* ── 第三步：用你自己的话说 ──────────────────────────────────────── */}
        <CollapsibleStep
          step={3}
          title="用你自己的话说"
          done={!!unitDetail.userNote}
          expanded={noteExpanded}
          onToggle={() => setNoteExpanded((v) => !v)}
        >
          <textarea
            className="kup-note-textarea"
            placeholder="用你自己的话，解释这个知识单元..."
            value={noteText}
            onChange={(e) => handleNoteChange(e.target.value)}
            rows={5}
          />
          <div className="kup-note-footer">
            {noteSaved && <span className="kup-saved-hint">已保存</span>}
            {parsedExplanation?.essenceSentence && !noteText.trim() && (
              <button
                className="kup-starter-btn"
                onClick={() => handleNoteChange(parsedExplanation.essenceSentence)}
              >
                💡 给我一个出发点
              </button>
            )}
            {noteText.trim().length > 20 && (
              <button className="kup-mirror-btn" onClick={triggerMirror} disabled={mirrorStatus === "streaming"}>
                {mirrorStatus === "streaming" ? (
                  <><Loader2 size={13} className="kup-spin" /> 核对中...</>
                ) : (
                  <>和 AI 核对一下</>
                )}
              </button>
            )}
          </div>

          {/* 镜子反馈 */}
          {mirrorStatus === "done" && mirrorFeedback && (
            <MirrorFeedbackDisplay feedback={mirrorFeedback} />
          )}
        </CollapsibleStep>

        {/* ── 理解历史时间线 ────────────────────────────────────────────── */}
        {snapshots.length > 0 && (
          <CollapsibleStep
            step={4}
            title={`理解历史（${snapshots.length} 次核对）`}
            done={snapshots.length >= 2}
            expanded={historyExpanded}
            onToggle={() => setHistoryExpanded((v) => !v)}
          >
            <SnapshotTimeline snapshots={snapshots} />
          </CollapsibleStep>
        )}
      </div>

      {/* 推荐行动卡（固定底部） */}
      <ActionCardFooter
        status={status}
        nextReviewDue={unitDetail.nextReviewDue}
        hasSummary={!!unitDetail.summary}
        hasExplanation={explanationStatus === "done"}
        hasNote={!!noteText.trim()}
        onTriggerSummary={() => triggerSummary(false)}
        onExpandExplanation={triggerExplanation}
        onExpandNote={() => setNoteExpanded(true)}
        onMirror={triggerMirror}
      />
    </div>
  );
}

// ─── 折叠步骤组件 ─────────────────────────────────────────────────────────────

interface StepProps {
  step: number;
  title: string;
  done: boolean;
  expanded: boolean;
  onToggle: () => void;
  children: React.ReactNode;
}

function CollapsibleStep({ step, title, done, expanded, onToggle, children }: StepProps) {
  return (
    <div className={`kup-step ${done ? "kup-step-done" : ""}`}>
      <button className="kup-step-header" onClick={onToggle}>
        <span className="kup-step-num">{done ? "✓" : step}</span>
        <span className="kup-step-title">{title}</span>
        {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
      </button>
      {expanded && <div className="kup-step-body">{children}</div>}
    </div>
  );
}

// ─── 推荐行动卡底部 ───────────────────────────────────────────────────────────

interface ActionCardFooterProps {
  status: KnowledgeStatus;
  nextReviewDue: string | null;
  hasSummary: boolean;
  hasExplanation: boolean;
  hasNote: boolean;
  onTriggerSummary: () => void;
  onExpandExplanation: () => void;
  onExpandNote: () => void;
  onMirror: () => void;
}

function ActionCardFooter({
  status,
  nextReviewDue,
  hasSummary,
  hasExplanation,
  hasNote,
  onTriggerSummary,
  onExpandExplanation,
  onExpandNote,
  onMirror,
}: ActionCardFooterProps) {
  let label = "";
  let cta = "";
  let action: (() => void) | null = null;

  if (!hasSummary) {
    label = "还没有摘要，先看看你的文档怎么说？";
    cta = "生成摘要";
    action = onTriggerSummary;
  } else if (!hasExplanation) {
    label = "读完摘要了？生成理解框架帮你建立知识结构";
    cta = "生成理解框架 →";
    action = onExpandExplanation;
  } else if (!hasNote) {
    label = "框架有了，用自己的话说一遍？";
    cta = "写我的理解 →";
    action = onExpandNote;
  } else if (status !== "validated" && status !== "consolidated" && status !== "mastered") {
    label = "写完了，和 AI 核对一下看看还有哪些角度？";
    cta = "核对理解 →";
    action = onMirror;
  } else if (nextReviewDue && new Date(nextReviewDue) <= new Date()) {
    const daysSince = Math.round(
      (Date.now() - new Date(nextReviewDue).getTime()) / 86400000
    );
    label = `距上次复习已过 ${daysSince > 0 ? daysSince : 0} 天，重新核对一下？`;
    cta = "5 分钟巩固 →";
    action = onMirror;
  }

  if (!action) return null;

  return (
    <div className="kup-action-footer">
      <div className="kup-action-inner">
        <p className="kup-action-label">{label}</p>
        <button className="kup-action-cta" onClick={action}>
          {cta}
        </button>
      </div>
    </div>
  );
}

// ─── 镜子反馈展示 ─────────────────────────────────────────────────────────────

function MirrorFeedbackDisplay({ feedback }: { feedback: MirrorFeedbackResult }) {
  return (
    <div className="kup-mirror">
      <div className="kup-mirror-section">
        <div className="kup-mirror-label">✓ 你说到了这些要点</div>
        {feedback.coveredPoints.length > 0 ? (
          <ul className="kup-mirror-list">
            {feedback.coveredPoints.map((p, i) => (
              <li key={i} className="kup-mirror-covered">{p}</li>
            ))}
          </ul>
        ) : (
          <p className="kup-mirror-empty">暂未捕捉到明确要点</p>
        )}
      </div>
      {feedback.additionalPerspectives.length > 0 && (
        <div className="kup-mirror-section">
          <div className="kup-mirror-label">文档里还有这些角度</div>
          <ul className="kup-mirror-list">
            {feedback.additionalPerspectives.map((p, i) => (
              <li key={i} className="kup-mirror-add">
                {typeof p === "string" ? p : p.text}
                {typeof p !== "string" && p.source && (
                  <span className="kup-source-tag">{p.source}</span>
                )}
              </li>
            ))}
          </ul>
        </div>
      )}
      {feedback.differenceNote && (
        <p className="kup-mirror-diff">{feedback.differenceNote}</p>
      )}
    </div>
  );
}

// ─── 快照时间线 ───────────────────────────────────────────────────────────────

function SnapshotTimeline({ snapshots }: { snapshots: UnderstandingSnapshot[] }) {
  return (
    <div className="kup-timeline">
      {snapshots.map((snap, i) => (
        <div key={snap.id} className="kup-tl-item">
          <div className="kup-tl-dot" />
          {i < snapshots.length - 1 && <div className="kup-tl-line" />}
          <div className="kup-tl-content">
            <div className="kup-tl-meta">
              <span>{formatRelDate(snap.timestamp)}</span>
              <span className="kup-tl-depth">{"★".repeat(snap.depthLevelAtTime)}{"☆".repeat(5 - snap.depthLevelAtTime)}</span>
              <span>捕捉 {snap.mirrorCoveredCount} 个要点</span>
            </div>
            <p className="kup-tl-text">{truncate(snap.userExplanation, 120)}</p>
          </div>
        </div>
      ))}
      {snapshots.length >= 2 && (
        <p className="kup-tl-growth">↑ 你的理解在进化</p>
      )}
    </div>
  );
}

// ─── 工具 ─────────────────────────────────────────────────────────────────────

function truncate(s: string, max: number) {
  return s.length > max ? s.slice(0, max) + "…" : s;
}

function formatRelDate(iso: string): string {
  const diffDays = Math.floor((Date.now() - new Date(iso).getTime()) / 86400000);
  if (diffDays === 0) return "今天";
  if (diffDays === 1) return "昨天";
  if (diffDays < 7) return `${diffDays}天前`;
  if (diffDays < 30) return `${Math.floor(diffDays / 7)}周前`;
  return `${Math.floor(diffDays / 30)}个月前`;
}
