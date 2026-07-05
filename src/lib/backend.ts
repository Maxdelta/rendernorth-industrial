// The only file allowed to touch Tauri IPC. Everywhere else imports from here.
// In a plain browser (vite dev without the Rust shell) it falls back to mock data,
// so frontend work never requires the Rust toolchain.

export interface TierCoverage {
  label: string;
  coverage: number; // 0..1
}

export interface MissingMaterial {
  name: string;
  quantity: number;
  category: string;
}

export interface Recommendation {
  title: string;
  reason: string;
  ruleId: string;
  /** Machine-readable inputs the rule fired on (Determinism Doctrine). */
  inputs: Record<string, number | string> | null;
}

export interface SelectedTarget {
  projectId: number;
  name: string;
  className: string;
  overallProgress: number; // 0..1
  tiers: TierCoverage[]; // labels come from data — never hardcoded per hull
}

export interface BuildTargetSummary {
  projectId: number;
  name: string;
  className: string;
  status: string;
  overallProgress: number;
  isSelected: boolean;
}

export interface FactoryHealth {
  health: number; // 0..1
  status: string; // Blocked / Degraded / Operational (derived deterministically)
  idleSlots: number;
  blockedJobs: number;
  missingInputs: number;
  iskLockedInJobs: number;
  projectedFinishDays: number;
}

export interface MissionControl {
  source: "sqlite" | "mock";
  asOf: string;
  runningJobs: number;
  idleCharacters: number;
  idleBpos: number;
  walletIsk: number;
  factoryHealth: FactoryHealth;
  selectedTarget: SelectedTarget;
  missingMaterials: MissingMaterial[];
  recommendation: Recommendation;
}

export interface DbHealth {
  ok: boolean;
  schemaVersion: number;
  dbPath: string;
}

function inTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

export async function getMissionControl(): Promise<MissionControl> {
  if (inTauri()) {
    try {
      return await invoke<MissionControl>("get_mission_control");
    } catch (err) {
      console.error("get_mission_control failed, falling back to mock:", err);
    }
  }
  const mock = await import("../data/mock");
  return mock.getMockMissionControl();
}

export async function listBuildTargets(): Promise<BuildTargetSummary[]> {
  if (inTauri()) {
    try {
      return await invoke<BuildTargetSummary[]>("list_build_targets");
    } catch (err) {
      console.error("list_build_targets failed, falling back to mock:", err);
    }
  }
  const mock = await import("../data/mock");
  return mock.listMockTargets();
}

export async function selectBuildTarget(projectId: number): Promise<MissionControl> {
  if (inTauri()) {
    try {
      return await invoke<MissionControl>("select_build_target", { projectId });
    } catch (err) {
      console.error("select_build_target failed, falling back to mock:", err);
    }
  }
  const mock = await import("../data/mock");
  return mock.selectMockTarget(projectId);
}

export async function healthCheck(): Promise<DbHealth | null> {
  if (!inTauri()) return null;
  try {
    return await invoke<DbHealth>("health_check");
  } catch (err) {
    console.error("health_check failed:", err);
    return { ok: false, schemaVersion: -1, dbPath: "unavailable" };
  }
}

export function formatIsk(value: number): string {
  const abs = Math.abs(value);
  if (abs >= 1e12) return (value / 1e12).toFixed(1) + "T ISK";
  if (abs >= 1e9) return (value / 1e9).toFixed(1) + "B ISK";
  if (abs >= 1e6) return (value / 1e6).toFixed(1) + "M ISK";
  return value.toLocaleString() + " ISK";
}

export function formatQty(value: number): string {
  if (value >= 1_000_000) return (value / 1_000_000).toFixed(1).replace(/\.0$/, "") + "m";
  if (value >= 10_000) return Math.round(value / 1000) + "k";
  return value.toLocaleString();
}
