/**
 * 测试与调试专用日志（Vitest / 开发期），统一前缀便于 grep。
 * 勿在正式业务路径中高噪声调用。
 */
export type TestLogLevel = "debug" | "info" | "warn" | "error";

function formatLine(scope: string, message: string, extra?: unknown): string {
  const suffix =
    extra !== undefined ? ` ${typeof extra === "object" ? JSON.stringify(extra) : String(extra)}` : "";
  return `[TEST][${scope}] ${message}${suffix}`;
}

export function testLog(level: TestLogLevel, scope: string, message: string, extra?: unknown): void {
  const line = formatLine(scope, message, extra);
  switch (level) {
    case "debug":
      console.debug(line);
      break;
    case "info":
      console.info(line);
      break;
    case "warn":
      console.warn(line);
      break;
    case "error":
      console.error(line);
      break;
  }
}
