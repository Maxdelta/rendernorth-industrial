import { useEffect, useState } from "react";
import { AboutPrivacy } from "../components/AboutPrivacy";
import { CharactersPanel } from "../components/CharactersPanel";
import { MarketSetup } from "../components/MarketSetup";
import { Panel } from "../components/Panel";
import { StaticDataSetup } from "../components/StaticDataSetup";
import { getAppSetting, setAppSetting } from "../lib/backend";

export function SettingsPage() {
  const [releaseIndicator, setReleaseIndicator] = useState(true);
  useEffect(() => {
    getAppSetting("release_notes_indicator_enabled").then(value => setReleaseIndicator(value !== "false"));
  }, []);
  async function toggleReleaseIndicator(enabled: boolean) {
    setReleaseIndicator(enabled);
    await setAppSetting("release_notes_indicator_enabled", String(enabled));
    window.dispatchEvent(new Event("rendernorth-release-preference"));
  }
  async function reopenWizard() {
    await setAppSetting("onboarding_completed", "false");
    window.location.reload();
  }
  return <div className="dash">
    <Panel title="Setup Status" keel="coolant" className="dash-hero" headerRight={<button className="target-select enabled" onClick={reopenWizard}>Run Setup Wizard</button>}>
      <p className="ph-mission">Review or repeat any first-run step here. Existing imports, connected characters, secure tokens, and synchronized snapshots are preserved.</p>
      <label className="release-preference"><input type="checkbox" checked={releaseIndicator} onChange={event => toggleReleaseIndicator(event.target.checked)} /><span>Show a subtle What's New indicator when a newer release has not been viewed.</span></label>
    </Panel>
    <StaticDataSetup />
    <CharactersPanel />
    <MarketSetup />
    <AboutPrivacy />
  </div>;
}
