import catalogJson from "../data/releases.json";

export type ReleaseChannel = "Open Beta" | "Beta" | "Release Candidate" | "Stable";
export type ReleaseCategoryKey = "added" | "changed" | "fixed" | "knownIssues" | "security" | "deprecated" | "removed";

export interface ReleaseEntry {
  version: string;
  name: string;
  date: string;
  channel: ReleaseChannel;
  summary: string;
  added: string[];
  changed: string[];
  fixed: string[];
  knownIssues: string[];
  security?: string[];
  deprecated?: string[];
  removed?: string[];
  internalCheckpoint?: string;
}

export interface ReleaseCatalog {
  publicVersion: string;
  releases: ReleaseEntry[];
}

export const RELEASE_CATALOG = catalogJson as ReleaseCatalog;
export const CURRENT_RELEASE = RELEASE_CATALOG.releases[0];

export const RELEASE_CATEGORIES: Array<{ key: ReleaseCategoryKey; label: string }> = [
  { key: "added", label: "Added" },
  { key: "changed", label: "Changed" },
  { key: "fixed", label: "Fixed" },
  { key: "knownIssues", label: "Known Issues" },
  { key: "security", label: "Security & Privacy" },
  { key: "deprecated", label: "Deprecated" },
  { key: "removed", label: "Removed" },
];

export function populatedCategories(release: ReleaseEntry) {
  return RELEASE_CATEGORIES.filter(category => (release[category.key] ?? []).length > 0);
}
