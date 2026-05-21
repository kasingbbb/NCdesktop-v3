import type { Tag } from "./common";

/** 项目 — 对应一堂课 / 一个会议 / 一个研究主题 */
export interface Project {
  id: string;
  libraryId: string;
  name: string;
  description: string;
  coverAssetId: string | null;
  source: ProjectSource;
  tags: Tag[];
  isPinned: boolean;
  isArchived: boolean;
  createdAt: string;
  updatedAt: string;
  metadata: ProjectMetadata;
}

/** 项目来源 */
export type ProjectSource =
  | { type: "tf_card"; deviceId: string; sessionId: string }
  | { type: "dropzone" }
  | { type: "manual" };

/** 项目统计元数据 */
export interface ProjectMetadata {
  totalDuration: number | null;
  assetCount: number;
  wordCount: number;
  importedAt: string | null;
}
