import { useEffect, useState } from "react";
import {
  checkForUpdates,
  clearSkippedUpdate,
  getUpdatePreferences,
  getUpdateState,
  openExternalUrl,
  saveUpdatePreferences,
  type UpdatePreferences,
  type UpdateState,
  type UpdateStatus,
} from "../lib/backend";
import { Panel } from "./Panel";

const STATUS_LABELS: Record<UpdateStatus, string> = {
  not_checked: "Not Checked",
  checking: "Checking",
  up_to_date: "Up to Date",
  update_available: "Update Available",
  check_failed: "Check Failed",
  offline: "Offline",
  skipped: "Skipped",
  disabled: "Disabled",
};

function when(value: string | null): string {
  return value ? new Date(value).toLocaleString() : "Never";
}

export function UpdateSettings() {
  const [state, setState] = useState<UpdateState | null>(null);
  const [preferences, setPreferences] = useState<UpdatePreferences>({
    automaticallyCheck: true,
    frequency: "daily",
    includePrerelease: true,
  });
  const [working, setWorking] = useState(false);
  const [feedback, setFeedback] = useState<string | null>(null);

  useEffect(() => {
    Promise.all([getUpdateState(), getUpdatePreferences()])
      .then(([nextState, nextPreferences]) => {
        setState(nextState);
        setPreferences(nextPreferences);
      })
      .catch(error => setFeedback(String(error)));
  }, []);

  async function save(next: UpdatePreferences) {
    setPreferences(next);
    try {
      await saveUpdatePreferences(next);
      const refreshed = await getUpdateState();
      setState(refreshed);
      window.dispatchEvent(new CustomEvent("rendernorth-update-state", { detail: refreshed }));
      setFeedback("Update preferences saved.");
    } catch (error) {
      setFeedback(`Could not save update preferences: ${String(error)}`);
    }
  }

  async function checkNow() {
    setWorking(true);
    setFeedback(null);
    setState(current => current ? { ...current, status: "checking" } : current);
    try {
      const refreshed = await checkForUpdates(true);
      setState(refreshed);
      window.dispatchEvent(new CustomEvent("rendernorth-update-state", { detail: refreshed }));
    } catch (error) {
      setFeedback(String(error));
    } finally {
      setWorking(false);
    }
  }

  async function clearSkipped() {
    const refreshed = await clearSkippedUpdate();
    setState(refreshed);
    window.dispatchEvent(new CustomEvent("rendernorth-update-state", { detail: refreshed }));
    setFeedback("Skipped-version notifications restored.");
  }

  const releaseTarget = state?.installerUrl ?? state?.releaseUrl;
  return <Panel title="Updates" keel="furnace" className="dash-hero">
    <p className="ph-mission">RenderNorth quietly checks the official public GitHub Releases API. Downloads open in your default browser; the application never downloads or installs updates automatically.</p>
    <div className="update-state-grid">
      <div><span>Installed Version</span><strong>{state?.installedVersion ?? "Loading…"}</strong></div>
      <div><span>Latest Known Version</span><strong>{state?.latestVersion ?? "Not checked"}</strong></div>
      <div><span>Last Checked</span><strong>{when(state?.lastChecked ?? null)}</strong></div>
      <div><span>Update Status</span><strong className={`update-status update-${state?.status ?? "not_checked"}`}>{STATUS_LABELS[state?.status ?? "not_checked"]}</strong></div>
    </div>

    {state?.latestReleaseTitle && <div className="update-release-card">
      <strong>{state.latestReleaseTitle}</strong>
      {state.summary && <span>{state.summary}</span>}
      {state.releaseDate && <span>Published {when(state.releaseDate)}</span>}
    </div>}
    {state?.lastError && <div className="sd-error"><div className="conflict-desc">{state.lastError}</div></div>}

    <div className="update-preferences">
      <label className="release-preference">
        <input type="checkbox" checked={preferences.automaticallyCheck} onChange={event => save({ ...preferences, automaticallyCheck: event.target.checked })} />
        <span>Automatically check for updates</span>
      </label>
      <label className="new-op-field">
        <span className="ops-field-label">Check frequency</span>
        <select value={preferences.frequency} onChange={event => save({ ...preferences, frequency: event.target.value as UpdatePreferences["frequency"] })}>
          <option value="daily">Daily</option>
          <option value="weekly">Weekly</option>
          <option value="never">Never</option>
        </select>
      </label>
      <label className="release-preference">
        <input type="checkbox" checked={preferences.includePrerelease} onChange={event => save({ ...preferences, includePrerelease: event.target.checked })} />
        <span>Include pre-release versions</span>
      </label>
    </div>

    <div className="new-op-actions setup-wrap">
      <button className="target-select enabled" disabled={working} onClick={checkNow}>{working ? "Checking…" : "Check Now"}</button>
      <button className="target-select enabled" disabled={!state?.releaseUrl} onClick={() => state?.releaseUrl && openExternalUrl(state.releaseUrl)}>View Latest Release</button>
      <button className="target-select enabled" disabled={!releaseTarget} onClick={() => releaseTarget && openExternalUrl(releaseTarget)}>{state?.installerUrl ? "Download Update" : "Open Release Page"}</button>
      {state?.portableUrl && <button className="target-select enabled" onClick={() => openExternalUrl(state.portableUrl!)}>Portable ZIP</button>}
      <button className="target-select" disabled={!state?.skippedVersion && !state?.reminderUntil} onClick={clearSkipped}>Restore Skipped Version Notifications</button>
    </div>
    {feedback && <div className="setup-feedback" role="status">{feedback}</div>}
  </Panel>;
}
