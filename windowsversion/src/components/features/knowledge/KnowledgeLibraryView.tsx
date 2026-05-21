/**
 * KnowledgeLibraryView — 知识库主视图（v5 知识进化系统）
 *
 * 布局：
 *   左侧  知识单元列表（按需要行动排序，课程分组，五级状态图标）
 *   右侧  KnowledgeUnitDetailPanel（渐进式交互详情）
 *
 * 约束（宪章 A1/A2/K1/K3）：
 *   - 用户看到的主要单元是知识单元，不是概念
 *   - 每个知识单元有清晰的状态标识
 */

import { useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { Search, RefreshCw, BrainCircuit, Loader2, Network, List, ScanSearch, AlertCircle, X } from "lucide-react";
import { useKnowledgeUnitsStore } from "../../../stores/knowledgeUnitsStore";
import { useLibraryStore } from "../../../stores/libraryStore";
import { useProjectStore } from "../../../stores/projectStore";
import { useExtractionStore } from "../../../stores/extractionStore";
import { KnowledgeUnitDetailPanel } from "./KnowledgeUnitDetailPanel";
import { KnowledgeGraphView } from "./KnowledgeGraphView";
import { statusToIcon, STATUS_LABELS, type KnowledgeStatus, type KnowledgeUnitSummary } from "../../../types/knowledge-units";
import type { SynthesisProgress } from "../../../lib/tauri-commands";
import "./KnowledgeLibraryView.css";

// ─── 主组件 ───────────────────────────────────────────────────────────────────

export function KnowledgeLibraryView() {
  const libraryId = useLibraryStore((s) => s.activeLibraryId) ?? "";
  const projectId = useProjectStore((s) => s.activeProjectId) ?? "";
  const { isExtracting, extractProjectAssets } = useExtractionStore();

  const {
    units,
    selectedUnitId,
    synthesisStage,
    synthesisGroupsFound,
    synthesisUnitsWritten,
    searchQuery,
    isLoading,
    error,
    fetchUnits,
    selectUnit,
    startSynthesis,
    setSearchQuery,
    setSynthesisProgress,
    getFilteredUnits,
  } = useKnowledgeUnitsStore();

  const unlistenRef = useRef<UnlistenFn | null>(null);
  const [viewMode, setViewMode] = useState<"list" | "graph">("list");
  const [errorDismissed, setErrorDismissed] = useState(false);

  // 每次新合成开始时重置 dismiss 状态
  useEffect(() => {
    if (synthesisStage === "clustering") setErrorDismissed(false);
  }, [synthesisStage]);

  // 挂载时拉取知识单元列表
  useEffect(() => {
    if (libraryId) fetchUnits(libraryId);
  }, [libraryId, fetchUnits]);

  // 监听合成进度事件
  useEffect(() => {
    let unlisten: UnlistenFn | null = null;
    listen<SynthesisProgress>("notecapt/knowledge-synthesis-progress", (event) => {
      const { stage, groupsFound, unitsWritten } = event.payload;
      setSynthesisProgress(stage, groupsFound, unitsWritten);
      if (stage === "completed" || stage === "error") {
        fetchUnits(libraryId);
      }
    }).then((fn) => {
      unlisten = fn;
      unlistenRef.current = fn;
    });
    return () => {
      unlisten?.();
    };
  }, [libraryId, fetchUnits, setSynthesisProgress]);

  const filtered = getFilteredUnits();
  const grouped = groupByInferredCourse(filtered);

  const isSynthesizing = synthesisStage !== null && synthesisStage !== "completed" && synthesisStage !== "error";

  return (
    <div className="kl-root">
      {/* 左侧：知识单元列表 */}
      <div className="kl-sidebar">
        {/* 头部工具栏 */}
        <div className="kl-toolbar">
          <div className="kl-search-wrap">
            <Search size={14} className="kl-search-icon" />
            <input
              className="kl-search-input"
              placeholder="搜索知识..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
            />
          </div>
          {/* 列表 / 图谱切换 */}
          <div className="kl-view-toggle">
            <button
              className={`kl-toggle-btn ${viewMode === "list" ? "kl-toggle-active" : ""}`}
              title="列表视图"
              onClick={() => setViewMode("list")}
            >
              <List size={14} />
            </button>
            <button
              className={`kl-toggle-btn ${viewMode === "graph" ? "kl-toggle-active" : ""}`}
              title="知识图谱"
              onClick={() => setViewMode("graph")}
            >
              <Network size={14} />
            </button>
          </div>
          <button
            className="kl-btn-icon"
            title="扫描提取素材"
            onClick={() => extractProjectAssets(projectId)}
            disabled={isExtracting || isSynthesizing || !projectId}
          >
            {isExtracting ? <Loader2 size={15} className="kl-spin" /> : <ScanSearch size={15} />}
          </button>
          <button
            className="kl-btn-icon"
            title="重新合成知识单元"
            onClick={() => startSynthesis(libraryId, true)}
            disabled={isSynthesizing || isExtracting}
          >
            {isSynthesizing ? <Loader2 size={15} className="kl-spin" /> : <RefreshCw size={15} />}
          </button>
        </div>

        {/* 合成进度横幅 */}
        {isSynthesizing && (
          <div className="kl-synthesis-banner">
            <BrainCircuit size={13} />
            <span>
              {synthesisStage === "clustering"
                ? "正在归纳主题群..."
                : `命名知识单元 ${synthesisUnitsWritten}/${synthesisGroupsFound}...`}
            </span>
          </div>
        )}

        {/* 合成错误横幅 */}
        {synthesisStage === "error" && error && !errorDismissed && (
          <div className="kl-error-banner">
            <AlertCircle size={13} className="kl-error-icon" />
            <span className="kl-error-text">{error}</span>
            <button className="kl-error-dismiss" onClick={() => setErrorDismissed(true)}>
              <X size={12} />
            </button>
          </div>
        )}

        {/* 空状态 */}
        {!isLoading && units.length === 0 && (
          <div className="kl-empty">
            <BrainCircuit size={32} className="kl-empty-icon" />
            <p>尚无知识单元</p>
            <p className="kl-empty-sub">点击上方扫描按钮提取素材，再点刷新合成知识单元</p>
          </div>
        )}

        {/* 加载中 */}
        {isLoading && (
          <div className="kl-loading">
            <Loader2 size={20} className="kl-spin" />
          </div>
        )}

        {/* 分组列表 */}
        {!isLoading && Object.entries(grouped).map(([course, courseUnits]) => (
          <div key={course} className="kl-group">
            <div className="kl-group-header">
              <span className="kl-group-name">{course}</span>
              <span className="kl-group-count">{courseUnits.length}</span>
            </div>
            {courseUnits.map((unit) => (
              <KnowledgeUnitItem
                key={unit.id}
                unit={unit}
                selected={selectedUnitId === unit.id}
                onClick={() => selectUnit(unit.id)}
              />
            ))}
          </div>
        ))}
      </div>

      {/* 右侧：详情面板 or 图谱 */}
      <div className="kl-detail">
        {viewMode === "graph" ? (
          <KnowledgeGraphView
            libraryId={libraryId}
            onNodeClick={(id) => {
              selectUnit(id);
              setViewMode("list");
            }}
          />
        ) : selectedUnitId ? (
          <KnowledgeUnitDetailPanel unitId={selectedUnitId} libraryId={libraryId} />
        ) : (
          <div className="kl-detail-empty">
            <p>选择一个知识单元开始学习</p>
          </div>
        )}
      </div>
    </div>
  );
}

// ─── 知识单元列表项 ────────────────────────────────────────────────────────────

interface KuItemProps {
  unit: KnowledgeUnitSummary;
  selected: boolean;
  onClick: () => void;
}

function KnowledgeUnitItem({ unit, selected, onClick }: KuItemProps) {
  const hasAction = needsAction(unit.status as KnowledgeStatus);
  return (
    <button
      className={`kl-unit-item ${selected ? "kl-unit-selected" : ""} ${hasAction ? "kl-unit-actionable" : ""}`}
      onClick={onClick}
    >
      <span className={`kl-status-icon kl-status-${unit.status}`} title={STATUS_LABELS[unit.status as KnowledgeStatus]}>
        {statusToIcon(unit.status as KnowledgeStatus)}
      </span>
      <span className="kl-unit-content">
        <span className="kl-unit-title">{unit.title}</span>
        <span className="kl-unit-meta">
          {unit.sourceAssetCount} 份素材
          {unit.nextReviewDue && isOverdue(unit.nextReviewDue) && (
            <span className="kl-overdue">· 待复习</span>
          )}
        </span>
      </span>
      {hasAction && <span className="kl-unit-arrow">→</span>}
    </button>
  );
}

// ─── 工具函数 ─────────────────────────────────────────────────────────────────

function needsAction(status: KnowledgeStatus): boolean {
  return status === "synthesized" || status === "understood" || status === "articulated";
}

function isOverdue(nextReviewDue: string): boolean {
  return new Date(nextReviewDue) <= new Date();
}

function groupByInferredCourse(
  units: KnowledgeUnitSummary[]
): Record<string, KnowledgeUnitSummary[]> {
  // 暂时不从 inference 查询课程，先用"全部"分组
  // TODO Step 3 完成信号推断引擎后，改为从 asset_inferences 读取 inferred_course
  const result: Record<string, KnowledgeUnitSummary[]> = {};

  // 按状态排序：待行动 → 待复习 → 已完成
  const sorted = [...units].sort((a, b) => {
    const order: Record<string, number> = {
      synthesized: 0,
      raw: 1,
      understood: 2,
      articulated: 3,
      validated: 4,
      consolidated: 5,
      mastered: 6,
    };
    return (order[a.status] ?? 9) - (order[b.status] ?? 9);
  });

  for (const unit of sorted) {
    const course = "全部知识"; // placeholder until Step 3
    if (!result[course]) result[course] = [];
    result[course].push(unit);
  }

  return result;
}
