/**
 * KnowledgeGraphView — 知识图谱（Step 9）
 *
 * 纯 Canvas + Verlet 物理仿真，无第三方图形库依赖。
 *
 * 视觉规则：
 *   - 节点圆半径 = 8 + depthLevel × 4（掌握越深越大）
 *   - 节点颜色   = 状态色（○→●）
 *   - 边线型     = solid（同域）/ dashed（跨域）
 *   - 边粗细     = weight × 2
 *   - 相同 inferredCourse 的节点被轻量分组（相同色调晕圈）
 *
 * 交互：
 *   - 拖拽节点（单指）
 *   - 滚轮/双指缩放
 *   - 平移（拖拽背景）
 *   - 悬浮：展示 tooltip（title + coreInsight）
 *   - 单击节点：onNodeClick(id)
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { Loader2, RefreshCw, ZoomIn, ZoomOut, Maximize2 } from "lucide-react";
import type { GraphNode, GraphEdge, KnowledgeGraphData } from "../../../lib/tauri-commands";
import { getKnowledgeGraph } from "../../../lib/tauri-commands";
import "./KnowledgeGraphView.css";

// ─── Props ────────────────────────────────────────────────────────────────────

interface Props {
  libraryId: string;
  onNodeClick?: (nodeId: string) => void;
}

// ─── 物理仿真节点 ─────────────────────────────────────────────────────────────

interface SimNode extends GraphNode {
  x: number;
  y: number;
  vx: number;
  vy: number;
  radius: number;
  color: string;
  pinned: boolean;
}

// ─── 颜色常量 ─────────────────────────────────────────────────────────────────

const STATUS_COLORS: Record<string, string> = {
  raw:         "#94a3b8",
  synthesized: "#f59e0b",
  understood:  "#3b82f6",
  articulated: "#8b5cf6",
  validated:   "#10b981",
  consolidated:"#059669",
  mastered:    "#f59e0b",
};

const COURSE_PALETTE = [
  "rgba(59,130,246,0.08)",
  "rgba(16,185,129,0.08)",
  "rgba(245,158,11,0.08)",
  "rgba(139,92,246,0.08)",
  "rgba(239,68,68,0.08)",
  "rgba(236,72,153,0.08)",
];

// ─── 物理常量 ─────────────────────────────────────────────────────────────────
const REPULSION    = 3500;  // 节点间斥力强度
const SPRING_K     = 0.04;  // 边弹簧系数
const SPRING_LEN   = 160;   // 边自然长度（px）
const DAMPING      = 0.82;  // 速度衰减
const CENTER_PULL  = 0.008; // 向中心的吸引力
const TICK_MS      = 16;    // 每帧间隔

// ─── 主组件 ───────────────────────────────────────────────────────────────────

export function KnowledgeGraphView({ libraryId, onNodeClick }: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const simRef = useRef<{
    nodes: SimNode[];
    edges: GraphEdge[];
    courseColors: Map<string, string>;
    running: boolean;
    tickTimer: ReturnType<typeof setInterval> | null;
  }>({ nodes: [], edges: [], courseColors: new Map(), running: false, tickTimer: null });

  // viewport transform
  const viewRef = useRef({ scale: 1, tx: 0, ty: 0 });
  // interaction state
  const dragRef = useRef<{
    type: "node" | "pan" | null;
    nodeIdx: number;
    startX: number;
    startY: number;
    lastTx: number;
    lastTy: number;
  }>({ type: null, nodeIdx: -1, startX: 0, startY: 0, lastTx: 0, lastTy: 0 });

  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [tooltip, setTooltip] = useState<{ x: number; y: number; node: SimNode } | null>(null);
  const [nodeCount, setNodeCount] = useState(0);

  // ── 加载数据 + 初始化仿真 ───────────────────────────────────────────────────
  const initGraph = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    setTooltip(null);

    try {
      const data: KnowledgeGraphData = await getKnowledgeGraph(libraryId);
      const canvas = canvasRef.current;
      const container = containerRef.current;
      if (!canvas || !container) return;

      const W = container.clientWidth || 800;
      const H = container.clientHeight || 600;

      // 收集课程→颜色映射
      const courses = Array.from(new Set(data.nodes.map(n => n.inferredCourse).filter(Boolean))) as string[];
      const courseColors = new Map<string, string>();
      courses.forEach((c, i) => courseColors.set(c, COURSE_PALETTE[i % COURSE_PALETTE.length]));

      // 初始化节点，随机散布在中心区域
      const nodes: SimNode[] = data.nodes.map((n, i) => {
        const angle = (i / data.nodes.length) * 2 * Math.PI;
        const r = 80 + Math.random() * 120;
        return {
          ...n,
          x: W / 2 + Math.cos(angle) * r,
          y: H / 2 + Math.sin(angle) * r,
          vx: (Math.random() - 0.5) * 2,
          vy: (Math.random() - 0.5) * 2,
          radius: 8 + n.depthLevel * 4,
          color: STATUS_COLORS[n.status] ?? "#94a3b8",
          pinned: false,
        };
      });

      // 重置视口
      viewRef.current = { scale: 1, tx: 0, ty: 0 };

      // 停止旧仿真
      const sim = simRef.current;
      if (sim.tickTimer) clearInterval(sim.tickTimer);
      sim.nodes = nodes;
      sim.edges = data.edges;
      sim.courseColors = courseColors;
      sim.running = true;
      setNodeCount(nodes.length);

      // 启动物理仿真
      sim.tickTimer = setInterval(() => tick(), TICK_MS);
    } catch (e) {
      setError(String(e));
    } finally {
      setIsLoading(false);
    }
  }, [libraryId]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    initGraph();
    return () => {
      const sim = simRef.current;
      if (sim.tickTimer) clearInterval(sim.tickTimer);
    };
  }, [initGraph]);

  // ── 物理仿真 tick ──────────────────────────────────────────────────────────
  const tick = useCallback(() => {
    const sim = simRef.current;
    if (!sim.running || sim.nodes.length === 0) return;

    const canvas = canvasRef.current;
    if (!canvas) return;
    const W = canvas.width;
    const H = canvas.height;

    // 计算力
    const forces: { fx: number; fy: number }[] = sim.nodes.map(() => ({ fx: 0, fy: 0 }));
    const nodeIdx = new Map(sim.nodes.map((n, i) => [n.id, i]));

    // 斥力（每对节点）
    for (let i = 0; i < sim.nodes.length; i++) {
      for (let j = i + 1; j < sim.nodes.length; j++) {
        const a = sim.nodes[i], b = sim.nodes[j];
        const dx = b.x - a.x;
        const dy = b.y - a.y;
        const dist2 = dx * dx + dy * dy + 1;
        const f = REPULSION / dist2;
        const fx = f * dx / Math.sqrt(dist2);
        const fy = f * dy / Math.sqrt(dist2);
        forces[i].fx -= fx;
        forces[i].fy -= fy;
        forces[j].fx += fx;
        forces[j].fy += fy;
      }
    }

    // 弹簧吸引力（边）
    for (const edge of sim.edges) {
      const si = nodeIdx.get(edge.source);
      const ti = nodeIdx.get(edge.target);
      if (si === undefined || ti === undefined) continue;
      const a = sim.nodes[si], b = sim.nodes[ti];
      const dx = b.x - a.x;
      const dy = b.y - a.y;
      const dist = Math.sqrt(dx * dx + dy * dy) + 0.01;
      const idealLen = SPRING_LEN / (edge.weight + 0.3);
      const f = SPRING_K * (dist - idealLen);
      const fx = f * dx / dist;
      const fy = f * dy / dist;
      forces[si].fx += fx;
      forces[si].fy += fy;
      forces[ti].fx -= fx;
      forces[ti].fy -= fy;
    }

    // 向中心拉力
    const cx = W / 2, cy = H / 2;
    for (let i = 0; i < sim.nodes.length; i++) {
      forces[i].fx += CENTER_PULL * (cx - sim.nodes[i].x);
      forces[i].fy += CENTER_PULL * (cy - sim.nodes[i].y);
    }

    // 积分 + 衰减
    for (let i = 0; i < sim.nodes.length; i++) {
      const n = sim.nodes[i];
      if (n.pinned) continue;
      n.vx = (n.vx + forces[i].fx) * DAMPING;
      n.vy = (n.vy + forces[i].fy) * DAMPING;
      n.x += n.vx;
      n.y += n.vy;
      // 边界软约束
      n.x = Math.max(n.radius, Math.min(W - n.radius, n.x));
      n.y = Math.max(n.radius, Math.min(H - n.radius, n.y));
    }

    render();
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // ── 渲染 ───────────────────────────────────────────────────────────────────
  const render = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const { scale, tx, ty } = viewRef.current;
    const sim = simRef.current;

    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.save();
    ctx.translate(tx, ty);
    ctx.scale(scale, scale);

    // 1. 课程分组晕圈
    const courseGroups = new Map<string, SimNode[]>();
    for (const n of sim.nodes) {
      if (n.inferredCourse) {
        const g = courseGroups.get(n.inferredCourse) ?? [];
        g.push(n);
        courseGroups.set(n.inferredCourse, g);
      }
    }
    for (const [course, members] of courseGroups) {
      if (members.length < 2) continue;
      const cx = members.reduce((s, n) => s + n.x, 0) / members.length;
      const cy = members.reduce((s, n) => s + n.y, 0) / members.length;
      let maxR = 0;
      for (const n of members) {
        const d = Math.sqrt((n.x - cx) ** 2 + (n.y - cy) ** 2) + n.radius + 20;
        if (d > maxR) maxR = d;
      }
      const grad = ctx.createRadialGradient(cx, cy, 0, cx, cy, maxR);
      const color = sim.courseColors.get(course) ?? "rgba(0,0,0,0.04)";
      grad.addColorStop(0, color);
      grad.addColorStop(1, "rgba(0,0,0,0)");
      ctx.beginPath();
      ctx.arc(cx, cy, maxR, 0, Math.PI * 2);
      ctx.fillStyle = grad;
      ctx.fill();
    }

    // 2. 边
    for (const edge of sim.edges) {
      const si = sim.nodes.findIndex(n => n.id === edge.source);
      const ti = sim.nodes.findIndex(n => n.id === edge.target);
      if (si < 0 || ti < 0) continue;
      const a = sim.nodes[si], b = sim.nodes[ti];

      ctx.beginPath();
      ctx.moveTo(a.x, a.y);
      ctx.lineTo(b.x, b.y);
      ctx.lineWidth = edge.weight * 2;

      if (edge.isCrossDomain) {
        ctx.setLineDash([6, 4]);
        ctx.strokeStyle = "rgba(148,163,184,0.5)";
      } else if (edge.edgeType === "supplement") {
        ctx.setLineDash([3, 3]);
        ctx.strokeStyle = "rgba(99,102,241,0.4)";
      } else {
        ctx.setLineDash([]);
        ctx.strokeStyle = "rgba(148,163,184,0.35)";
      }
      ctx.stroke();
      ctx.setLineDash([]);
    }

    // 3. 节点
    for (const n of sim.nodes) {
      // 外圈 glow（掌握状态越高越明显）
      if (n.depthLevel >= 3) {
        ctx.beginPath();
        ctx.arc(n.x, n.y, n.radius + 4, 0, Math.PI * 2);
        ctx.fillStyle = n.color + "22";
        ctx.fill();
      }

      ctx.beginPath();
      ctx.arc(n.x, n.y, n.radius, 0, Math.PI * 2);
      ctx.fillStyle = n.color;
      ctx.fill();

      // 白色内圈（视觉分层）
      ctx.beginPath();
      ctx.arc(n.x, n.y, n.radius * 0.45, 0, Math.PI * 2);
      ctx.fillStyle = "rgba(255,255,255,0.3)";
      ctx.fill();

      // 标签（节点够大时显示）
      if (n.radius >= 14 || scale > 1.2) {
        const label = n.title.length > 12 ? n.title.slice(0, 11) + "…" : n.title;
        ctx.font = `${Math.max(10, n.radius * 0.7)}px sans-serif`;
        ctx.fillStyle = "rgba(15,23,42,0.85)";
        ctx.textAlign = "center";
        ctx.textBaseline = "middle";
        ctx.fillText(label, n.x, n.y + n.radius + 10);
      }
    }

    ctx.restore();
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // ── Canvas 大小同步 ────────────────────────────────────────────────────────
  useEffect(() => {
    const canvas = canvasRef.current;
    const container = containerRef.current;
    if (!canvas || !container) return;

    const ro = new ResizeObserver(() => {
      canvas.width = container.clientWidth;
      canvas.height = container.clientHeight;
      render();
    });
    ro.observe(container);
    canvas.width = container.clientWidth;
    canvas.height = container.clientHeight;
    return () => ro.disconnect();
  }, [render]);

  // ── 鼠标事件 ──────────────────────────────────────────────────────────────
  const toWorld = (ex: number, ey: number) => {
    const canvas = canvasRef.current!;
    const rect = canvas.getBoundingClientRect();
    const { scale, tx, ty } = viewRef.current;
    return {
      wx: (ex - rect.left - tx) / scale,
      wy: (ey - rect.top - ty) / scale,
    };
  };

  const hitTest = (wx: number, wy: number): number => {
    const sim = simRef.current;
    for (let i = sim.nodes.length - 1; i >= 0; i--) {
      const n = sim.nodes[i];
      const dx = wx - n.x, dy = wy - n.y;
      if (Math.sqrt(dx * dx + dy * dy) <= n.radius) return i;
    }
    return -1;
  };

  const handleMouseDown = (e: React.MouseEvent) => {
    const { wx, wy } = toWorld(e.clientX, e.clientY);
    const idx = hitTest(wx, wy);
    const drag = dragRef.current;
    drag.startX = e.clientX;
    drag.startY = e.clientY;
    drag.lastTx = viewRef.current.tx;
    drag.lastTy = viewRef.current.ty;

    if (idx >= 0) {
      drag.type = "node";
      drag.nodeIdx = idx;
      simRef.current.nodes[idx].pinned = true;
    } else {
      drag.type = "pan";
      drag.nodeIdx = -1;
    }
  };

  const handleMouseMove = (e: React.MouseEvent) => {
    const drag = dragRef.current;
    const { wx, wy } = toWorld(e.clientX, e.clientY);

    // Tooltip on hover
    if (drag.type === null) {
      const idx = hitTest(wx, wy);
      if (idx >= 0) {
        const canvas = canvasRef.current!;
        const rect = canvas.getBoundingClientRect();
        setTooltip({
          x: e.clientX - rect.left + 12,
          y: e.clientY - rect.top - 8,
          node: simRef.current.nodes[idx],
        });
      } else {
        setTooltip(null);
      }
      return;
    }

    if (drag.type === "node") {
      const n = simRef.current.nodes[drag.nodeIdx];
      n.x = wx;
      n.y = wy;
      n.vx = 0;
      n.vy = 0;
      render();
    } else if (drag.type === "pan") {
      viewRef.current.tx = drag.lastTx + (e.clientX - drag.startX);
      viewRef.current.ty = drag.lastTy + (e.clientY - drag.startY);
      render();
    }
  };

  const handleMouseUp = (e: React.MouseEvent) => {
    const drag = dragRef.current;

    if (drag.type === "node" && drag.nodeIdx >= 0) {
      const n = simRef.current.nodes[drag.nodeIdx];
      n.pinned = false;
      // 如果几乎没移动，视为单击
      const dist = Math.hypot(e.clientX - drag.startX, e.clientY - drag.startY);
      if (dist < 5) {
        onNodeClick?.(n.id);
      }
    }

    drag.type = null;
    drag.nodeIdx = -1;
  };

  const handleMouseLeave = () => {
    const drag = dragRef.current;
    if (drag.type === "node" && drag.nodeIdx >= 0) {
      simRef.current.nodes[drag.nodeIdx].pinned = false;
    }
    drag.type = null;
    drag.nodeIdx = -1;
    setTooltip(null);
  };

  const handleWheel = (e: React.WheelEvent) => {
    e.preventDefault();
    const canvas = canvasRef.current!;
    const rect = canvas.getBoundingClientRect();
    const mx = e.clientX - rect.left;
    const my = e.clientY - rect.top;
    const { scale, tx, ty } = viewRef.current;
    const delta = e.deltaY > 0 ? 0.9 : 1.1;
    const newScale = Math.min(3, Math.max(0.3, scale * delta));
    // Zoom towards cursor
    viewRef.current.tx = mx - (mx - tx) * (newScale / scale);
    viewRef.current.ty = my - (my - ty) * (newScale / scale);
    viewRef.current.scale = newScale;
    render();
  };

  const zoom = (factor: number) => {
    const canvas = canvasRef.current!;
    const cx = canvas.width / 2, cy = canvas.height / 2;
    const { scale, tx, ty } = viewRef.current;
    const newScale = Math.min(3, Math.max(0.3, scale * factor));
    viewRef.current.tx = cx - (cx - tx) * (newScale / scale);
    viewRef.current.ty = cy - (cy - ty) * (newScale / scale);
    viewRef.current.scale = newScale;
    render();
  };

  const resetView = () => {
    viewRef.current = { scale: 1, tx: 0, ty: 0 };
    render();
  };

  // ─── 渲染 UI ────────────────────────────────────────────────────────────────

  if (isLoading) {
    return (
      <div className="kgv-loading">
        <Loader2 size={20} className="kgv-spin" />
        <span>生成知识图谱...</span>
      </div>
    );
  }

  if (error) {
    return (
      <div className="kgv-error">
        <p>加载失败：{error}</p>
        <button className="kgv-retry" onClick={initGraph}>
          <RefreshCw size={13} /> 重试
        </button>
      </div>
    );
  }

  return (
    <div className="kgv-root" ref={containerRef}>
      {/* Canvas */}
      <canvas
        ref={canvasRef}
        className="kgv-canvas"
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseLeave}
        onWheel={handleWheel}
      />

      {/* 悬浮 Tooltip */}
      {tooltip && (
        <div
          className="kgv-tooltip"
          style={{ left: tooltip.x, top: tooltip.y }}
        >
          <div className="kgv-tip-status" style={{ color: tooltip.node.color }}>
            {tooltip.node.status}
            {" "}{"★".repeat(tooltip.node.depthLevel)}{"☆".repeat(5 - tooltip.node.depthLevel)}
          </div>
          <div className="kgv-tip-title">{tooltip.node.title}</div>
          <div className="kgv-tip-insight">{tooltip.node.coreInsight}</div>
          {tooltip.node.inferredCourse && (
            <div className="kgv-tip-course">{tooltip.node.inferredCourse}</div>
          )}
        </div>
      )}

      {/* 控制栏 */}
      <div className="kgv-controls">
        <button className="kgv-ctrl-btn" onClick={() => zoom(1.2)} title="放大">
          <ZoomIn size={14} />
        </button>
        <button className="kgv-ctrl-btn" onClick={() => zoom(0.8)} title="缩小">
          <ZoomOut size={14} />
        </button>
        <button className="kgv-ctrl-btn" onClick={resetView} title="重置视图">
          <Maximize2 size={14} />
        </button>
        <button className="kgv-ctrl-btn" onClick={initGraph} title="重新布局">
          <RefreshCw size={14} />
        </button>
      </div>

      {/* 图例 */}
      <div className="kgv-legend">
        <div className="kgv-legend-row">
          <span className="kgv-leg-line kgv-leg-solid" />
          <span>同域连接</span>
        </div>
        <div className="kgv-legend-row">
          <span className="kgv-leg-line kgv-leg-dashed" />
          <span>跨域连接</span>
        </div>
        <div className="kgv-legend-count">{nodeCount} 个知识单元</div>
      </div>
    </div>
  );
}
