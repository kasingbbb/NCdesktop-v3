/**
 * task_008 AC-6 / task_014 配套：MarkItDown 提取失效码 → 中文文案映射表（zh-CN）。
 *
 * 与后端 `src-tauri/src/extraction/failure_code.rs::FailureCode::as_str()` 字符级一致
 * （SCREAMING_SNAKE_CASE）。新增/重命名错误码必须先回 Architect 修订
 * `task_001_architect/output.md` ADR-007 共识。
 *
 * 渲染规则（沿用 `lib/ipc-errors.ts` 风格）：
 * - 中文、动名词短句、不加感叹号、不加 emoji；
 * - 建议长度 ≤ 32 字，UI 渲染处可附"查看详情"链接展开 raw stderr；
 * - 面向终端用户，避免 stack/路径/包名 等技术行话。
 */

/** 8 类失败码字面（与后端 `FailureCode::as_str()` 同步） */
export type ExtractionFailureCode =
  | "E_RUNTIME_MISSING"
  | "E_EXTRA_MISSING_EPUB"
  | "E_SCAN_PDF_UNSUPPORTED"
  | "E_AUDIO_WRONG_ROUTE"
  | "E_OUTPUT_EMPTY"
  | "E_OUTPUT_GIBBERISH"
  | "E_OUTPUT_NO_STRUCTURE"
  | "E_TIMEOUT_90S";

/** task_014 用：旧 DB 中 `status=success & content=''` 被回填为该标签的"已知未验证"态。 */
export type LegacyUnverifiedCode = "legacy_unverified";

/** UI 可见的所有失败相关 code（8 + 1）联合。 */
export type ExtractionFailureLabel =
  | ExtractionFailureCode
  | LegacyUnverifiedCode;

/**
 * 9 项 code → 用户可见中文文案。
 *
 * **唯一来源**：UI 渲染请只通过本表查阅；后端 raw stderr 仅用于日志展开区，
 * 禁止直接展示。
 */
export const EXTRACTION_FAILURE_MESSAGES: Record<ExtractionFailureLabel, string> = {
  E_RUNTIME_MISSING: "内置转换运行时未就绪，请重启应用",
  E_EXTRA_MISSING_EPUB: "EPUB 解析组件缺失，无法读取该电子书",
  E_SCAN_PDF_UNSUPPORTED: "扫描型 PDF 暂不支持，需先用 OCR 转为文本",
  E_AUDIO_WRONG_ROUTE: "音频文件应由录音转写处理，不走文档转换",
  E_OUTPUT_EMPTY: "文档内容为空或无法识别，未生成有效文本",
  E_OUTPUT_GIBBERISH: "文档输出含大量乱码，无法用于知识库",
  E_OUTPUT_NO_STRUCTURE: "文档已读出但无可识别的标题或段落",
  E_TIMEOUT_90S: "文档处理超过 90 秒已自动终止，建议拆分后重试",
  legacy_unverified: "旧版本记录未经新校验，重新转换以确认内容可用",
};

/** 失败码闭集（运行时校验依据；与上表 key 双向一致）。 */
export const EXTRACTION_FAILURE_CODE_SET: ReadonlySet<string> = new Set(
  Object.keys(EXTRACTION_FAILURE_MESSAGES),
);

/** 类型守卫：字符串是否在 9 项闭集内。 */
export function isExtractionFailureLabel(
  code: unknown,
): code is ExtractionFailureLabel {
  return typeof code === "string" && EXTRACTION_FAILURE_CODE_SET.has(code);
}

/**
 * 取文案；未知 code 走通用兜底（不抛错），同时 `console.warn` 提示契约破坏，
 * 便于 dev 阶段及时发现后端新增了未在本表登记的 code。
 */
export function getExtractionFailureMessage(
  code: string | null | undefined,
): string {
  if (!code) return "处理失败，请查看详情";
  if (isExtractionFailureLabel(code)) {
    return EXTRACTION_FAILURE_MESSAGES[code];
  }
  if (typeof console !== "undefined") {
    console.warn(
      `[extraction-failure-codes] 未登记的 failure_code: ${code}，请回 task_008 / task_001_architect 同步`,
    );
  }
  return "处理失败，请查看详情";
}
