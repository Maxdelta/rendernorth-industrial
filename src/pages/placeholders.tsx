import { PlaceholderPage } from "../components/PlaceholderPage";

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


