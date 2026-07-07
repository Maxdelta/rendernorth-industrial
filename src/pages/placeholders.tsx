import { PlaceholderPage } from "../components/PlaceholderPage";

export const ProductionPage = () => (
  <PlaceholderPage
    title="Production"
    mission="Everything that turns materials into hulls: run planning and buildable-today. Blueprint ownership and readiness now live on their own Blueprints page (Sprint 006) — this module covers turning owned blueprints into scheduled production."
    planned={[
      "Manufacturing planner: ME/TE-aware run proposals per character and facility",
      "Buildable-today calculator from live inventory",
      "Component line balancing for the selected build target",
    ]}
    sprint="Sprint 003–004"
  />
);

export const IndustryPage = () => (
  <PlaceholderPage
    title="Industry"
    mission="Live industrial operations: every manufacturing, reaction, and science job across all authorized characters."
    planned={[
      "Job board with countdowns and completion alerts",
      "Slot utilization per character and facility",
      "Job output mapped to build targets",
      "Blocked-job detection feeding Factory Health",
    ]}
    sprint="Sprint 004"
  />
);

export const LogisticsPage = () => (
  <PlaceholderPage
    title="Logistics"
    mission="Moving the right things to the right place: shopping, hauling, and recovery."
    planned={[
      "Shopping list: deterministic missing-input diff per build target",
      "Buy / build / mine / PI / asset-safety routing per item",
      "Copy-to-clipboard multibuy export",
      "Asset safety recovery planning with delivery costs",
    ]}
    sprint="Sprint 004"
  />
);

export const MarketIntelligencePage = () => (
  <PlaceholderPage
    title="Market Intelligence"
    mission="Prices, orders, and liquidity — the ISK side of the operation."
    planned={[
      "Wallet balances and journal per character",
      "Open buy/sell orders with fill status",
      "Price snapshots from optional feeds (Janice / Fuzzworks / ESI markets)",
      "Input cost tracking against build budgets",
    ]}
    sprint="Sprint 005"
  />
);

export const PlanningPage = () => (
  <PlaceholderPage
    title="Planning"
    mission="The forward view for any selected build target: requirement trees, critical paths, and what to start first."
    planned={[
      "Build tracker: blueprint-expanded requirement tree with ME-adjusted quantities",
      "On-hand vs in-progress vs missing per node",
      "Critical path by activity time",
      "PI and mining supply plans matched to target burn-down",
    ]}
    sprint="Sprint 004"
  />
);

export const IntelligencePage = () => (
  <PlaceholderPage
    title="Intelligence"
    mission="The deterministic advisory layer: every recommendation traceable to a rule, its inputs, and its thresholds. No black boxes, no AI-generated advice."
    planned={[
      "Recommendation feed across all build targets with rule IDs and inputs",
      "Rule registry: every REC-xxx rule documented and inspectable",
      "Bottleneck analysis: which tier blocks which target",
      "Dismiss/acknowledge flow with history",
    ]}
    sprint="Sprint 005"
  />
);

export const ReportsPage = () => (
  <PlaceholderPage
    title="Reports"
    mission="What the operation earned and spent, with explainable line items."
    planned={[
      "Profit calculator: full cost rollup per target (materials, job install, structure bonuses)",
      "Build vs buy per component from price snapshots",
      "Production history and throughput over time",
    ]}
    sprint="Sprint 005–006"
  />
);

export const SettingsPage = () => (
  <PlaceholderPage
    title="Settings"
    mission="Local-first control of the install: characters, data, and appearance. No cloud sync, no accounts, no billing — ever."
    planned={[
      "Character management: add via official CCP ESI OAuth (PKCE) browser flow",
      "Scope review: every ESI scope listed with why it is needed",
      "Data management: sync freshness, SDE version, wipe-all-local-data",
      "Notification preferences",
    ]}
    sprint="Sprint 002–003 (ESI auth), ongoing"
  />
);
