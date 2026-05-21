/**
 * 悬浮窗「最近导入」第二行：将后端/LLM 原始错误整理为可读中文，避免满屏环境变量名。
 */
export function formatDropzoneImportDetail(
  aiClassified: boolean,
  aiNote: string | null | undefined,
  aiPending?: boolean
): string {
  if (aiClassified) {
    return "已入库 · AI 已完成";
  }

  if (aiPending) {
    return "已入库 · AI 后台分析中…";
  }

  const raw = (aiNote ?? "").trim();
  if (!raw) {
    return "已入库";
  }

  if (raw.includes("未配置 AI，已跳过自动分类")) {
    return "已入库 · 未配置 AI，已跳过分类";
  }

  // 与 src-tauri/src/llm/client.rs from_env 等错误对齐
  if (
    raw.includes("环境变量未设置") ||
    raw.includes("未检测到 API Key") ||
    raw.includes("API Key 为空")
  ) {
    return "已入库 · AI 未配置（请设置 ARK_API_KEY 或 OPENAI_API_KEY）";
  }

  let line = raw.replace(/\b(sk-[a-zA-Z0-9]{12,})\b/g, "sk-***");
  if (line.length > 44) {
    line = `${line.slice(0, 42)}…`;
  }
  return `已入库 · AI：${line}`;
}
