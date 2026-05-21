/**
 * 工作区折叠列表 DTO（task_001_architect §六 / ADR-002）。
 *
 * 后端来源：`src-tauri/src/models/asset.rs::WorkspaceAssetView`。
 * 仅用于"工作区列表"路径；其它非工作区视图（搜索 / 时间轴 / 知识中枢）
 * 仍使用 `types/asset.ts` 中的 `Asset`。
 *
 * 字段命名严格对齐后端 serde camelCase；任何新增字段都必须同步两侧。
 */

/** 工作区资产四态（由后端 `compute_asset_state` 实时派生，不持久化）。 */
export type AssetState = "done" | "converting" | "failed" | "offline";

/**
 * 工作区折叠列表中的"逻辑资产" = root asset + 派生关联 + 实时四态。
 *
 * - `id` / `name` / `filePath` 等：root asset 字段。
 * - `renditionPath`：canonical markdown 衍生件绝对路径（若存在）；outbound
 *   拖拽前需调用 `prepare_outbound_payload` 把该路径转换为带 displayName 的
 *   缓存 hardlink。
 * - `state` / `stateReason`：UI 状态徽章；`stateReason` 仅在 `state === "failed"`
 *   时携带（error_class 优先于 pipeline error_message）。
 * - `sourceMissing`：源文件已不在磁盘（用户手动删除 / 外部清理）；不改变 state，
 *   仅作为辅助提示位（task_007 启动期扫描会主动设置；命令调用时 stat 兜底）。
 */
export interface WorkspaceAssetView {
  id: string;
  projectId: string;
  assetType: string;
  /** 工作区内展示名（可被 AI / 用户重命名） */
  name: string;
  /** 拖入时的原始文件名 */
  originalName: string;
  /** source 文件绝对路径 */
  filePath: string;
  fileSize: number;
  mimeType: string;
  capturedAt: string;
  importedAt: string;
  sourceType: string;
  sourceData: string | null;
  isStarred: boolean;
  derivativeVersion: number;

  /** canonical markdown 衍生件 id（若存在） */
  renditionId: string | null;
  /** canonical markdown 绝对路径（若存在） */
  renditionPath: string | null;
  renditionSize: number | null;

  /** 实时派生的四态 */
  state: AssetState;
  /** 失败原因（仅 state === "failed" 时携带） */
  stateReason: string | null;
  /** source 文件已缺失（不改变 state，仅 UI 标记） */
  sourceMissing: boolean;
  /**
   * task_014 Fix-A4：`extracted_content.extractor_type`。
   *
   * 前端用于区分"占位 MD"（前缀 `placeholder_`，如 `placeholder_unsupported`、
   * `placeholder_extract_failed`、`placeholder_read_failed`）vs 真 MD
   * （如 `markitdown`、`text_passthrough`、`audio_asr_iflytek`、`source_markdown` 等）。
   * `null` = 该 root 尚无 extracted_content 行（或字段为空字符串）。
   */
  extractorType?: string | null;
  /**
   * task_014 AC-4：最近一行 `conversion_meta.failure_code`。
   *
   * - `"legacy_unverified"`：旧版本"成功 + 空内容"被 V14 backfill 标注；
   * - `"E_*"`（8 字面之一，见 `lib/extraction-failure-codes.ts`）：明确失败；
   * - `null`：当前为成功 / 未尝试。
   *
   * 三态 badge 渲染依据：与四态 `state` 互补，本字段聚焦"提取链路"的微观失败。
   */
  extractionFailureCode?: string | null;
}
