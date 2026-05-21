/**
 * SkillsView — 技能视图（Step 10）
 *
 * 展示当前知识库中的所有技能（能力域），以及每个技能的进度。
 * 宪章 K8：技能验证使用开放式场景题，不用选择题。
 *
 * 布局：
 *   左侧  技能列表（按进度排序）
 *   右侧  技能详情 + 挑战面板（SkillChallengePanel）
 */

import { useEffect, useState } from "react";
import {
  BrainCircuit,
  CheckCircle2,
  ChevronRight,
  Loader2,
  Plus,
  RefreshCw,
  Sparkles,
  Zap,
} from "lucide-react";
import type { Skill } from "../../../lib/tauri-commands";
import {
  skillAutoAggregate,
  skillComputeProgress,
  skillGetList,
} from "../../../lib/tauri-commands";
import { SkillChallengePanel } from "./SkillChallengePanel";
import { SkillMcpPanel } from "./SkillMcpPanel";
import "./SkillsView.css";

// ─── Props ────────────────────────────────────────────────────────────────────

interface Props {
  libraryId: string;
}

// ─── 状态颜色 ─────────────────────────────────────────────────────────────────

const STATUS_META: Record<string, { label: string; color: string }> = {
  learning:   { label: "学习中",  color: "var(--text-tertiary)" },
  practicing: { label: "练习中",  color: "var(--color-accent)" },
  verified:   { label: "已验证",  color: "var(--color-success)" },
};

// ─── 主组件 ───────────────────────────────────────────────────────────────────

export function SkillsView({ libraryId }: Props) {
  const [skills, setSkills] = useState<Skill[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isAggregating, setIsAggregating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = async () => {
    setIsLoading(true);
    setError(null);
    try {
      const list = await skillGetList(libraryId);
      setSkills(list);
      if (list.length > 0 && !selectedId) {
        setSelectedId(list[0].id);
      }
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

  const handleAutoAggregate = async () => {
    setIsAggregating(true);
    try {
      await skillAutoAggregate(libraryId);
      await load();
    } catch {
      // ignore
    } finally {
      setIsAggregating(false);
    }
  };

  const handleRefreshProgress = async (skillId: string) => {
    await skillComputeProgress(skillId);
    const updated = await skillGetList(libraryId);
    setSkills(updated);
  };

  const selectedSkill = skills.find((s) => s.id === selectedId) ?? null;

  return (
    <div className="skv-root">
      {/* 左侧列表 */}
      <div className="skv-sidebar">
        <div className="skv-toolbar">
          <span className="skv-toolbar-title">技能</span>
          <button
            className="skv-toolbar-btn"
            onClick={handleAutoAggregate}
            disabled={isAggregating}
            title="从知识单元自动聚合技能"
          >
            {isAggregating ? (
              <Loader2 size={14} className="skv-spin" />
            ) : (
              <Sparkles size={14} />
            )}
            自动聚合
          </button>
        </div>

        {isLoading && (
          <div className="skv-loading">
            <Loader2 size={18} className="skv-spin" />
          </div>
        )}

        {!isLoading && error && (
          <div className="skv-error">
            <p>{error}</p>
            <button className="skv-retry" onClick={load}>
              <RefreshCw size={13} /> 重试
            </button>
          </div>
        )}

        {!isLoading && !error && skills.length === 0 && (
          <div className="skv-empty">
            <BrainCircuit size={28} className="skv-empty-icon" />
            <p>还没有技能</p>
            <p className="skv-empty-sub">
              点击「自动聚合」从知识单元生成技能，<br />
              或从信号推断引擎跑完后自动触发
            </p>
          </div>
        )}

        {!isLoading && skills.map((skill) => (
          <SkillListItem
            key={skill.id}
            skill={skill}
            selected={selectedId === skill.id}
            onClick={() => setSelectedId(skill.id)}
          />
        ))}
      </div>

      {/* 右侧详情 */}
      <div className="skv-detail">
        {selectedSkill ? (
          <SkillDetail
            skill={selectedSkill}
            libraryId={libraryId}
            onProgressRefresh={() => handleRefreshProgress(selectedSkill.id)}
            onSkillUpdated={load}
          />
        ) : (
          <div className="skv-detail-empty">
            <Plus size={24} className="skv-detail-empty-icon" />
            <p>选择一个技能查看详情</p>
          </div>
        )}
      </div>
    </div>
  );
}

// ─── SkillListItem ────────────────────────────────────────────────────────────

interface ListItemProps {
  skill: Skill;
  selected: boolean;
  onClick: () => void;
}

function SkillListItem({ skill, selected, onClick }: ListItemProps) {
  const meta = STATUS_META[skill.status] ?? STATUS_META.learning;
  const pct = Math.round(skill.progress * 100);

  return (
    <button
      className={`skv-skill-item ${selected ? "skv-skill-selected" : ""}`}
      onClick={onClick}
    >
      <div className="skv-skill-header">
        <span className="skv-skill-name">{skill.name}</span>
        <span className="skv-skill-status" style={{ color: meta.color }}>
          {meta.label}
        </span>
      </div>
      <div className="skv-progress-bar">
        <div
          className="skv-progress-fill"
          style={{
            width: `${pct}%`,
            background: meta.color,
          }}
        />
      </div>
      <div className="skv-skill-meta">
        <span>{skill.kuIds.length} 个知识单元</span>
        <span>{pct}% 完成</span>
      </div>
    </button>
  );
}

// ─── SkillDetail ──────────────────────────────────────────────────────────────

interface DetailProps {
  skill: Skill;
  libraryId: string;
  onProgressRefresh: () => void;
  onSkillUpdated: () => void;
}

function SkillDetail({ skill, libraryId, onProgressRefresh, onSkillUpdated }: DetailProps) {
  const [showChallenge, setShowChallenge] = useState(false);
  const meta = STATUS_META[skill.status] ?? STATUS_META.learning;
  const pct = Math.round(skill.progress * 100);

  return (
    <div className="skv-detail-root">
      {/* 头部 */}
      <div className="skv-detail-header">
        <div className="skv-detail-status-row">
          <span className="skv-detail-status" style={{ color: meta.color }}>
            {skill.status === "verified" ? (
              <CheckCircle2 size={14} />
            ) : (
              <BrainCircuit size={14} />
            )}
            {meta.label}
          </span>
          <button
            className="skv-refresh-btn"
            onClick={onProgressRefresh}
            title="重新计算进度"
          >
            <RefreshCw size={12} />
          </button>
        </div>
        <h2 className="skv-detail-name">{skill.name}</h2>
        {skill.description && (
          <p className="skv-detail-desc">{skill.description}</p>
        )}

        {/* 进度条 */}
        <div className="skv-detail-progress-wrap">
          <div className="skv-detail-progress-bar">
            <div
              className="skv-detail-progress-fill"
              style={{ width: `${pct}%`, background: meta.color }}
            />
          </div>
          <span className="skv-detail-pct">{pct}%</span>
        </div>

        <div className="skv-detail-ku-count">
          {skill.kuIds.length} 个知识单元 ·{" "}
          {Math.round(skill.progress * skill.kuIds.length)} 个已核对
        </div>
      </div>

      {/* 验证区 */}
      {!showChallenge ? (
        <div className="skv-challenge-cta">
          {skill.status === "verified" ? (
            <div className="skv-verified-badge">
              <CheckCircle2 size={16} />
              <span>技能已验证</span>
              {skill.verifiedAt && (
                <span className="skv-verified-date">
                  {new Date(skill.verifiedAt).toLocaleDateString("zh-CN")}
                </span>
              )}
            </div>
          ) : skill.progress >= 0.5 ? (
            <div className="skv-challenge-ready">
              <p className="skv-challenge-hint">
                已核对超过一半的知识单元，可以开始技能验证了
              </p>
              <button
                className="skv-challenge-btn"
                onClick={() => setShowChallenge(true)}
              >
                <Zap size={14} />
                开始技能验证
                <ChevronRight size={14} />
              </button>
            </div>
          ) : (
            <p className="skv-challenge-hint skv-challenge-hint--wait">
              还需继续核对知识单元（当前 {pct}%），核对超过 50% 后可触发技能验证
            </p>
          )}
        </div>
      ) : (
        <SkillChallengePanel
          skill={skill}
          onClose={() => setShowChallenge(false)}
          onVerified={() => {
            setShowChallenge(false);
            onSkillUpdated();
          }}
        />
      )}

      {/* 上次评估结果 */}
      {skill.lastEvaluation && !showChallenge && (
        <LastEvaluationSummary evaluation={skill.lastEvaluation} />
      )}

      {/* MCP 导出（仅 verified 状态） */}
      {skill.status === "verified" && !showChallenge && (
        <SkillMcpPanel
          skillId={skill.id}
          skillName={skill.name}
          libraryId={libraryId}
        />
      )}
    </div>
  );
}

// ─── LastEvaluationSummary ────────────────────────────────────────────────────

function LastEvaluationSummary({ evaluation }: { evaluation: unknown }) {
  const ev = evaluation as {
    qualityScore: number;
    coveredPoints: string[];
    missedPoints: string[];
    feedback: string;
    evaluatedAt: string;
  };

  return (
    <div className="skv-last-eval">
      <div className="skv-last-eval-header">
        <span className="skv-last-eval-label">上次验证结果</span>
        <span className="skv-last-eval-score">
          {Math.round(ev.qualityScore * 100)}分
        </span>
      </div>
      <p className="skv-last-eval-feedback">{ev.feedback}</p>
      {ev.coveredPoints.length > 0 && (
        <ul className="skv-eval-points skv-eval-covered">
          {ev.coveredPoints.slice(0, 3).map((p, i) => (
            <li key={i}>✓ {p}</li>
          ))}
        </ul>
      )}
      {ev.missedPoints.length > 0 && (
        <ul className="skv-eval-points skv-eval-missed">
          {ev.missedPoints.slice(0, 2).map((p, i) => (
            <li key={i}>+ {p}</li>
          ))}
        </ul>
      )}
    </div>
  );
}
