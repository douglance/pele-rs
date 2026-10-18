// Pele errors.ts, pinned 458322a5 (MIT, kepano). Stack traces remain host-owned.
export class PeleError extends Error {
  code;
  type;
  line;
  column;
  snippet;
  constructor(message, code = 'syntax', info = {}) {
    super(message);
    this.name = 'PeleError';
    this.code = code;
    this.type = info.type ?? null;
    this.line = info.line ?? 0;
    this.column = info.column ?? 0;
    this.snippet = info.snippet ?? '';
  }
}
// Transport decoding is separate from the public constructor's string argument.
export function fromNativeError(error) {
  return new PeleError(error.message, error.code, error);
}
