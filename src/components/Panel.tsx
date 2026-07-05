import type { ReactNode } from "react";

export type Keel = "none" | "furnace" | "coolant" | "nominal" | "alert";

interface PanelProps {
  title?: string;
  keel?: Keel;
  className?: string;
  headerRight?: ReactNode;
  children: ReactNode;
}

/** Chamfered plate — the RenderNorth signature panel with a status keel. */
export function Panel({ title, keel = "none", className = "", headerRight, children }: PanelProps) {
  const keelClass = keel === "none" ? "" : `keel-${keel}`;
  return (
    <section className={`panel ${keelClass} ${className}`.trim()}>
      {title && (
        <header className="panel-header">
          <h2 className="panel-title">{title}</h2>
          <div className="spacer" />
          {headerRight}
        </header>
      )}
      {children}
    </section>
  );
}
