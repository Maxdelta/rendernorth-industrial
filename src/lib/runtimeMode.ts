// Open Beta builds keep development-only UI out of the normal runtime.
// Opt in explicitly with ?debug=1 when working on internal fixtures or diagnostics.
export const SHOW_DEVELOPMENT_UI =
  typeof window !== "undefined" && new URLSearchParams(window.location.search).get("debug") === "1";
