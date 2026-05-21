/** 标签 */
export interface Tag {
  id: string;
  name: string;
  color: string;
  source: "ai" | "user";
  usageCount: number;
}

/** 用户笔记 */
export interface Note {
  id: string;
  projectId: string;
  assetId: string | null;
  timelineTime: number | null;
  content: string;
  createdAt: string;
  updatedAt: string;
}
