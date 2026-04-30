export class MarkdownFSError extends Error {
  readonly status: number;
  readonly body: unknown;

  constructor(status: number, message: string, body?: unknown) {
    super(`MarkdownFS ${status}: ${message}`);
    this.name = "MarkdownFSError";
    this.status = status;
    this.body = body;
  }
}
