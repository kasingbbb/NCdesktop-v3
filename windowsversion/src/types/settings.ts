/** 应用设置 */
export interface AppSettings {
  theme: "light" | "dark" | "system";
  sidebarWidth: number;
  timelineHeight: number;

  autoImportOnConnect: boolean;
  importDeleteOriginal: boolean;
  defaultImportPath: string;

  dropzoneEnabled: boolean;
  dropzonePosition: { x: number; y: number };
  dropzoneSize: "small" | "medium" | "large";
  dropzoneAutoClassify: boolean;

  defaultPlaybackSpeed: number;
  preRollSeconds: number;
  waveformColor: string;

  transcriptionLanguage: string;
  aiClassificationEnabled: boolean;
  llmBridgeTarget: LLMTarget;

  analyticsEnabled: boolean;
  dataStoragePath: string;

  /** 学习功能总开关（sidebar v2，ADR-005）— 控制左侧栏「知识系统」分组（今日复习 + 知识库） */
  showLearningFeatures: boolean;
  /** 学生中心总开关 — 控制左侧栏「日历」入口 */
  showStudentCenter: boolean;
  /** 校历绑定（学习功能子项） */
  bindSchoolCalendar: boolean;
  /** 每日复习提醒（学习功能子项） */
  enableDailyReviewReminder: boolean;
  /** 升级智能 ON 评估一次性标记（fail-open，仅 false→true） */
  learningAutoEnableEvaluated: boolean;
}

/** LLM 目标 */
export type LLMTarget =
  | { type: "notebookLM" }
  | { type: "chatgpt" }
  | { type: "claude" }
  | { type: "custom"; endpoint: string };
