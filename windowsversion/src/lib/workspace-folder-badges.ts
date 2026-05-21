/** 与 WorkspaceFolderStrip / 侧栏共用 */
export function workspaceFolderKindBadge(kind: string): string {
  if (kind === "ai_organized") return "AI 归类";
  if (kind === "root_import") return "导入";
  if (kind === "root") return "文件夹";
  return "";
}
