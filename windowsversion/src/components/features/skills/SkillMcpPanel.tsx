/**
 * SkillMcpPanel — 技能 MCP 导出面板（Step 11）
 *
 * 在已验证的技能详情页底部展示：
 *   • 导出 SkillPackage JSON 文件
 *   • 一键启动 / 停止 localhost MCP 服务器
 *   • 展示接入配置（Claude Desktop / Cursor）
 *   • 复制配置到剪贴板
 */

import { useEffect, useState } from "react";
import {
  Check,
  CheckCircle2,
  Copy,
  Download,
  Loader2,
  Power,
  PowerOff,
  Server,
} from "lucide-react";
import type { McpServerStatus } from "../../../lib/tauri-commands";
import {
  skillExportPackage,
  skillGetMcpConfig,
  skillGetMcpServerStatus,
  skillStartMcpServer,
  skillStopMcpServer,
} from "../../../lib/tauri-commands";
import "./SkillMcpPanel.css";

// ─── Props ────────────────────────────────────────────────────────────────────

interface Props {
  skillId: string;
  skillName: string;
  libraryId: string;
}

// ─── 主组件 ───────────────────────────────────────────────────────────────────

export function SkillMcpPanel({ skillId, skillName, libraryId }: Props) {
  const [serverStatus, setServerStatus] = useState<McpServerStatus>({ running: false });
  const [isTogglingServer, setIsTogglingServer] = useState(false);
  const [isExporting, setIsExporting] = useState(false);
  const [exportResult, setExportResult] = useState<string | null>(null);
  const [mcpConfig, setMcpConfig] = useState<string | null>(null);
  const [copied, setCopied] = useState<"config" | "url" | null>(null);
  const [error, setError] = useState<string | null>(null);

  // 初始化时获取服务器状态
  useEffect(() => {
    skillGetMcpServerStatus()
      .then(setServerStatus)
      .catch(() => {});
  }, []);

  // 服务器运行时加载配置
  useEffect(() => {
    if (serverStatus.running && serverStatus.port) {
      skillGetMcpConfig(serverStatus.port)
        .then(setMcpConfig)
        .catch(() => {});
    } else {
      setMcpConfig(null);
    }
  }, [serverStatus.running, serverStatus.port]);

  const handleToggleServer = async () => {
    setIsTogglingServer(true);
    setError(null);
    try {
      if (serverStatus.running) {
        await skillStopMcpServer();
        setServerStatus({ running: false });
      } else {
        const status = await skillStartMcpServer(libraryId);
        setServerStatus(status);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setIsTogglingServer(false);
    }
  };

  const handleExport = async () => {
    setIsExporting(true);
    setError(null);
    setExportResult(null);
    try {
      // 空路径 → 返回 JSON 字符串，让用户另存
      const result = await skillExportPackage(skillId, "");
      const blob = new Blob([result], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `${skillName.replace(/\s+/g, "_")}_skill_package.json`;
      a.click();
      URL.revokeObjectURL(url);
      setExportResult("已导出");
    } catch (e) {
      setError(String(e));
    } finally {
      setIsExporting(false);
    }
  };

  const handleCopy = async (text: string, type: "config" | "url") => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(type);
      setTimeout(() => setCopied(null), 2000);
    } catch {
      // ignore
    }
  };

  return (
    <div className="smcp-root">
      {/* 标题 */}
      <div className="smcp-header">
        <Server size={13} />
        <span className="smcp-title">MCP 导出</span>
        <span className="smcp-badge">Step 11</span>
      </div>

      {/* 说明 */}
      <p className="smcp-desc">
        将此技能暴露为 MCP Tool，让 Claude Desktop 或 Cursor 直接查询你的知识库。
      </p>

      {/* 错误 */}
      {error && (
        <div className="smcp-error">{error}</div>
      )}

      {/* 服务器控制 */}
      <div className="smcp-section">
        <div className="smcp-section-header">
          <span className="smcp-section-label">本地 MCP 服务器</span>
          <span className={`smcp-status-dot ${serverStatus.running ? "smcp-dot-on" : "smcp-dot-off"}`} />
          <span className={`smcp-status-text ${serverStatus.running ? "smcp-text-on" : ""}`}>
            {serverStatus.running ? `运行中 · 端口 ${serverStatus.port}` : "已停止"}
          </span>
        </div>

        <div className="smcp-server-actions">
          <button
            className={`smcp-toggle-btn ${serverStatus.running ? "smcp-toggle-stop" : "smcp-toggle-start"}`}
            onClick={handleToggleServer}
            disabled={isTogglingServer}
          >
            {isTogglingServer ? (
              <Loader2 size={13} className="smcp-spin" />
            ) : serverStatus.running ? (
              <PowerOff size={13} />
            ) : (
              <Power size={13} />
            )}
            {serverStatus.running ? "停止服务器" : "启动服务器"}
          </button>

          {serverStatus.running && serverStatus.url && (
            <button
              className="smcp-copy-url-btn"
              onClick={() => handleCopy(serverStatus.url!, "url")}
              title="复制服务器 URL"
            >
              {copied === "url" ? <Check size={12} /> : <Copy size={12} />}
              {serverStatus.url}
            </button>
          )}
        </div>
      </div>

      {/* MCP 配置片段 */}
      {serverStatus.running && mcpConfig && (
        <div className="smcp-section">
          <div className="smcp-section-header">
            <span className="smcp-section-label">接入配置</span>
            <span className="smcp-section-hint">粘贴到 claude_desktop_config.json</span>
          </div>
          <div className="smcp-config-wrap">
            <pre className="smcp-config-code">{mcpConfig}</pre>
            <button
              className="smcp-copy-config-btn"
              onClick={() => handleCopy(mcpConfig, "config")}
            >
              {copied === "config" ? (
                <>
                  <CheckCircle2 size={12} />
                  已复制
                </>
              ) : (
                <>
                  <Copy size={12} />
                  复制
                </>
              )}
            </button>
          </div>
        </div>
      )}

      {/* 导出 SkillPackage */}
      <div className="smcp-section">
        <div className="smcp-section-header">
          <span className="smcp-section-label">导出技能包</span>
          <span className="smcp-section-hint">SkillPackage JSON</span>
        </div>
        <button
          className="smcp-export-btn"
          onClick={handleExport}
          disabled={isExporting}
        >
          {isExporting ? (
            <Loader2 size={13} className="smcp-spin" />
          ) : exportResult ? (
            <CheckCircle2 size={13} />
          ) : (
            <Download size={13} />
          )}
          {exportResult ?? "下载技能包 JSON"}
        </button>
        <p className="smcp-export-hint">
          包含技能元数据、知识单元摘要和 MCP Tool 定义，可分享或导入其他 AI 工具。
        </p>
      </div>
    </div>
  );
}
