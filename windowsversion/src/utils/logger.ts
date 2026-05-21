/**
 * Central logger utility for NCdesktop.
 * Built-in logging code for components to output their states or lifecycle events.
 */

type LogLevel = 'info' | 'warn' | 'error' | 'debug';

class Logger {
  private getTimestamp() {
    return new Date().toISOString();
  }

  private formatMessage(level: LogLevel, context: string, message: string, data?: unknown) {
    const dataStr = data ? `\nData: ${JSON.stringify(data, null, 2)}` : '';
    return `[${this.getTimestamp()}] [${level.toUpperCase()}] [${context}] ${message}${dataStr}`;
  }

  info(context: string, message: string, data?: unknown) {
    console.info(this.formatMessage('info', context, message, data));
  }

  warn(context: string, message: string, data?: unknown) {
    console.warn(this.formatMessage('warn', context, message, data));
  }

  error(context: string, message: string, data?: unknown) {
    console.error(this.formatMessage('error', context, message, data));
  }

  debug(context: string, message: string, data?: unknown) {
    if (import.meta.env.DEV) {
      console.debug(this.formatMessage("debug", context, message, data));
    }
  }
}

export const logger = new Logger();
