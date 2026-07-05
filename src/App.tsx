import { HashRouter, Routes, Route } from "react-router-dom";
import { Sidebar } from "./components/Sidebar";
import { TopBar } from "./components/TopBar";
import { MissionControlPage } from "./pages/MissionControl";
import { BuildTargetsPage } from "./pages/BuildTargets";
import { InventoryPage } from "./pages/Inventory";
import { OperationsWorkspacePage } from "./pages/OperationsWorkspace";
import {
  ProductionPage,
  IndustryPage,
  LogisticsPage,
  MarketIntelligencePage,
  PlanningPage,
  IntelligencePage,
  ReportsPage,
  SettingsPage,
} from "./pages/placeholders";

export default function App() {
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
              <Route path="/targets" element={<BuildTargetsPage />} />
              <Route path="/inventory" element={<InventoryPage />} />
              <Route path="/production" element={<ProductionPage />} />
              <Route path="/industry" element={<IndustryPage />} />
              <Route path="/logistics" element={<LogisticsPage />} />
              <Route path="/market" element={<MarketIntelligencePage />} />
              <Route path="/planning" element={<PlanningPage />} />
              <Route path="/intelligence" element={<IntelligencePage />} />
              <Route path="/reports" element={<ReportsPage />} />
              <Route path="/settings" element={<SettingsPage />} />
            </Routes>
          </main>
        </div>
      </div>
    </HashRouter>
  );
}
