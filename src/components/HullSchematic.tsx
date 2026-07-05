interface HullSchematicProps {
  progress: number; // 0..1
}

/**
 * Abstract capital-hull elevation schematic for any build target. Original
 * angular silhouette (deliberately not CCP art) that floods with furnace
 * amber as the build progresses, bottom-up like a drydock fit-out.
 * Later sprints can vary the silhouette by target category — never per hull.
 */
export function HullSchematic({ progress }: HullSchematicProps) {
  const pct = Math.max(0, Math.min(1, progress));
  const fillY = 300 - pct * 300;
  return (
    <svg
      viewBox="0 0 220 300"
      width="200"
      height="272"
      role="img"
      aria-label={`Build target hull schematic, ${Math.round(pct * 100)}% complete`}
    >
      <defs>
        <clipPath id="hull-clip">
          {/* angular monolith hull, stern at bottom */}
          <path d="M110 6 L134 44 L128 96 L164 128 L156 178 L182 208 L170 262 L136 250 L128 292 L92 292 L84 250 L50 262 L38 208 L64 178 L56 128 L92 96 L86 44 Z" />
        </clipPath>
      </defs>

      {/* drydock grid */}
      {Array.from({ length: 9 }, (_, i) => (
        <line key={i} x1="0" x2="220" y1={30 + i * 30} y2={30 + i * 30} stroke="var(--bulkhead)" strokeWidth="0.5" />
      ))}

      {/* progress flood inside the hull */}
      <g clipPath="url(#hull-clip)">
        <rect x="0" y="0" width="220" height="300" fill="var(--deck)" />
        <rect x="0" y={fillY} width="220" height={300 - fillY} fill="var(--furnace)" opacity="0.85" />
        <rect x="0" y={fillY - 2} width="220" height="2" fill="#ffd28a" />
      </g>

      {/* hull outline over the flood */}
      <path
        d="M110 6 L134 44 L128 96 L164 128 L156 178 L182 208 L170 262 L136 250 L128 292 L92 292 L84 250 L50 262 L38 208 L64 178 L56 128 L92 96 L86 44 Z"
        fill="none"
        stroke="var(--coolant)"
        strokeWidth="1.4"
      />

      {/* interior frame lines */}
      <g stroke="var(--coolant)" strokeWidth="0.6" opacity="0.5">
        <line x1="110" y1="6" x2="110" y2="292" />
        <line x1="56" y1="128" x2="164" y2="128" />
        <line x1="64" y1="178" x2="156" y2="178" />
        <line x1="50" y1="262" x2="170" y2="262" />
      </g>

      {/* completion waterline marker */}
      <line x1="6" y1={fillY} x2="214" y2={fillY} stroke="var(--furnace)" strokeWidth="0.8" strokeDasharray="4 4" />
    </svg>
  );
}
