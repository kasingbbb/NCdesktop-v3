/**
 * KnowledgeUnitDetailPanel — 知识单元右侧面板（知识库视图内嵌）
 *
 * 展示选中知识单元的摘要信息，并提供跳转到完整学习页的入口。
 * 完整渐进式学习体验在 KnowledgeUnitPage 中（Step 6 实现）。
 *
 * 约束（宪章 K3）：每个状态只展示一个推荐下一步动作。
 */

import { useEffect } from "react";
import { BookOpen, ChevronRight, Loader2, RotateCcw } from "lucide-react";
import { useKnowledgeUnitsStore } from "../../../stores/knowledgeUnitsStore";
import {
  statusToIcon,
  STATUS_LABELS,
  type KnowledgeStatus,
} from "../../../types/knowledge-units";
import "./KnowledgeUnitDetailPanel.css";

interface Props {
  unitId: string;
  libraryId: string;
  onOpenFullPage?: (unitId: string) => void;
}

export function KnowledgeUnitDetailPanel({ unitId, onOpenFullPage }: Props) {
  const { unitDetail, snapshots, isLoadingDetail, loadDetail, loadSnapshots } =
    useKnowledgeUnitsStore();

  useEffect(() => {
    loadDetail(unitId);
    loadSnapshots(unitId);
  }, [unitId, loadDetail, loadSnapshots]);

  if (isLoadingDetail || !unitDetail) {
    return (
      <div className="kudp-loading">
        <Loader2 size={20} className="kudp-spin" />
        <span>加载中...</span>
      </div>
    );
  }

  const status = unitDetail.status as KnowledgeStatus;
  const actionCard = buildActionCard(status, unitDetail.nextReviewDue);

  return (
    <div className="kudp-root">
      {/* 头部 */}
      <div className="kudp-header">
        <div className="kudp-status-row">
          <span className={`kudp-status-icon kudp-status-${status}`}>
            {statusToIcon(status)}
          </span>
          <span className="kudp-status-label">{STATUS_LABELS[status]}</span>
          <span className="kudp-depth">
            {"★".repeat(unitDetail.depthLevel)}{"☆".repeat(5 - unitDetail.depthLevel)}
          </span>
        </div>
        <h2 className="kudp-title">{unitDetail.title}</h2>
        <p className="kudp-insight">{unitDetail.coreInsight}</p>
        <div className="kudp-meta">
          {unitDetail.sourceAssetIds.length} 份素材
          {unitDetail.firstCapturedAt && (
            <> · {formatRelativeDate(unitDetail.firstCapturedAt)} 首学</>
          )}
        </div>
      </div>

      {/* 推荐行动卡（宪章 K3：只有一个主操作） */}
      {actionCard && (
        <div className="kudp-action-card">
          <p className="kudp-action-label">{actionCard.label}</p>
          <button
            className="kudp-action-btn"
            onClick={() => onOpenFullPage?.(unitId)}
          >
            {actionCard.cta}
            <ChevronRight size={14} />
          </button>
        </div>
      )}

      {/* 摘要预览 */}
      {unitDetail.summary && (
        <div className="kudp-section">
          <div className="kudp-section-title">
            <BookOpen size={13} />
            <span>你的文档怎么说</span>
          </div>
          <p className="kudp-summary-text">{truncate(unitDetail.summary, 300)}</p>
          {unitDetail.summary.length > 300 && (
            <button
              className="kudp-read-more"
              onClick={() => onOpenFullPage?.(unitId)}
            >
              阅读全文 →
            </button>
          )}
        </div>
      )}

      {/* 用户笔记预览 */}
      {unitDetail.userNote && (
        <div className="kudp-section">
          <div className="kudp-section-title">
            <span>📝</span>
            <span>我的理解</span>
          </div>
          <p className="kudp-note-text">{truncate(unitDetail.userNote, 200)}</p>
        </div>
      )}

      {/* 理解历史（有快照时展示） */}
      {snapshots.length >= 2 && (
        <div className="kudp-section">
          <div className="kudp-section-title">
            <RotateCcw size={13} />
            <span>理解在进化</span>
          </div>
          <div className="kudp-snapshots">
            {snapshots.slice(-3).map((snap) => (
              <div key={snap.id} className="kudp-snapshot-item">
                <span className="kudp-snap-time">
                  {formatRelativeDate(snap.timestamp)}
                </span>
                <span className="kudp-snap-depth">
                  {"★".repeat(snap.depthLevelAtTime)}{"☆".repeat(5 - snap.depthLevelAtTime)}
                </span>
                <span className="kudp-snap-covered">
                  捕捉到 {snap.mirrorCoveredCount} 个要点
                </span>
              </div>
            ))}
          </div>
          <p className="kudp-evolution-hint">↑ 你的理解在深化</p>
        </div>
      )}

      {/* 深入学习入口 */}
      <div className="kudp-footer">
        <button
          className="kudp-deep-btn"
          onClick={() => onOpenFullPage?.(unitId)}
        >
          <BookOpen size={14} />
          深入学习
          <ChevronRight size={14} />
        </button>
      </div>
    </div>
  );
}

// ─── 工具函数 ─────────────────────────────────────────────────────────────────

interface ActionCard {
  label: string;
  cta: string;
}

function buildActionCard(
  status: KnowledgeStatus,
  nextReviewDue: string | null
): ActionCard | null {
  switch (status) {
    case "raw":
      return { label: "这个知识刚刚采集，还没有摘要", cta: "生成摘要" };
    case "synthesized":
      return { label: "摘要已生成，读一读了解一下", cta: "了解这个知识 →" };
    case "understood":
      return { label: "读了摘要？试着用自己的话说说看", cta: "写下我的理解 →" };
    case "articulated":
      return { label: "写了理解，和 AI 核对一下？", cta: "核对理解 →" };
    case "validated":
    case "consolidated":
      if (nextReviewDue && new Date(nextReviewDue) <= new Date()) {
        const days = Math.round(
          (new Date().getTime() - new Date(nextReviewDue).getTime()) /
            (1000 * 60 * 60 * 24)
        );
        return {
          label: `距上次复习已 ${days > 0 ? days : 0} 天，快到遗忘临界点了`,
          cta: "5 分钟巩固 →",
        };
      }
      return null;
    case "mastered":
      return null;
    default:
      return null;
  }
}

function truncate(text: string, max: number): string {
  return text.length > max ? text.slice(0, max) + "…" : text;
}

function formatRelativeDate(iso: string): string {
  const date = new Date(iso);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));
  if (diffDays === 0) return "今天";
  if (diffDays === 1) return "昨天";
  if (diffDays < 7) return `${diffDays}天前`;
  if (diffDays < 30) return `${Math.floor(diffDays / 7)}周前`;
  if (diffDays < 365) return `${Math.floor(diffDays / 30)}个月前`;
  return `${Math.floor(diffDays / 365)}年前`;
}
