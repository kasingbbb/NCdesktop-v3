/**
 * Tauri IPC 命令封装层
 * 所有前端到 Rust 后端的调用统一在此文件管理
 */
import { invoke } from "@tauri-apps/api/core";
import type {
  Library,
  Project,
  Asset,
  Timeline,
  AudioTrack,
  Keyframe,
  Marker,
  Tag,
  Note,
  SearchResult,
  WorkspaceFolderEntry,
} from "../types";
import type { AIAnalysis } from "../types/asset";
import type {
  ConceptWithStats,
  ConceptDetail,
  ConceptViewpoint,
  ConceptExtension,
  ExtractionProgress as ConceptExtractionProgress,
} from "../types/knowledge";
import type {
  KnowledgeUnit,
  KnowledgeUnitSummary,
  UnderstandingSnapshot,
  CreateSnapshot,
} from "../types/knowledge-units";

// ── Library ────────────────────────────────────────

export async function getLibraries(): Promise<Library[]> {
  return invoke<Library[]>("get_libraries");
}

export async function createLibrary(name: string, rootPath: string): Promise<Library> {
  return invoke<Library>("create_library", { name, rootPath });
}

export async function updateLibrary(library: Library): Promise<void> {
  return invoke<void>("update_library", { library });
}

export async function deleteLibrary(id: string): Promise<void> {
  return invoke<void>("delete_library", { id });
}

// ── Project ────────────────────────────────────────

export async function getProjects(libraryId: string): Promise<Project[]> {
  return invoke<Project[]>("get_projects", { libraryId });
}

export async function getProject(id: string): Promise<Project | null> {
  return invoke<Project | null>("get_project", { id });
}

export async function createProject(libraryId: string, name: string): Promise<Project> {
  return invoke<Project>("create_project", { libraryId, name });
}

export async function updateProject(project: Project): Promise<void> {
  return invoke<void>("update_project", { project });
}

export async function deleteProject(id: string): Promise<void> {
  return invoke<void>("delete_project", { id });
}

// ── 工作区文件夹（NoteCaptWorkPlace/<projectId>）────────────────

export async function getProjectWorkspaceRoot(projectId: string): Promise<string> {
  return invoke<string>("get_project_workspace_root", { projectId });
}

export async function listProjectWorkspaceFolders(
  projectId: string
): Promise<WorkspaceFolderEntry[]> {
  return invoke<WorkspaceFolderEntry[]>("list_project_workspace_folders", { projectId });
}

export async function revealProjectWorkspaceFolder(
  projectId: string,
  relativePath: string
): Promise<void> {
  return invoke<void>("reveal_project_workspace_folder", { projectId, relativePath });
}

/**
 * task_011 AC-1：在 Finder（macOS）中显示给定源文件（高亮选中）。
 * 入参为绝对路径字符串；后端会校验路径非空 + 存在。
 */
export async function revealSourceFile(sourcePath: string): Promise<void> {
  return invoke<void>("reveal_source_file", { sourcePath });
}

// ── Asset ──────────────────────────────────────────

export async function getAssets(projectId: string): Promise<Asset[]> {
  return invoke<Asset[]>("get_assets", { projectId });
}

/** 项目内 assetId → 标签名（用于工作区视图） */
export async function getProjectAssetTagMap(
  projectId: string
): Promise<Record<string, string[]>> {
  return invoke<Record<string, string[]>>("get_project_asset_tag_map", { projectId });
}

export async function getAssetsByTag(projectId: string, tagId: string): Promise<Asset[]> {
  return invoke<Asset[]>("get_assets_by_tag", { projectId, tagId });
}

export async function getAsset(id: string): Promise<Asset | null> {
  return invoke<Asset | null>("get_asset", { id });
}

export async function createAsset(params: {
  projectId: string;
  assetType: string;
  name: string;
  filePath: string;
  fileSize: number;
  mimeType: string;
}): Promise<Asset> {
  return invoke<Asset>("create_asset", params);
}

/**
 * @deprecated 工作区 rename 已迁移到 {@link renameAsset}（ADR-007，命令以 asset_id 为唯一目标）。
 *  本函数仍保留供"非 rename"的整行 Asset 更新场景（is_starred 切换等同名兼容）使用。
 *  rename 调用者必须切换到 `renameAsset`，否则 markdown 衍生件 .name 不会双写。
 */
export async function updateAsset(asset: Asset): Promise<void> {
  return invoke<void>("update_asset", { asset });
}

/**
 * 工作区 rename 唯一入口（ADR-007）。
 * 后端会双写 root.name + markdown 衍生件 .name；不动磁盘文件。
 * 入参为 asset_id（接受 root.id 或 markdown derivative.id）+ 新展示名。
 * 返回最新的 WorkspaceAssetView，前端可就地 patch 不必重 fetch 整个列表。
 */
export async function renameAsset(
  assetId: string,
  newDisplayName: string
): Promise<import("../types/workspaceAsset").WorkspaceAssetView> {
  return invoke<import("../types/workspaceAsset").WorkspaceAssetView>("rename_asset", {
    assetId,
    newDisplayName,
  });
}

export async function deleteAsset(id: string): Promise<void> {
  return invoke<void>("delete_asset", { id });
}

export async function toggleAssetStar(id: string): Promise<boolean> {
  return invoke<boolean>("toggle_asset_star", { id });
}

export async function getAssetAnalysis(assetId: string): Promise<AIAnalysis | null> {
  return invoke<AIAnalysis | null>("get_asset_analysis", { assetId });
}

export async function moveAssetToWorkspaceFolder(
  assetIds: string[],
  targetRelativePath: string,
  projectId: string
): Promise<void> {
  return invoke<void>("move_asset_to_workspace_folder", {
    assetIds,
    targetRelativePath,
    projectId,
  });
}

/** 跨项目移动素材（BatchToolbar"移动到"路径）。返回更新后的素材行。 */
export async function moveAssets(
  assetIds: string[],
  targetProjectId: string
): Promise<Asset[]> {
  return invoke<Asset[]>("move_assets", { assetIds, targetProjectId });
}

/** 跨项目复制素材（BatchToolbar"复制到"路径）。返回新插入的素材行。 */
export async function copyAssets(
  assetIds: string[],
  targetProjectId: string
): Promise<Asset[]> {
  return invoke<Asset[]>("copy_assets", { assetIds, targetProjectId });
}

// ── MarkItDown 转换 ────────────────────────────────

export interface MarkitdownStatus {
  available: boolean;
  version: string | null;
  pythonCmd: string | null;
  reason: string | null;
  installHint: string | null;
}

export async function checkMarkitdownStatus(): Promise<MarkitdownStatus> {
  return invoke<MarkitdownStatus>("check_markitdown_status");
}

export interface ConversionResult {
  extractorType: string;
  markdown: string;
  qualityLevel: number;
  segmentCount: number;
}

export async function convertAssetToMarkdown(assetId: string): Promise<ConversionResult> {
  return invoke<ConversionResult>("convert_asset_to_markdown", { assetId });
}

// ── Timeline ───────────────────────────────────────

export async function getTimeline(projectId: string): Promise<Timeline | null> {
  return invoke<Timeline | null>("get_timeline", { projectId });
}

export async function createTimeline(params: {
  projectId: string;
  startTime: string;
  endTime: string;
  duration: number;
}): Promise<Timeline> {
  return invoke<Timeline>("create_timeline", params);
}

// ── AudioTrack ─────────────────────────────────────

export async function getAudioTracks(timelineId: string): Promise<AudioTrack[]> {
  return invoke<AudioTrack[]>("get_audio_tracks", { timelineId });
}

export async function createAudioTrack(params: {
  timelineId: string;
  filePath: string;
  fileName: string;
  format: string;
  duration: number;
  sampleRate: number;
  channels: number;
}): Promise<AudioTrack> {
  return invoke<AudioTrack>("create_audio_track", params);
}

// ── Keyframe ───────────────────────────────────────

export async function getKeyframes(timelineId: string): Promise<Keyframe[]> {
  return invoke<Keyframe[]>("get_keyframes", { timelineId });
}

export async function createKeyframe(params: {
  timelineId: string;
  assetId: string;
  anchorTime: number;
  source: string;
}): Promise<Keyframe> {
  return invoke<Keyframe>("create_keyframe", params);
}

export async function deleteKeyframe(id: string): Promise<void> {
  return invoke<void>("delete_keyframe", { id });
}

// ── Marker ─────────────────────────────────────────

export async function getMarkers(timelineId: string): Promise<Marker[]> {
  return invoke<Marker[]>("get_markers", { timelineId });
}

export async function createMarker(params: {
  timelineId: string;
  time: number;
  label: string;
  color: string;
  markerType: string;
}): Promise<Marker> {
  return invoke<Marker>("create_marker", params);
}

export async function deleteMarker(id: string): Promise<void> {
  return invoke<void>("delete_marker", { id });
}

// ── Tag ────────────────────────────────────────────

export async function getTags(): Promise<Tag[]> {
  return invoke<Tag[]>("get_tags");
}

export async function createTag(name: string, color: string, source: string): Promise<Tag> {
  return invoke<Tag>("create_tag", { name, color, source });
}

export async function deleteTag(id: string): Promise<void> {
  return invoke<void>("delete_tag", { id });
}

export async function linkTagToAsset(assetId: string, tagId: string): Promise<void> {
  return invoke<void>("link_tag_to_asset", { assetId, tagId });
}

export async function unlinkTagFromAsset(assetId: string, tagId: string): Promise<void> {
  return invoke<void>("unlink_tag_from_asset", { assetId, tagId });
}

/** 按名称查找或创建标签并关联到素材 */
export async function ensureAssetTagByName(assetId: string, name: string): Promise<Tag> {
  return invoke<Tag>("ensure_asset_tag_by_name", { assetId, name });
}

export async function getAssetTags(assetId: string): Promise<Tag[]> {
  return invoke<Tag[]>("get_asset_tags", { assetId });
}

/** 从 AI 分析行解析建议标签（后端 `suggestedTags` 为 JSON 字符串） */
export async function getAssetSuggestedTagNames(assetId: string): Promise<string[]> {
  const row = await invoke<{ suggestedTags?: string } | null>("get_asset_analysis", { assetId });
  if (!row?.suggestedTags?.trim()) {
    return [];
  }
  try {
    const parsed: unknown = JSON.parse(row.suggestedTags);
    if (!Array.isArray(parsed)) {
      return [];
    }
    return parsed.map((x) => String(x).trim()).filter((s) => s.length > 0);
  } catch {
    return [];
  }
}

// ── Note ───────────────────────────────────────────

export async function getNotes(projectId: string): Promise<Note[]> {
  return invoke<Note[]>("get_notes", { projectId });
}

export async function getNote(id: string): Promise<Note | null> {
  return invoke<Note | null>("get_note", { id });
}

export async function createNote(params: {
  projectId: string;
  content: string;
  assetId?: string;
  timelineTime?: number;
}): Promise<Note> {
  return invoke<Note>("create_note", params);
}

export async function updateNote(id: string, content: string): Promise<void> {
  return invoke<void>("update_note", { id, content });
}

export async function deleteNote(id: string): Promise<void> {
  return invoke<void>("delete_note", { id });
}

// ── Search ─────────────────────────────────────────

export async function searchAll(query: string, limit?: number): Promise<SearchResult[]> {
  return invoke<SearchResult[]>("search", { query, limit });
}

// ── Settings ───────────────────────────────────────

export async function getSetting(key: string): Promise<string | null> {
  return invoke<string | null>("get_setting", { key });
}

export async function setSetting(key: string, value: string): Promise<void> {
  return invoke<void>("set_setting", { key, value });
}

export async function getAllSettings(): Promise<Record<string, string>> {
  return invoke<Record<string, string>>("get_all_settings");
}

// ── 悬浮窗拖入导入 ─────────────────────────────────

/** 与后端 `ImportDropCreated` 一致：`Asset` 字段扁平 + AI 状态 */
export type ImportDropCreated = Asset & {
  aiClassified: boolean;
  aiNote: string | null;
  /** 为 true 时 LLM 在后台运行，界面可先显示「分析中」 */
  aiPending?: boolean;
};

export interface ImportDropSummary {
  created: ImportDropCreated[];
  failures: string[];
  importProjectName: string;
}

export async function importDropPaths(paths: string[]): Promise<ImportDropSummary> {
  return invoke<ImportDropSummary>("import_drop_paths", { paths });
}

export async function closeDropzoneWindow(): Promise<void> {
  return invoke<void>("close_dropzone_window");
}

// ── Sync ───────────────────────────────────────────

export interface DetectedCard {
  mountPath: string;
  arcaPath: string;
  deviceId: string;
  deviceName: string;
}

export interface ImportPreview {
  deviceName: string;
  deviceId: string;
  sessions: Array<{
    sessionId: string;
    title: string;
    startTime: string;
    endTime: string;
    audioDuration: number;
    photoCount: number;
    scanCount: number;
    isSynced: boolean;
  }>;
  newSessions: string[];
}

export async function scanTFCard(): Promise<{ cards: DetectedCard[] }> {
  return invoke<{ cards: DetectedCard[] }>("scan_tf_card");
}

export async function previewImport(arcaPath: string): Promise<ImportPreview> {
  return invoke<ImportPreview>("preview_import", { arcaPath });
}

export async function importSessions(params: {
  arcaPath: string;
  sessionIds: string[];
  libraryId: string;
}): Promise<string[]> {
  return invoke<string[]>("import_sessions", params);
}

export async function getSyncStatus(arcaPath: string): Promise<Array<{
  sessionId: string;
  deviceId: string;
  syncedAt: string;
  projectId: string;
}>> {
  return invoke("get_sync_status", { arcaPath });
}

// ── Audio ──────────────────────────────────────────

export interface AudioMetadataResult {
  duration: number;
  sampleRate: number;
  channels: number;
  format: string;
  fileSize: number;
}

export interface WaveformDataResult {
  sampleRate: number;
  duration: number;
  peaksPerSecond: number;
  peaks: Array<{ min: number; max: number }>;
}

export async function getAudioMetadata(filePath: string): Promise<AudioMetadataResult> {
  return invoke<AudioMetadataResult>("get_audio_metadata", { filePath });
}

export async function getWaveformData(filePath: string): Promise<WaveformDataResult> {
  return invoke<WaveformDataResult>("get_waveform_data", { filePath });
}

// ── Export ──────────────────────────────────────────

export interface ExportOptions {
  project_id: string;
  include_transcription: boolean;
  include_ocr: boolean;
  include_ai_summary: boolean;
  include_tags: boolean;
  include_notes: boolean;
  include_timeline: boolean;
}

export interface ExportResult {
  markdown: string;
  word_count: number;
  section_count: number;
}

export async function exportProjectMarkdown(options: ExportOptions): Promise<ExportResult> {
  return invoke<ExportResult>("export_project_markdown", { options });
}

export async function copyToClipboard(text: string): Promise<void> {
  return invoke("copy_to_clipboard", { text });
}

// ── LLM ──────────────────────────────────────────

export interface LLMConfig {
  api_key_masked: string;
  base_url: string;
  model: string;
  is_configured: boolean;
}

export interface LLMSummaryResult {
  summary: string;
  model: string;
  token_count: number;
}

export interface ClassifyResult {
  category: string;
  tags: string[];
  confidence: number;
  language: string;
  /** 建议主文件名（不含扩展名），导入分类后用于整理 */
  suggestedFileName?: string;
}

export async function getLLMConfig(): Promise<LLMConfig> {
  return invoke<LLMConfig>("get_llm_config");
}

/** 保存 LLM 到本地数据库；`apiKeyAction`：`keep` 不改 Key，`set` 用 `apiKeyValue`，`clear` 清除应用内 Key */
export interface SaveLlmConfigPayload {
  baseUrl: string;
  model: string;
  apiKeyAction: "keep" | "clear" | "set";
  apiKeyValue?: string;
}

export async function saveLLMConfig(payload: SaveLlmConfigPayload): Promise<void> {
  return invoke("save_llm_config", {
    payload: {
      baseUrl: payload.baseUrl,
      model: payload.model,
      apiKeyAction: payload.apiKeyAction,
      apiKeyValue: payload.apiKeyValue ?? "",
    },
  });
}

export async function llmSummarize(content: string, language: string): Promise<LLMSummaryResult> {
  return invoke<LLMSummaryResult>("llm_summarize", { content, language });
}

export async function llmClassify(content: string): Promise<ClassifyResult> {
  return invoke<ClassifyResult>("llm_classify", { content });
}

/** 固定样本调用分类 API，用于设置页验证连通性与 JSON 解析 */
export async function llmProbe(): Promise<ClassifyResult> {
  return invoke<ClassifyResult>("llm_probe");
}

export async function llmEnhanceExport(markdown: string): Promise<string> {
  return invoke<string>("llm_enhance_export", { markdown });
}

// ── 知识关联：概念 ─────────────────────────────────────

export async function getConcepts(libraryId: string): Promise<ConceptWithStats[]> {
  return invoke<ConceptWithStats[]>("get_concepts", { libraryId });
}

export async function getConceptDetail(conceptId: string): Promise<ConceptDetail | null> {
  return invoke<ConceptDetail | null>("get_concept_detail", { conceptId });
}

export async function updateConcept(
  conceptId: string,
  name?: string,
  definition?: string
): Promise<void> {
  return invoke("update_concept", { conceptId, name, definition });
}

export async function deleteConcept(conceptId: string): Promise<void> {
  return invoke("delete_concept", { conceptId });
}

/**
 * 触发知识库概念提取。
 *
 * `forceFull` 语义（concept_rescan_perf_v1 / task_perf_02）：
 *   - `true`  ⇒ 强制全量重扫（清空 concept_extracted_at 标记后重跑所有文档）
 *   - `false` ⇒ 增量扫描，跳过已扫描文档（task_perf_01 后端落地后启用）
 *
 * 调用层使用 camelCase `forceFull`，Tauri runtime 自动序列化为后端的
 * snake_case `force_full`。本期"重新扫描"按钮硬编码 `forceFull = true`
 * 以保持既有用户体验；增量扫描的 UI 入口（双按钮）放到 P2。
 *
 * 后端 command 名为 task_perf_01 新注册的 `start_concept_extraction`
 * （参数 `force_full: bool`）。旧 thin wrapper `extract_concepts_for_library`
 * 参数是 `force: bool`，与本函数 `forceFull` 不匹配——故走新入口。
 */
export async function extractConceptsForLibrary(
  libraryId: string,
  forceFull: boolean
): Promise<ConceptExtractionProgress> {
  return invoke<ConceptExtractionProgress>("start_concept_extraction", {
    libraryId,
    forceFull,
  });
}

export async function synthesizeViewpoints(
  conceptId: string
): Promise<ConceptViewpoint[]> {
  return invoke<ConceptViewpoint[]>("synthesize_viewpoints", { conceptId });
}

export async function generateExtensions(
  conceptId: string
): Promise<ConceptExtension[]> {
  return invoke<ConceptExtension[]>("generate_extensions", { conceptId });
}

/** 知识合成管道进度事件载荷（`notecapt/knowledge-synthesis-progress`） */
export interface SynthesisProgress {
  libraryId: string;
  stage: string;
  groupsFound: number;
  unitsWritten: number;
  error?: string | null;
}

// ── 知识单元（KU） ─────────────────────────────────────

export async function synthesizeKnowledgeUnits(
  libraryId: string,
  force: boolean
): Promise<KnowledgeUnitSummary[]> {
  return invoke<KnowledgeUnitSummary[]>("synthesize_knowledge_units", {
    libraryId,
    force,
  });
}

export async function kuGetList(libraryId: string): Promise<KnowledgeUnitSummary[]> {
  return invoke<KnowledgeUnitSummary[]>("ku_get_list", { libraryId });
}

export async function kuGetDetail(id: string): Promise<KnowledgeUnit | null> {
  return invoke<KnowledgeUnit | null>("ku_get_detail", { id });
}

export async function kuGetSnapshots(
  knowledgeUnitId: string
): Promise<UnderstandingSnapshot[]> {
  return invoke<UnderstandingSnapshot[]>("ku_get_snapshots", { knowledgeUnitId });
}

export async function kuCreateSnapshot(snapshot: CreateSnapshot): Promise<void> {
  return invoke("ku_create_snapshot", { snapshot });
}

export async function kuUpdateStatus(id: string, status: string): Promise<void> {
  return invoke("ku_update_status", { id, status });
}

export async function kuUpdateNote(id: string, userNote: string): Promise<void> {
  return invoke("ku_update_note", { id, userNote });
}

export async function kuUpdateMirrorFeedback(
  id: string,
  feedbackJson: string
): Promise<void> {
  return invoke("ku_update_mirror_feedback", { id, feedbackJson });
}

export async function kuUpdateReviewSchedule(
  id: string,
  nextReviewDue: string | null,
  depthLevel: number
): Promise<void> {
  return invoke("ku_update_review_schedule", { id, nextReviewDue, depthLevel });
}

export async function kuDelete(id: string): Promise<void> {
  return invoke("ku_delete", { id });
}

export async function kuGetDueForReview(
  libraryId: string,
  limit?: number
): Promise<KnowledgeUnitSummary[]> {
  return invoke<KnowledgeUnitSummary[]>("ku_get_due_for_review", {
    libraryId,
    limit,
  });
}

export interface ConversionMetaRow {
  id: string;
  sourceAssetId: string;
  derivedAssetId: string | null;
  converterName: string;
  converterVersion: string;
  sourceMime: string;
  sourceHash: string;
  qualityLevel: number;
  fallbackUsed: boolean;
  errorClass: string | null;
  conversionMs: number | null;
  convertedAt: string;
}

export async function getConversionMeta(assetId: string): Promise<ConversionMetaRow[]> {
  return invoke<ConversionMetaRow[]>("get_conversion_meta", { assetId });
}

// ── 提取重试（task_011 / task_026）─────────────────────────────────────────
// 后端 `retrigger_extraction`：从 failed/extracted 任一态干净重跑；
// 命中 queued/extracting 时安全 noop（后端幂等）。
//
// task_026 AC-1：新增 `forceKcRefresh` 可选参数（默认 false）。
// - false / 缺省：与 task_011 旧行为完全一致（reset extracted_content / pipeline_tasks
//   → enqueue → 唤醒 scheduler，markitdown 重跑会拉到当前 kc_enriched 值，已 enrich
//   过的 asset 不会重新跑 KC）。
// - true：在 reset 之后额外把 extracted_content.kc_enriched 置 NULL，
//   让 task_012 注入的 enrichment 在 save_and_materialize 时重新跑 KC。
//   仅用于 Inspector "重新增强"按钮（task_026 AC-3），不影响其他调用方。
export async function retriggerExtraction(
  assetId: string,
  forceKcRefresh?: boolean,
): Promise<void> {
  return invoke<void>("retrigger_extraction", { assetId, forceKcRefresh });
}

// ── 提取流水线查询/触发（与 extractionStore 对齐）─────────────────────────
import type { ExtractedContent, PipelineProgress } from "../types/extraction";

export async function extractAsset(assetId: string): Promise<string> {
  return invoke<string>("extract_asset", { assetId });
}

export async function extractProjectAssets(projectId: string): Promise<string> {
  return invoke<string>("extract_project_assets", { projectId });
}

export async function getExtractionStatus(
  assetId: string,
): Promise<ExtractedContent | null> {
  return invoke<ExtractedContent | null>("get_extraction_status", { assetId });
}

export async function getExtractedContent(
  assetId: string,
): Promise<ExtractedContent | null> {
  return invoke<ExtractedContent | null>("get_extracted_content", { assetId });
}

export async function getPipelineProgress(): Promise<PipelineProgress> {
  return invoke<PipelineProgress>("get_pipeline_progress");
}

// task_006 AC-1（M5）：工作区"重试转换"命令唯一入口。
// 本函数是后端 `retry_asset_conversion` 的薄包装，内部转发到
// `retrigger_extraction`，提供与 asset 视角对齐的命名。幂等性由后端三道
// 护栏保证（详见 `commands::extraction::retry_asset_conversion`）。
export async function retryAssetConversion(assetId: string): Promise<void> {
  return invoke<void>("retry_asset_conversion", { assetId });
}

// ── Outbound payload（task_005_dev_m4）─────────────────────────────────────
// 为多选 done 态资产准备 outbound .md 投影；非 done / 混合 / rendition 缺失等
// 错误以 JSON 字符串返回（OutboundError 联合类型），由调用方解析后 toast。

export interface OutboundEntry {
  assetId: string;
  /** 缓存目录内 .md 文件的绝对路径 */
  path: string;
  /** sanitize 后的文件名（含 .md 后缀） */
  displayName: string;
}

export type OutboundError =
  | { kind: "emptyInput"; message: string }
  | { kind: "stateNotDone"; assetId: string; state: string; message: string }
  | { kind: "mixedStates"; offending: string[]; message: string }
  | { kind: "renditionMissing"; assetId: string; message: string }
  | { kind: "assetNotFound"; assetId: string; message: string }
  | { kind: "ioFailed"; assetId: string | null; detail: string; message: string };

/**
 * 准备 outbound .md 投影。错误以 JSON 字符串通过 Promise reject 抛出，
 * 调用方应 `try { ... } catch (e) { parseOutboundError(e) }`。
 */
export async function prepareOutboundPayload(assetIds: string[]): Promise<OutboundEntry[]> {
  return invoke<OutboundEntry[]>("prepare_outbound_payload", { assetIds });
}

/** 把 Tauri 错误（可能是 OutboundError JSON 字符串）解析为结构化错误；解析失败回退为 null。 */
export function parseOutboundError(raw: unknown): OutboundError | null {
  if (typeof raw !== "string") return null;
  try {
    const parsed = JSON.parse(raw);
    if (parsed && typeof parsed === "object" && typeof parsed.kind === "string") {
      return parsed as OutboundError;
    }
    return null;
  } catch {
    return null;
  }
}

// ── User Prompt ────────────────────────────────────
// 用户自定义 Prompt 功能 — 与后端 `commands::user_prompt::{list,get,save,reset}_user_prompt` 对齐。
// 真相来源：task_001_architect / output.md § 6.2；后端落地见 task_002_dev_backend_data。
//
// 命名隔离（ADR-005 / R6）：函数名固定 `*UserPrompt*` 前缀，避免与 PR-4 半成品
// `stores/promptStore.ts` 中的 `cmd.getPrompt / savePrompt` 字面冲突。
//
// 错误：后端返回 `Result<T, String>`，Tauri 会把 `Err` 以 string 形式 reject，
// 调用方按既有范式 `try { await ... } catch (e) { /* String */ }` 即可。
import type { PromptInfo, PromptModule } from "../types/user-prompt";

/** 一次性加载全部 4 条 Prompt（按 tagging/para/concept/aggregation 序）。 */
export async function listUserPrompts(): Promise<PromptInfo[]> {
  return invoke<PromptInfo[]>("list_user_prompts");
}

/** 查询单条 Prompt 信息（编辑器进入或刷新时调用）。 */
export async function getUserPrompt(module: PromptModule): Promise<PromptInfo> {
  return invoke<PromptInfo>("get_user_prompt", { module });
}

/**
 * 保存用户自定义文本；后端经 `validate_module` 白名单 → `ensure_writable` 守卫 →
 * 16 KiB 字节校验 → `validate_required_placeholders`（task_003 起生效）四道防线。
 * 任一失败以 string 形式 reject。
 */
export async function saveUserPrompt(module: PromptModule, text: string): Promise<void> {
  return invoke<void>("save_user_prompt", { module, text });
}

/**
 * 恢复默认。
 * - `module = null` ⇒ 全部 4 条恢复默认（删除 user_custom_prompt 表所有行）
 * - `module = "..."` ⇒ 仅删除该 module 一行（缺行等价"未自定义"，回退到内置默认）
 */
export async function resetUserPrompt(module: PromptModule | null): Promise<void> {
  return invoke<void>("reset_user_prompt", { module });
}

// ── Knowledge Graph (Step 9) ──────────────────────────
// 前端 KnowledgeGraphView 力导向图数据源。
// 后端真相来源：`src-tauri/src/commands/knowledge_graph.rs::get_knowledge_graph`
// 之前因 `commands/mod.rs` 未声明该模块 + invoke_handler 未注册而导致
// `Importing binding name 'getKnowledgeGraph' is not found` BLOCKER。

/** 图谱节点：与后端 `GraphNode`（serde rename_all = camelCase）逐字段对齐。 */
export interface GraphNode {
  id: string;
  title: string;
  coreInsight: string;
  status: string;
  depthLevel: number;
  sourceAssetCount: number;
  /** 分组/颜色用：inferred_course（来自 asset_inferences）。 */
  inferredCourse: string | null;
}

/** 图谱边：与后端 `GraphEdge` 对齐。 */
export interface GraphEdge {
  source: string;
  target: string;
  /** "concept" | "similarity" | "supplement" */
  edgeType: string;
  /** 0.0–1.0，影响边粗细 */
  weight: number;
  /** true ⇒ 跨域虚线渲染 */
  isCrossDomain: boolean;
}

/** 图谱数据集：与后端 `KnowledgeGraphData` 对齐。 */
export interface KnowledgeGraphData {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

/** 加载指定知识库的图谱（节点 + 边）。 */
export async function getKnowledgeGraph(libraryId: string): Promise<KnowledgeGraphData> {
  return invoke<KnowledgeGraphData>("get_knowledge_graph", { libraryId });
}

// ── KC 集成层（task_020）─────────────────────────────────────────────────
// 后端真相来源：src-tauri/src/commands/kc.rs。
// 与 commands::kc::KcHealthStatusDto / KcSettingsPayload 字段严格 round-trip。
// camelCase 由后端 #[serde(rename_all = "camelCase")] 锁住——双向无歧义。

/** KC 子进程当前状态。前端 banner / settings 页 polling 显示。 */
export interface KcHealthStatus {
  /** "ready" | "starting" | "stopped" | "unavailable" */
  status: string;
  /** 仅 status=unavailable 时非空 */
  reason: string | null;
  /** 当前监听端口；非 ready 时为 null */
  port: number | null;
  /** 自进入 ready 起累计秒数；非 ready 时为 null */
  uptimeSecs: number | null;
  /** 后端调 health_check 时刻（RFC3339，可直接 `new Date()`） */
  lastCheck: string;
  /**
   * KC 子进程是否启用 AI 能力（基于 ai_provider 配置与 Key 实际可用性）。
   *
   * **PM ESCALATE 2026-05-27 补丁（task_016 AC-7）**：KC 后端 `/health` 已返回 ai_enabled。
   * 前端用此字段区分"Key 已配置但 AI 未启用"（如 ai_provider 缺失）vs"AI 完整就绪"。
   *
   * 后端 task_020 KcHealthStatusDto 当前可能尚未透传此字段（缺失时为 undefined/null）；
   * 前端按"未知"处理（不显示判定，避免误导用户）。
   */
  aiEnabled?: boolean | null;
}

/**
 * KcSettings 保存载荷。
 *
 * `*KeyAction` 三态语义（与 SaveLlmConfigPayload 一致）：
 * - `"keep"` ⇒ 不动 DB 中现有 Key；`*KeyValue` 可省略；
 * - `"clear"` ⇒ 清除 Key（DB 写空串，等价"未配置"）；`*KeyValue` 可省略；
 * - `"set"` ⇒ 用 `*KeyValue` 作为新 Key（trim 后非空，否则后端报错）。
 *
 * Key 任一变化时后端会异步触发 KcProcessManager.restart()
 * （前端订阅 `notecapt/kc-status-changed` 事件感知进度，本调用立即返回 Ok）。
 */
export interface KcSettingsPayload {
  enabled: boolean;
  useAi: boolean;
  enableQa: boolean;
  enableLinks: boolean;
  zhipuKeyAction: "keep" | "clear" | "set";
  zhipuKeyValue?: string;
  openaiKeyAction: "keep" | "clear" | "set";
  openaiKeyValue?: string;
}

/** 查询 KC 当前进程状态（成功路径永不抛错；HTTP 探测降级走 reason 字段）。 */
export async function getKcHealth(): Promise<KcHealthStatus> {
  return invoke<KcHealthStatus>("get_kc_health");
}

/**
 * 用户手动重启 KC 子进程。
 * 受冷却期约束（30s 内 ≥ 2 次 OR 60s 内 ≥ 3 次会被拒，错误以 string reject）。
 */
export async function restartKcProcess(): Promise<void> {
  return invoke<void>("restart_kc_process");
}

/**
 * 保存 KcSettings 7 字段；如两个 Key 任一发生变化，后端**异步**触发 KC 重启
 * （本调用立即返回 Ok，不阻塞 UI）。
 *
 * 错误以 string reject（如 `"请填写 zhipu_key_action 对应的 Key..."` /
 * `"无效的 openai_key_action..."`）。调用方按既有 `try { await ... } catch (e: string)` 模式处理。
 */
export async function setKcSettings(settings: KcSettingsPayload): Promise<void> {
  return invoke<void>("set_kc_settings", {
    settings: {
      enabled: settings.enabled,
      useAi: settings.useAi,
      enableQa: settings.enableQa,
      enableLinks: settings.enableLinks,
      zhipuKeyAction: settings.zhipuKeyAction,
      zhipuKeyValue: settings.zhipuKeyValue ?? "",
      openaiKeyAction: settings.openaiKeyAction,
      openaiKeyValue: settings.openaiKeyValue ?? "",
    },
  });
}

// ────────────────────────────────────────────────────────────────────────────
// TEMPORARY STUBS — skill IPC 链路在 main 上未接通（commit 184c6c0d 引入
// 后端 commands/skills.rs + commands/skill_mcp.rs 与 SkillsView UI，但漏掉
// 了 commands/mod.rs 注册、lib.rs invoke_handler 挂载、以及本文件 wrapper）。
// 临时给 10 个函数 + 4 个类型 stub，让 vite build 通过；运行时调用会 throw
// （DMG 里不点 skill 入口即可）。修复时按 getKnowledgeGraph 风格手写真正
// 的 invoke wrapper 并删除本节。
// ────────────────────────────────────────────────────────────────────────────

export type Skill = unknown;
export type SkillChallenge = unknown;
export type SkillEvaluation = unknown;
export type McpServerStatus = unknown;

const SKILL_IPC_NOT_WIRED = (name: string): never => {
  throw new Error(
    `skill IPC "${name}" not wired in main; see commands/skills.rs + lib.rs invoke_handler`,
  );
};

export async function skillGetList(_libraryId?: string): Promise<Skill[]> {
  return SKILL_IPC_NOT_WIRED("skillGetList");
}
export async function skillAutoAggregate(_libraryId?: string): Promise<unknown> {
  return SKILL_IPC_NOT_WIRED("skillAutoAggregate");
}
export async function skillComputeProgress(_skillId?: string): Promise<unknown> {
  return SKILL_IPC_NOT_WIRED("skillComputeProgress");
}
export async function skillGenerateChallenge(_skillId?: string): Promise<SkillChallenge> {
  return SKILL_IPC_NOT_WIRED("skillGenerateChallenge");
}
export async function skillEvaluateAnswer(_args?: unknown): Promise<SkillEvaluation> {
  return SKILL_IPC_NOT_WIRED("skillEvaluateAnswer");
}
export async function skillExportPackage(_skillId?: string): Promise<unknown> {
  return SKILL_IPC_NOT_WIRED("skillExportPackage");
}
export async function skillGetMcpConfig(_skillId?: string): Promise<unknown> {
  return SKILL_IPC_NOT_WIRED("skillGetMcpConfig");
}
export async function skillGetMcpServerStatus(_skillId?: string): Promise<McpServerStatus> {
  return SKILL_IPC_NOT_WIRED("skillGetMcpServerStatus");
}
export async function skillStartMcpServer(_skillId?: string): Promise<unknown> {
  return SKILL_IPC_NOT_WIRED("skillStartMcpServer");
}
export async function skillStopMcpServer(_skillId?: string): Promise<unknown> {
  return SKILL_IPC_NOT_WIRED("skillStopMcpServer");
}
