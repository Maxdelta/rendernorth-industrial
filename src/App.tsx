import { useEffect, useState } from "react";
import { HashRouter, Routes, Route } from "react-router-dom";
import { Sidebar } from "./components/Sidebar";
import { TopBar } from "./components/TopBar";
import { MissionControlPage } from "./pages/MissionControl";
import { InventoryPage } from "./pages/Inventory";
import { BlueprintsPage } from "./pages/Blueprints";
import { OperationsWorkspacePage } from "./pages/OperationsWorkspace";
import { ProductionPage } from "./pages/Production";
import { SettingsPage } from "./pages/Settings";
import { QuartermasterPage } from "./pages/Quartermaster";
import { WhatsNewPage } from "./pages/WhatsNew";
import { FirstRunWizard } from "./components/FirstRunWizard";
import { getAppSetting, setApplicationSectionTitle } from "./lib/backend";
import { SHOW_DEVELOPMENT_UI } from "./lib/runtimeMode";
import {
  IndustryPage,
  LogisticsPage,
  MarketIntelligencePage,
  PlanningPage,
  IntelligencePage,
  ReportsPage,
} from "./pages/placeholders";

// NOTE: src/pages/BuildTargets.tsx and src/components/TargetPicker.tsx are
// deliberately retained but unrouted. The Build Targets concept and its
// backend (list_build_targets/select_build_target, build_projects and its
// schema) are preserved pending a product decision on its future role
// (planning sandbox / quick estimator / production preview / pre-operation
// workflow) — see docs/architecture/DATA_OWNERSHIP.md. Only the redundant
// legacy UI surface is retired here; nothing about the concept is deleted.

export default function App() {
  const [onboardingComplete, setOnboardingComplete] = useState<boolean | null>(null);
  useEffect(() => { getAppSetting("onboarding_completed").then(value => setOnboardingComplete(value === "true")); }, []);
  useEffect(() => {
    if (onboardingComplete === false && !SHOW_DEVELOPMENT_UI) {
      void setApplicationSectionTitle("First-Run Setup").catch((error) => {
        console.error("failed to update application title", error);
      });
    }
  }, [onboardingComplete]);
  if (onboardingComplete === null) return <div className="setup-loading">Loading RenderNorth configuration…</div>;
  if (!onboardingComplete && !SHOW_DEVELOPMENT_UI) return <FirstRunWizard onComplete={() => setOnboardingComplete(true)} />;
  return (
    <HashRouter>
      <div className="shell">
        <Sidebar />
        <div className="main">
          <TopBar />
          <main className="route-outlet">
            <Routes>
              <Route path="/" element={<MissionControlPage />} />
              <Route path="/operations" element={<OperationsWorkspacePage />} />
              <Route path="/inventory" element={<InventoryPage />} />
              <Route path="/blueprints" element={<BlueprintsPage />} />
              <Route path="/production" element={<ProductionPage />} />
              <Route path="/quartermaster" element={<QuartermasterPage />} />
              <Route path="/industry" element={<IndustryPage />} />
              <Route path="/logistics" element={<LogisticsPage />} />
              <Route path="/market" element={<MarketIntelligencePage />} />
              <Route path="/planning" element={<PlanningPage />} />
              <Route path="/intelligence" element={<IntelligencePage />} />
              <Route path="/reports" element={<ReportsPage />} />
              <Route path="/settings" element={<SettingsPage />} />
              <Route path="/whats-new" element={<WhatsNewPage />} />
            </Routes>
          </main>
        </div>
      </div>
    </HashRouter>
  );
}
