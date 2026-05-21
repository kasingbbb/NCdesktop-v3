/**
 * 用户自定义 Prompt 功能 — 前端类型契约层
 *
 * 真相来源：custom_prompt_v1 / task_001_architect / output.md § 5.3
 * 字段命名与后端 `commands::user_prompt::PromptInfo`（Serde `rename_all = "camelCase"`）严格对齐。
 *
 * 命名隔离（ADR-005 / R6）：`PromptModule / PromptInfo / PROMPT_MODULES` 与 PR-4 半成品
 * `stores/promptStore.ts` 中的 `cmd.PromptInfo["kind"]`（classify/naming/tagging）语义独立、不复用。
 */

/**
 * 用户视角下 4 个可自定义的 Prompt 模块。
 *
 * - `tagging` 文件打标签
 * - `para`    PARA 分组
 * - `concept` 知识概念提取
 * - `aggregation` 知识聚合
 *
 * 注：后端实际映射为 3 条 LLM 调用链（task_001 § 2.1）；前端不感知这一聚合，
 * 仅按 4 module 暴露给用户。
 */
export type PromptModule = "tagging" | "para" | "concept" | "aggregation";

/**
 * 单条 Prompt 信息（List / Get 命令返回值）。
 *
 * 字段对齐 `commands::user_prompt::PromptInfo`（Serde camelCase）：
 * - `module`               白名单字面量
 * - `displayTitle`         展示标题（中文，PRD § 3.2 第 1 列）
 * - `defaultText`          当前内置默认 prompt 全文
 * - `userText`             用户覆写文本；`null` = 未自定义（走默认）
 * - `isCustom`             是否已自定义
 * - `builtinVersion`       用户保存时所基于的内置版本（R3 预留，MVP 固定 "1.0"）
 * - `updatedAt`            最近更新时间（RFC3339 / null 表示无记录）
 * - `requiredPlaceholders` 必含占位符（如 `{content}`），保存时静态校验（ADR-003 Layer B）
 * - `maxBytes`             保存时字节上限（ADR-004，MVP = 16 KiB = 16384）
 */
export interface PromptInfo {
  module: PromptModule;
  displayTitle: string;
  defaultText: string;
  userText: string | null;
  isCustom: boolean;
  builtinVersion: string;
  updatedAt: string | null;
  requiredPlaceholders: string[];
  maxBytes: number;
}

/**
 * 模块固定顺序：tagging → para → concept → aggregation。
 *
 * 后端 `list_user_prompts` 恒按本顺序返回 4 条；UI 渲染折叠区时遵循同一序。
 */
export const PROMPT_MODULES: PromptModule[] = ["tagging", "para", "concept", "aggregation"];

/**
 * 模块 → 用户可见中文标题。文案严格按 PRD § 3.2 第 1 列。
 */
export const PROMPT_MODULE_TITLES: Record<PromptModule, string> = {
  tagging: "文件打标签",
  para: "PARA 分组",
  concept: "知识概念提取",
  aggregation: "知识聚合",
};
