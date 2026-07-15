import { AboutPrivacy } from "../components/AboutPrivacy";
import { CharactersPanel } from "../components/CharactersPanel";
import { MarketSetup } from "../components/MarketSetup";
import { Panel } from "../components/Panel";
import { StaticDataSetup } from "../components/StaticDataSetup";
import { setAppSetting } from "../lib/backend";

export function SettingsPage() {
  async function reopenWizard() {
    await setAppSetting("onboarding_completed", "false");
    window.location.reload();
  }
  return <div className="dash">
    <Panel title="Setup Status" keel="coolant" className="dash-hero" headerRight={<button className="target-select enabled" onClick={reopenWizard}>Run Setup Wizard</button>}>
      <p className="ph-mission">Review or repeat any first-run step here. Existing imports, connected characters, secure tokens, and synchronized snapshots are preserved.</p>
    </Panel>
    <StaticDataSetup />
    <CharactersPanel />
    <MarketSetup />
    <AboutPrivacy />
  </div>;
}
