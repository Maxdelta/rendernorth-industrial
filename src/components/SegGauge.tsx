interface SegGaugeProps {
  progress: number; // 0..1
  segments?: number;
}

/** Segmented industrial progress gauge for the overall build readout. */
export function SegGauge({ progress, segments = 20 }: SegGaugeProps) {
  const lit = Math.round(progress * segments);
  return (
    <div
      className="seg-gauge"
      role="progressbar"
      aria-valuenow={Math.round(progress * 100)}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-label="Overall build progress"
    >
      {Array.from({ length: segments }, (_, i) => (
        <div key={i} className={i < lit ? "seg lit" : "seg"} />
      ))}
    </div>
  );
}
