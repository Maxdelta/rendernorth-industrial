export interface FormattedIsk {
  compact: string;
  exact: string;
  valid: boolean;
}

export function formatExactIsk(value: string | null): string;
export function formatCompactIsk(value: string | null): FormattedIsk;
