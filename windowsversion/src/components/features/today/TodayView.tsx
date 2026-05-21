/**
 * TodayView — 今天视图（Step 7 → v2 Sidebar Redesign task_010）。
 *
 * v2 改造（task_010 / ADR-006 / PRD F-P1-4 + AC-16，PM §C 升 P0）：
 *   - 内部新增两个 Tab：「课程预习」(course-prep) / 「今日复习」(daily-review)。
 *   - Tab 初始策略走纯函数 {@link computeInitialTodayTab}（三态：首次 / 后续 / JustEnabled）。
 *   - 学习模式 OFF→ON 的瞬态信号 `_learningJustEnabled` 在 mount 时**消费一次**就写回 false；
 *     不进入 persist 白名单（瞬态语义）。
 *   - JustEnabled 路径强制 Tab=course-prep 但 **不写** todayLastTab（保留用户上次原值）。
 *   - 用户主动切 Tab 是**唯一**写入 `todayLastTab` 的路径。
 *   - 当前阶段：course-prep 复用既有"今日主卡 + 次要列表 + 统计行"的实现；daily-review 为占位 panel，
 *     具体业务逻辑后续 task 接入；Tab 切换走条件渲染，不用 display:none。
 */

import { useEffect, useMemo, useState } from "react";
import { BookOpen, ChevronRight, Loader2, RefreshCw, Zap } from "lucide-react";
import type { KnowledgeUnitSummary, KnowledgeStatus } from "../../../types/knowledge-units";
import type { TodayTab } from "../../../types";
import { statusToIcon, STATUS_LABELS } from "../../../types/knowledge-units";
import { kuGetList, kuGetDueForReview } from "../../../lib/tauri-commands";
import { useUIStore } from "../../../stores/uiStore";
import { computeInitialTodayTab } from "./initialTab";
import "./TodayView.css";

// ─── Props ────────────────────────────────────────────────────────────────────

interface Props {
  libraryId: string;
  onOpenUnit?: (unitId: string) => void;
}

// ─── 优先级计算 ───────────────────────────────────────────────────────────────

/** 越小越紧迫 */
function urgencyScore(unit: KnowledgeUnitSummary): number {
  const statusOrder: Record<KnowledgeStatus, number> = {
    raw: 5,
    synthesized: 4,
    understood: 3,
    articulated: 2,
    validated: 1,
    consolidated: 1,
    mastered: 99,
  };

  const base = statusOrder[unit.status as KnowledgeStatus] ?? 5;

  // 到期复习的额外加权（越久没复习越紧迫）
  if (unit.nextReviewDue) {
    const overdueDays = Math.floor(
      (Date.now() - new Date(unit.nextReviewDue).getTime()) / 86400000
    );
    if (overdueDays > 0) return base - overdueDays * 0.1; // 过期越久优先级越高
  }
  return base;
}

// ─── 组件 ─────────────────────────────────────────────────────────────────────

const TAB_LABELS: Record<TodayTab, string> = {
  "course-prep": "课程预习",
  "daily-review": "今日复习",
};

export function TodayView({ libraryId, onOpenUnit }: Props) {
  // ── Tab 初始状态：lazy init 内一次性算出，避免 SSR/二次 mount 抖动 ─────────
  // ADR-006：读 store snapshot（非 selector，避免后续 _learningJustEnabled 复位再触发重算）。
  const [currentTab, setCurrentTab] = useState<TodayTab>(() => {
    const { todayLastTab, _learningJustEnabled } = useUIStore.getState();
    return computeInitialTodayTab(todayLastTab, _learningJustEnabled);
  });

  // ── JustEnabled 信号消费（mount 时一次性写回 false） ───────────────────────
  // 此 effect 仅在挂载时跑一次：不写 todayLastTab（保留用户上次原值），
  // 仅当瞬态信号为 true 时才调用 setLearningJustEnabled(false)。
  useEffect(() => {
    if (useUIStore.getState()._learningJustEnabled) {
      useUIStore.getState().setLearningJustEnabled(false);
    }
    // mount-only：依赖空数组是有意为之，参考 ADR-006 + AC-3/AC-5。
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // ── 用户主动切 Tab：唯一写入 todayLastTab 的路径（AC-4） ──────────────────
  const handleTabChange = (next: TodayTab) => {
    if (next === currentTab) return;
    setCurrentTab(next);
    useUIStore.getState().setTodayLastTab(next);
  };

  return (
    <div className="tdv-root">
      {/* Tab 头部 */}
      <div className="tdv-tabs" role="tablist" aria-label="今日视图分区">
        {(Object.keys(TAB_LABELS) as TodayTab[]).map((tab) => {
          const active = tab === currentTab;
          return (
            <button
              key={tab}
              type="button"
              role="tab"
              aria-selected={active}
              className={`tdv-tab${active ? " tdv-tab--active" : ""}`}
              onClick={() => handleTabChange(tab)}
              data-testid={`tdv-tab-${tab}`}
            >
              {TAB_LABELS[tab]}
            </button>
          );
        })}
      </div>

      {/* 条件渲染（AC-2：禁止 display:none） */}
      {currentTab === "course-prep" && (
        <CoursePrepPanel libraryId={libraryId} onOpenUnit={onOpenUnit} />
      )}
      {currentTab === "daily-review" && <DailyReviewPanel />}
    </div>
  );
}

// ─── CoursePrepPanel：复用既有"今日主卡 + 次要列表 + 统计行"实现 ─────────────

function CoursePrepPanel({ libraryId, onOpenUnit }: Props) {
  const [allUnits, setAllUnits] = useState<KnowledgeUnitSummary[]>([]);
  const [dueUnits, setDueUnits] = useState<KnowledgeUnitSummary[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = async () => {
    setIsLoading(true);
    setError(null);
    try {
      const [all, due] = await Promise.all([
        kuGetList(libraryId),
        kuGetDueForReview(libraryId),
      ]);
      setAllUnits(all);
      setDueUnits(due);
    } catch (e) {
      setError(String(e));
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [libraryId]);

  // 合并"到期复习"和"待初次学习"，按紧迫度排序
  const prioritized = useMemo<KnowledgeUnitSummary[]>(() => {
    const dueIds = new Set(dueUnits.map(u => u.id));
    const notStarted = allUnits.filter(
      u => (u.status === "raw" || u.status === "synthesized") && !dueIds.has(u.id)
    );
    const combined = [...dueUnits, ...notStarted];
    return combined.sort((a, b) => urgencyScore(a) - urgencyScore(b));
  }, [allUnits, dueUnits]);

  const mainCard = prioritized[0] ?? null;
  const secondary = prioritized.slice(1, 8);

  const stats = useMemo(() => {
    const mastered = allUnits.filter(u => u.status === "mastered").length;
    const validated = allUnits.filter(
      u => u.status === "validated" || u.status === "consolidated"
    ).length;
    const total = allUnits.length;
    return { mastered, validated, total };
  }, [allUnits]);

  if (isLoading) {
    return (
      <div className="tdv-loading">
        <Loader2 size={20} className="tdv-spin" />
        <span>加载今日任务...</span>
      </div>
    );
  }

  if (error) {
    return (
      <div className="tdv-error">
        <p>加载失败：{error}</p>
        <button className="tdv-retry-btn" onClick={load}>
          <RefreshCw size={13} /> 重试
        </button>
      </div>
    );
  }

  const today = new Date().toLocaleDateString("zh-CN", {
    month: "long",
    day: "numeric",
    weekday: "long",
  });

  return (
    <div className="tdv-panel" data-testid="tdv-panel-course-prep">
      {/* 顶部问候（v1.3 task_010 ES-03：去除庆祝 emoji，文案中性陈述） */}
      <div className="tdv-header">
        <div className="tdv-date">{today}</div>
        <div className="tdv-headline">
          {prioritized.length === 0
            ? "今日无待处理"
            : `有 ${prioritized.length} 个知识单元等待你`}
        </div>
      </div>

      {/* 统计行（v1.3 task_010 ES-02：全 0 时整行不渲染） */}
      {(stats.total > 0 || stats.validated > 0 || stats.mastered > 0) && (
        <div className="tdv-stats-row" data-testid="tdv-stats-row">
          <div className="tdv-stat">
            <span className="tdv-stat-value">{stats.total}</span>
            <span className="tdv-stat-label">知识单元</span>
          </div>
          <div className="tdv-stat-divider" />
          <div className="tdv-stat">
            <span className="tdv-stat-value">{stats.validated}</span>
            <span className="tdv-stat-label">已核对</span>
          </div>
          <div className="tdv-stat-divider" />
          <div className="tdv-stat">
            <span className="tdv-stat-value">{stats.mastered}</span>
            <span className="tdv-stat-label">已掌握</span>
          </div>
        </div>
      )}

      <div className="tdv-content">
        {/* 主卡：最重要的一件事 */}
        {mainCard ? (
          <section className="tdv-section">
            <div className="tdv-section-label">
              <Zap size={13} className="tdv-zap" />
              今天最重要的一件事
            </div>
            <MainActionCard unit={mainCard} onOpen={onOpenUnit} />
          </section>
        ) : (
          <div className="tdv-empty" data-testid="tdv-empty">
            <BookOpen size={32} className="tdv-empty-icon" />
            <p>今日无待处理</p>
            <p className="tdv-empty-sub">导入素材后这里会自动生成任务</p>
          </div>
        )}

        {/* 次要列表 */}
        {secondary.length > 0 && (
          <section className="tdv-section">
            <div className="tdv-section-label">
              <BookOpen size={13} />
              其他待处理
            </div>
            <div className="tdv-secondary-list">
              {secondary.map(unit => (
                <SecondaryItem key={unit.id} unit={unit} onOpen={onOpenUnit} />
              ))}
            </div>
          </section>
        )}
      </div>
    </div>
  );
}

// ─── DailyReviewPanel：占位（具体业务逻辑由后续 task 接入） ───────────────────

function DailyReviewPanel() {
  return (
    <div className="tdv-panel tdv-panel--placeholder" data-testid="tdv-panel-daily-review">
      <div className="tdv-empty">
        <RefreshCw size={32} className="tdv-empty-icon" />
        <p>今日复习</p>
        <p className="tdv-empty-sub">复习清单将在后续版本中开放</p>
      </div>
    </div>
  );
}

// ─── MainActionCard ───────────────────────────────────────────────────────────

interface MainCardProps {
  unit: KnowledgeUnitSummary;
  onOpen?: (id: string) => void;
}

function MainActionCard({ unit, onOpen }: MainCardProps) {
  const status = unit.status as KnowledgeStatus;
  const { label, cta } = getActionText(unit);
  const isOverdue = unit.nextReviewDue
    ? new Date(unit.nextReviewDue) <= new Date()
    : false;

  return (
    <div className={`tdv-main-card ${isOverdue ? "tdv-main-card--overdue" : ""}`}>
      <div className="tdv-main-card-header">
        <span className={`tdv-status-icon tdv-status-${status}`}>
          {statusToIcon(status)}
        </span>
        <span className="tdv-status-label">{STATUS_LABELS[status]}</span>
        {isOverdue && <span className="tdv-overdue-badge">到期复习</span>}
        <span className="tdv-depth-stars">
          {"★".repeat(unit.depthLevel)}{"☆".repeat(5 - unit.depthLevel)}
        </span>
      </div>

      <h2 className="tdv-main-title">{unit.title}</h2>
      <p className="tdv-main-insight">{unit.coreInsight}</p>

      <div className="tdv-main-footer">
        <span className="tdv-main-meta">
          {unit.sourceAssetCount} 份素材 · {unit.snapshotCount > 0 ? `${unit.snapshotCount} 次核对` : "未核对"}
        </span>
        <button className="tdv-main-cta" onClick={() => onOpen?.(unit.id)}>
          {cta}
          <ChevronRight size={15} />
        </button>
      </div>

      {label && <p className="tdv-main-label">{label}</p>}
    </div>
  );
}

// ─── SecondaryItem ────────────────────────────────────────────────────────────

interface SecondaryItemProps {
  unit: KnowledgeUnitSummary;
  onOpen?: (id: string) => void;
}

function SecondaryItem({ unit, onOpen }: SecondaryItemProps) {
  const status = unit.status as KnowledgeStatus;
  const { cta } = getActionText(unit);
  const isOverdue = unit.nextReviewDue
    ? new Date(unit.nextReviewDue) <= new Date()
    : false;

  return (
    <button className="tdv-secondary-item" onClick={() => onOpen?.(unit.id)}>
      <span className={`tdv-si-icon tdv-status-${status}`}>{statusToIcon(status)}</span>
      <div className="tdv-si-body">
        <span className="tdv-si-title">{unit.title}</span>
        {isOverdue && <span className="tdv-si-overdue">到期</span>}
      </div>
      <span className="tdv-si-cta">{cta}</span>
      <ChevronRight size={13} className="tdv-si-arrow" />
    </button>
  );
}

// ─── 工具 ─────────────────────────────────────────────────────────────────────

interface ActionText {
  label: string;
  cta: string;
}

function getActionText(unit: KnowledgeUnitSummary): ActionText {
  const isOverdue =
    unit.nextReviewDue ? new Date(unit.nextReviewDue) <= new Date() : false;

  if (isOverdue) {
    const overdueDays = Math.floor(
      (Date.now() - new Date(unit.nextReviewDue!).getTime()) / 86400000
    );
    return {
      label: `已过期 ${overdueDays} 天，快到遗忘临界点了`,
      cta: "5 分钟巩固",
    };
  }

  switch (unit.status as KnowledgeStatus) {
    case "raw":
      return { label: "刚采集的知识，还没有摘要", cta: "生成摘要" };
    case "synthesized":
      return { label: "摘要已生成，读一读了解一下", cta: "了解这个知识" };
    case "understood":
      return { label: "读了摘要？试着用自己的话说说看", cta: "写下我的理解" };
    case "articulated":
      return { label: "写了理解，和 AI 核对一下？", cta: "核对理解" };
    case "validated":
    case "consolidated":
      return { label: "定期复习巩固记忆", cta: "复习" };
    case "mastered":
      return { label: "", cta: "回顾" };
    default:
      return { label: "", cta: "学习" };
  }
}
