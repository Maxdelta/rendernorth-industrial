import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import {
  checkForUpdates,
  getUpdateState,
  openExternalUrl,
  remindUpdateLater,
  skipUpdateVersion,
  type UpdateState,
} from "../lib/backend";

function broadcast(state: UpdateState) {
  window.dispatchEvent(new CustomEvent("rendernorth-update-state", { detail: state }));
}

export function UpdateMonitor() {
  useEffect(() => {
    let active = true;
    const timer = window.setTimeout(() => {
      checkForUpdates(false)
        .then((state) => { if (active) broadcast(state); })
        .catch(() => undefined);
    }, 2200);
    return () => {
      active = false;
      window.clearTimeout(timer);
    };
  }, []);
  return null;
}

export function UpdateNotification() {
  const [state, setState] = useState<UpdateState | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    getUpdateState().then(setState).catch(() => undefined);
    const listener = (event: Event) => setState((event as CustomEvent<UpdateState>).detail);
    window.addEventListener("rendernorth-update-state", listener);
    return () => window.removeEventListener("rendernorth-update-state", listener);
  }, []);

  if (!state || state.status !== "update_available") return null;

  async function download() {
    const target = state?.installerUrl ?? state?.releaseUrl;
    if (!target) return;
    if (!state?.installerUrl) setMessage("Installer link unavailable; opening the official release page.");
    await openExternalUrl(target);
  }

  async function remind() {
    const updated = await remindUpdateLater();
    setState(updated);
    broadcast(updated);
  }

  async function skip() {
    const updated = await skipUpdateVersion();
    setState(updated);
    broadcast(updated);
  }

  return <aside className="update-notice" aria-label="Update available">
    <div className="update-notice-copy">
      <strong>Update Available</strong>
      <span>Current {state.installedVersion} · Latest {state.latestVersion}</span>
      {message && <span className="update-notice-message">{message}</span>}
    </div>
    <div className="update-notice-actions">
      {state.releaseInCatalog
        ? <Link className="target-select enabled" to={`/whats-new?release=${encodeURIComponent(state.latestVersion ?? "")}`}>View What's New</Link>
        : <button className="target-select enabled" disabled={!state.releaseUrl} onClick={() => state.releaseUrl && openExternalUrl(state.releaseUrl)}>Open Release Notes</button>}
      <button className="target-select enabled" disabled={!state.installerUrl && !state.releaseUrl} onClick={download}>Download Update</button>
      {state.portableUrl && <button className="target-select enabled" onClick={() => openExternalUrl(state.portableUrl!)}>Portable ZIP</button>}
      <button className="target-select" onClick={remind}>Remind Me Later</button>
      <button className="target-select" onClick={skip}>Skip This Version</button>
    </div>
  </aside>;
}
