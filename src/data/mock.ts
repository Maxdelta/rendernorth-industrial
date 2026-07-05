// Sprint 002 demo dataset — browser-mode fallback with working target
// selection. Mirrors the seed rows in migrations 0001 + 0002. No ship is
// special-cased: every target is plain data flowing through the same shapes.
import type {
  BuildTargetSummary,
  MissionControl,
  MissingMaterial,
  Recommendation,
  TierCoverage,
} from "../lib/backend";

interface MockTarget {
  projectId: number;
  name: string;
  className: string;
  status: string;
  overallProgress: number;
  tiers: TierCoverage[];
  missingMaterials: MissingMaterial[];
  recommendation: Recommendation;
}

const TARGETS: MockTarget[] = [
  {
    projectId: 1,
    name: "Avatar",
    className: "Titan — Amarr capital hull",
    status: "active",
    overallProgress: 0.61,
    tiers: [
      { label: "Minerals", coverage: 1.0 },
      { label: "Capital Components", coverage: 0.63 },
      { label: "Advanced Components", coverage: 0.44 },
      { label: "PI", coverage: 0.8 },
    ],
    missingMaterials: [
      { name: "Nocxium", quantity: 812_000, category: "mineral" },
      { name: "Auto-Integrity Preservation Seal", quantity: 43, category: "component" },
      { name: "Broadcast Node", quantity: 18, category: "pi" },
    ],
    recommendation: {
      title: "Start Capital Construction Parts",
      reason:
        "Capital component coverage is 63% against a 100% target while mineral coverage is complete and 2 characters with open industry slots are idle. Capital Construction Parts are the longest-lead component line still short.",
      ruleId: "REC-001 idle-slots + complete-inputs + lowest-coverage-component",
      inputs: { capital_component_coverage: 0.63, mineral_coverage: 1.0, idle_characters: 2 },
    },
  },
  {
    projectId: 2,
    name: "Navy Revelation",
    className: "Dreadnought — Amarr faction capital",
    status: "planned",
    overallProgress: 0.24,
    tiers: [
      { label: "Minerals", coverage: 0.55 },
      { label: "Capital Components", coverage: 0.18 },
      { label: "Advanced Components", coverage: 0.1 },
      { label: "Faction Materials", coverage: 0.3 },
    ],
    missingMaterials: [
      { name: "Isogen", quantity: 1_400_000, category: "mineral" },
      { name: "Capital Armor Plates", quantity: 62, category: "component" },
      { name: "Purple Loyalty Tokens (LP store)", quantity: 40_000, category: "faction" },
    ],
    recommendation: {
      title: "Research capital component BPOs before committing minerals",
      reason:
        "Capital component coverage is 18% and advanced component coverage is 10%; starting mineral purchases now would lock ISK ahead of the true bottleneck. Bring component blueprint lines online first.",
      ruleId: "REC-002 coverage-floor-before-spend",
      inputs: {
        capital_component_coverage: 0.18,
        advanced_component_coverage: 0.1,
        mineral_coverage: 0.55,
      },
    },
  },
  {
    projectId: 3,
    name: "Apostle",
    className: "Force Auxiliary — Amarr capital",
    status: "planned",
    overallProgress: 0.47,
    tiers: [
      { label: "Minerals", coverage: 0.8 },
      { label: "Capital Components", coverage: 0.4 },
      { label: "Advanced Components", coverage: 0.35 },
    ],
    missingMaterials: [
      { name: "Megacyte", quantity: 96_000, category: "mineral" },
      { name: "Capital Capacitor Batteries", quantity: 21, category: "component" },
    ],
    recommendation: {
      title: "Buy Megacyte before the weekend restock cycle",
      reason:
        "Megacyte is the only mineral below target (96k short) while both component tiers have active jobs; clearing it unblocks the next component wave.",
      ruleId: "REC-003 single-mineral-blocker",
      inputs: { megacyte_missing: 96_000, mineral_coverage: 0.8, capital_component_coverage: 0.4 },
    },
  },
  {
    projectId: 4,
    name: "Capital Construction Parts",
    className: "Capital component",
    status: "planned",
    overallProgress: 0.72,
    tiers: [
      { label: "Minerals", coverage: 1.0 },
      { label: "Blueprint Research", coverage: 0.9 },
    ],
    missingMaterials: [{ name: "Tritanium", quantity: 5_200_000, category: "mineral" }],
    recommendation: {
      title: "Start Capital Construction Parts runs on idle slots",
      reason:
        "Mineral coverage is 100% and research is at 90%; idle manufacturing slots can begin runs immediately with owned stock.",
      ruleId: "REC-001 idle-slots + complete-inputs + lowest-coverage-component",
      inputs: { mineral_coverage: 1.0, blueprint_research: 0.9, idle_characters: 2 },
    },
  },
  {
    projectId: 5,
    name: "Broadcast Node",
    className: "PI — P4 advanced commodity",
    status: "planned",
    overallProgress: 0.35,
    tiers: [
      { label: "P3 Inputs", coverage: 0.35 },
      { label: "Launchpad Capacity", coverage: 1.0 },
    ],
    missingMaterials: [
      { name: "Ukomi Superconductors", quantity: 120, category: "p3" },
      { name: "Condensates", quantity: 120, category: "p3" },
      { name: "High-Tech Transmitters", quantity: 120, category: "p3" },
    ],
    recommendation: {
      title: "Restock P3 inputs across launchpads",
      reason:
        "All three P3 inputs are at 120 units short each while launchpad capacity is free; one hauling pass restores the P4 chain.",
      ruleId: "REC-004 pi-input-floor",
      inputs: { p3_missing_each: 120, launchpad_capacity: 1.0 },
    },
  },
];

// Browser-mode selection state (the Tauri path persists this in app_meta).
let selectedProjectId = 1;

function deriveStatus(health: number, blockedJobs: number): string {
  if (blockedJobs > 0) return "Blocked";
  if (health < 0.6) return "Degraded";
  return "Operational";
}

export function getMockMissionControl(): MissionControl {
  const t = TARGETS.find((x) => x.projectId === selectedProjectId) ?? TARGETS[0];
  const health = 0.87;
  const blockedJobs = 0;
  return {
    source: "mock",
    asOf: new Date().toISOString(),
    runningJobs: 34,
    idleCharacters: 2,
    idleBpos: 7,
    walletIsk: 50_200_000_000,
    factoryHealth: {
      health,
      status: deriveStatus(health, blockedJobs),
      idleSlots: 12,
      blockedJobs,
      missingInputs: t.missingMaterials.length,
      iskLockedInJobs: 18_700_000_000,
      projectedFinishDays: 12,
    },
    selectedTarget: {
      projectId: t.projectId,
      name: t.name,
      className: t.className,
      overallProgress: t.overallProgress,
      tiers: t.tiers,
    },
    missingMaterials: t.missingMaterials,
    recommendation: t.recommendation,
  };
}

export function listMockTargets(): BuildTargetSummary[] {
  return TARGETS.map((t) => ({
    projectId: t.projectId,
    name: t.name,
    className: t.className,
    status: t.status,
    overallProgress: t.overallProgress,
    isSelected: t.projectId === selectedProjectId,
  }));
}

export function selectMockTarget(projectId: number): MissionControl {
  if (TARGETS.some((t) => t.projectId === projectId)) {
    selectedProjectId = projectId;
  }
  return getMockMissionControl();
}
