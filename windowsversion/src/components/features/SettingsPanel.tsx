import { useState } from "react";
import {
  X,
  Palette,
  CreditCard,
  MonitorSmartphone,
  Headphones,
  Brain,
  Shield,
  ToggleRight,
  FileText,
} from "lucide-react";
import { useSettingsStore } from "../../stores";
import { useUIStore } from "../../stores/uiStore";
import { useUserPromptStore } from "../../stores/userPromptStore";
import { LLMSettingsForm } from "./bridge/LLMSettingsForm";
import { PromptCustomizationPanel } from "../settings/PromptCustomizationPanel";
import type { AppSettings } from "../../types";

/** AC-8：离开前若有未保存的 Prompt 草稿，弹 confirm 守卫，避免误丢。 */
function confirmIfPromptDirty(): boolean {
  const dirty = useUserPromptStore.getState().dirty;
  if (Object.values(dirty).some(Boolean)) {
    return window.confirm("有未保存的 Prompt 修改，确定离开吗？");
  }
  return true;
}

type SettingsTab =
  | "appearance"
  | "features"
  | "tfcard"
  | "dropzone"
  | "audio"
  | "ai"
  | "prompt"
  | "privacy";

const TABS: Array<{ id: SettingsTab; label: string; icon: typeof Palette }> = [
  { id: "appearance", label: "外观", icon: Palette },
  { id: "features", label: "功能", icon: ToggleRight },
  { id: "tfcard", label: "TF 卡", icon: CreditCard },
  { id: "dropzone", label: "悬浮窗", icon: MonitorSmartphone },
  { id: "audio", label: "音频", icon: Headphones },
  { id: "ai", label: "AI / LLM", icon: Brain },
  { id: "prompt", label: "Prompt 自定义", icon: FileText },
  { id: "privacy", label: "隐私", icon: Shield },
];

interface SettingsPanelProps {
  onClose: () => void;
}

export function SettingsPanel({ onClose }: SettingsPanelProps) {
  const [activeTab, setActiveTab] = useState<SettingsTab>("appearance");
  const { settings, updateSetting, setTheme } = useSettingsStore();

  // AC-8：onClose / 切 Tab 离开 Prompt 自定义页前的 dirty 守卫
  const handleClose = () => {
    if (activeTab === "prompt" && !confirmIfPromptDirty()) return;
    onClose();
  };
  const handleSwitchTab = (next: SettingsTab) => {
    if (activeTab === "prompt" && next !== "prompt" && !confirmIfPromptDirty()) return;
    setActiveTab(next);
  };

  return (
    <>
      <div
        className="fixed inset-0 z-50"
        style={{ backgroundColor: "rgba(0, 0, 0, 0.4)" }}
        onClick={handleClose}
      />

      <div
        className="fixed top-[10%] left-1/2 -translate-x-1/2 z-50 w-[640px] max-h-[75vh] rounded-[var(--radius-lg)] flex overflow-hidden"
        style={{
          backgroundColor: "var(--surface-elevated)",
          border: "1px solid var(--border-primary)",
        }}
      >
        {/* 左侧标签栏 */}
        <div
          className="w-[180px] flex-shrink-0 border-r py-[var(--space-4)]"
          style={{ borderColor: "var(--border-primary)" }}
        >
          <div className="px-[var(--space-4)] mb-[var(--space-4)]">
            <h2
              className="text-[var(--text-base)] font-semibold"
              style={{ color: "var(--text-primary)" }}
            >
              设置
            </h2>
          </div>
          <nav className="space-y-[var(--space-1)] px-[var(--space-2)]">
            {TABS.map((tab) => {
              const Icon = tab.icon;
              const isActive = activeTab === tab.id;
              return (
                <button
                  key={tab.id}
                  className="w-full flex items-center gap-[var(--space-2)] px-[var(--space-3)] py-[var(--space-2)] rounded-[var(--radius-sm)] transition-colors"
                  style={{
                    backgroundColor: isActive ? "var(--surface-tertiary)" : "transparent",
                    color: isActive ? "var(--text-primary)" : "var(--text-secondary)",
                    fontWeight: isActive ? 600 : 400,
                  }}
                  onClick={() => handleSwitchTab(tab.id)}
                >
                  <Icon size={14} />
                  <span className="text-[var(--text-sm)]">{tab.label}</span>
                </button>
              );
            })}
          </nav>
        </div>

        {/* 右侧内容 */}
        <div className="flex-1 flex flex-col">
          <div className="flex items-center justify-end px-[var(--space-4)] py-[var(--space-3)]">
            <button className="p-1 rounded-[var(--radius-sm)] transition-colors" onClick={handleClose}>
              <X size={16} style={{ color: "var(--text-secondary)" }} />
            </button>
          </div>

          <div className="flex-1 overflow-y-auto px-[var(--space-6)] pb-[var(--space-6)]">
            {activeTab === "appearance" && (
              <div className="space-y-[var(--space-4)]">
                <h3 className="text-[var(--text-base)] font-semibold" style={{ color: "var(--text-primary)" }}>
                  外观设置
                </h3>
                <SettingRow label="主题">
                  <select
                    className="input-glass px-[var(--space-3)] py-[var(--space-2)] rounded-[var(--radius-md)] text-[var(--text-sm)]"
                    value={settings.theme}
                    onChange={(e) => setTheme(e.target.value as "light" | "dark" | "system")}
                  >
                    <option value="system">跟随系统</option>
                    <option value="light">亮色</option>
                    <option value="dark">暗色</option>
                  </select>
                </SettingRow>
                <SettingRow label="侧边栏宽度">
                  <input
                    type="range"
                    min={180}
                    max={360}
                    step={10}
                    value={settings.sidebarWidth}
                    onChange={(e) => updateSetting("sidebarWidth", Number(e.target.value))}
                    className="w-full accent-gray-700"
                  />
                  <span className="text-[10px] tabular-nums" style={{ color: "var(--text-tertiary)" }}>
                    {settings.sidebarWidth}px
                  </span>
                </SettingRow>
              </div>
            )}

            {activeTab === "features" && (
              <div className="space-y-[var(--space-4)]">
                <h3 className="text-[var(--text-base)] font-semibold" style={{ color: "var(--text-primary)" }}>
                  功能模块
                </h3>
                <p className="text-[var(--text-xs)]" style={{ color: "var(--text-tertiary)" }}>
                  打开后，对应入口才会出现在左侧栏。默认全部关闭。
                </p>
                <SettingRow label="学生中心（日历）">
                  <ToggleSwitch
                    checked={settings.showStudentCenter}
                    onChange={(v) => {
                      void updateSetting("showStudentCenter", v);
                      if (!v && useUIStore.getState().activeSidebarSection === "calendar") {
                        useUIStore.getState().setSidebarSection("recent");
                      }
                    }}
                  />
                </SettingRow>
                <SettingRow label="知识系统（今日复习、知识库）">
                  <ToggleSwitch
                    checked={settings.showLearningFeatures}
                    onChange={(v) => {
                      void updateSetting("showLearningFeatures", v);
                      if (!v) {
                        const section = useUIStore.getState().activeSidebarSection;
                        if (section === "today" || section === "knowledge-hub") {
                          useUIStore.getState().setSidebarSection("recent");
                        }
                      }
                    }}
                  />
                </SettingRow>
              </div>
            )}

            {activeTab === "tfcard" && (
              <div className="space-y-[var(--space-4)]">
                <h3 className="text-[var(--text-base)] font-semibold" style={{ color: "var(--text-primary)" }}>
                  TF 卡设置
                </h3>
                <SettingRow label="插入后自动导入">
                  <ToggleSwitch
                    checked={settings.autoImportOnConnect}
                    onChange={(v) => updateSetting("autoImportOnConnect", v)}
                  />
                </SettingRow>
                <SettingRow label="导入后删除原文件">
                  <ToggleSwitch
                    checked={settings.importDeleteOriginal}
                    onChange={(v) => updateSetting("importDeleteOriginal", v)}
                  />
                </SettingRow>
              </div>
            )}

            {activeTab === "dropzone" && (
              <div className="space-y-[var(--space-4)]">
                <h3 className="text-[var(--text-base)] font-semibold" style={{ color: "var(--text-primary)" }}>
                  悬浮窗设置
                </h3>
                <SettingRow label="启用全局悬浮窗">
                  <ToggleSwitch
                    checked={settings.dropzoneEnabled}
                    onChange={(v) => updateSetting("dropzoneEnabled", v)}
                  />
                </SettingRow>
                <SettingRow label="AI 自动分类">
                  <ToggleSwitch
                    checked={settings.dropzoneAutoClassify}
                    onChange={(v) => updateSetting("dropzoneAutoClassify", v)}
                  />
                </SettingRow>
                <SettingRow label="悬浮窗大小">
                  <select
                    className="input-glass px-[var(--space-3)] py-[var(--space-2)] rounded-[var(--radius-md)] text-[var(--text-sm)]"
                    value={settings.dropzoneSize}
                    onChange={(e) => updateSetting("dropzoneSize", e.target.value as AppSettings["dropzoneSize"])}
                  >
                    <option value="small">小</option>
                    <option value="medium">中</option>
                    <option value="large">大</option>
                  </select>
                </SettingRow>
              </div>
            )}

            {activeTab === "audio" && (
              <div className="space-y-[var(--space-4)]">
                <h3 className="text-[var(--text-base)] font-semibold" style={{ color: "var(--text-primary)" }}>
                  音频设置
                </h3>
                <SettingRow label="默认播放速度">
                  <select
                    className="input-glass px-[var(--space-3)] py-[var(--space-2)] rounded-[var(--radius-md)] text-[var(--text-sm)]"
                    value={settings.defaultPlaybackSpeed}
                    onChange={(e) => updateSetting("defaultPlaybackSpeed", Number(e.target.value))}
                  >
                    <option value={0.5}>0.5x</option>
                    <option value={0.75}>0.75x</option>
                    <option value={1}>1.0x</option>
                    <option value={1.25}>1.25x</option>
                    <option value={1.5}>1.5x</option>
                    <option value={2}>2.0x</option>
                  </select>
                </SettingRow>
                <SettingRow label="Pre-roll 倒退秒数">
                  <input
                    type="number"
                    min={0}
                    max={30}
                    step={1}
                    value={settings.preRollSeconds}
                    onChange={(e) => updateSetting("preRollSeconds", Number(e.target.value))}
                    className="input-glass w-20 px-[var(--space-2)] py-[var(--space-1)] text-[var(--text-sm)] text-center"
                  />
                  <span className="text-[10px]" style={{ color: "var(--text-tertiary)" }}>秒</span>
                </SettingRow>
                <SettingRow label="波形颜色">
                  <input
                    type="color"
                    value={settings.waveformColor}
                    onChange={(e) => updateSetting("waveformColor", e.target.value)}
                    className="w-8 h-8 rounded cursor-pointer"
                  />
                </SettingRow>
                <SettingRow label="转录语言">
                  <select
                    className="input-glass px-[var(--space-3)] py-[var(--space-2)] rounded-[var(--radius-md)] text-[var(--text-sm)]"
                    value={settings.transcriptionLanguage}
                    onChange={(e) => updateSetting("transcriptionLanguage", e.target.value)}
                  >
                    <option value="zh">中文</option>
                    <option value="en">English</option>
                    <option value="ja">日本語</option>
                    <option value="auto">自动检测</option>
                  </select>
                </SettingRow>
              </div>
            )}

            {activeTab === "ai" && <LLMSettingsForm />}

            {activeTab === "prompt" && <PromptCustomizationPanel />}

            {activeTab === "privacy" && (
              <div className="space-y-[var(--space-4)]">
                <h3 className="text-[var(--text-base)] font-semibold" style={{ color: "var(--text-primary)" }}>
                  隐私设置
                </h3>
                <SettingRow label="匿名使用分析">
                  <ToggleSwitch
                    checked={settings.analyticsEnabled}
                    onChange={(v) => updateSetting("analyticsEnabled", v)}
                  />
                </SettingRow>
                <SettingRow label="数据存储路径">
                  <span className="text-[var(--text-xs)] truncate max-w-[200px]" style={{ color: "var(--text-secondary)" }}>
                    {settings.dataStoragePath || "默认（应用数据目录）"}
                  </span>
                </SettingRow>
                <div
                  className="p-[var(--space-3)] rounded-[var(--radius-md)]"
                  style={{
                    backgroundColor: "rgba(52, 199, 89, 0.06)",
                    border: "1px solid rgba(52, 199, 89, 0.15)",
                  }}
                >
                  <p className="text-[var(--text-xs)]" style={{ color: "var(--text-secondary)" }}>
                    NoteCapt 不收集任何用户隐私数据。所有数据存储在本地 SQLite 数据库中。
                    AI 功能仅在用户主动触发时发送文本数据到 OpenAI API，不发送原始媒体文件。
                  </p>
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
    </>
  );
}

function SettingRow({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex items-center justify-between gap-[var(--space-4)]">
      <span className="text-[var(--text-sm)]" style={{ color: "var(--text-primary)" }}>
        {label}
      </span>
      <div className="flex items-center gap-[var(--space-2)]">{children}</div>
    </div>
  );
}

function ToggleSwitch({
  checked,
  onChange,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <button
      className="relative w-10 h-6 rounded-full transition-colors"
      style={{
        backgroundColor: checked ? "#111827" : "var(--surface-tertiary)",
        border: `1px solid ${checked ? "#111827" : "var(--border-primary)"}`,
      }}
      onClick={() => onChange(!checked)}
    >
      <div
        className="absolute top-0.5 w-4 h-4 rounded-full transition-transform bg-white"
        style={{
          transform: checked ? "translateX(18px)" : "translateX(2px)",
          boxShadow: "var(--shadow-sm)",
        }}
      />
    </button>
  );
}
