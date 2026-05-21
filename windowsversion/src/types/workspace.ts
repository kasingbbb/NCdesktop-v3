/** 与 Rust `WorkspaceFolderEntry` 对齐 */
export interface WorkspaceFolderEntry {
  relativePath: string;
  displayLabel: string;
  /** `ai_organized` | `root` | `root_import` */
  kind: string;
}
