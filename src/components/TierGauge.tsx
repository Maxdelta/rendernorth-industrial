interface TierGaugeProps {
  label: string;
  coverage: number; // 0..1
}

function toneFor(coverage: number): string {
  if (coverage >= 1) return "var(--nominal)";
  if (coverage >= 0.6) return "var(--furnace)";
  return "var(--alert)";
}

export function TierGauge({ label, coverage }: TierGaugeProps) {
  const pct = Math.round(coverage * 100);
  const color = toneFor(coverage);
  return (
    <div className="tier">
      <div className="tier-label">{label}</div>
      <div
        className="tier-track"
        role="progressbar"
        aria-valuenow={pct}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-label={label}
      >
        <div className="tier-fill" style={{ width: `${pct}%`, background: color }} />
      </div>
      <div className="tier-pct" style={{ color }}>
        {pct}%
      </div>
    </div>
  );
}
