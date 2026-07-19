export interface RequestGate {
  begin(): () => boolean;
  invalidate(): void;
}

export function createRequestGate(): RequestGate;
