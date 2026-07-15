import { useEffect, useState } from "react";
import { exportDiagnostics, getAboutInfo, openExternalUrl, type AboutInfo } from "../lib/backend";
import { Panel } from "./Panel";

export function AboutPrivacy() {
  const [about, setAbout] = useState<AboutInfo | null>(null);
  const [feedback, setFeedback] = useState<string | null>(null);
  const [contactFeedback, setContactFeedback] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => { getAboutInfo().then(setAbout).catch((cause) => setError(String(cause))); }, []);
  async function diagnostics() {
    setError(null); setFeedback(null);
    try { const path = await exportDiagnostics(); if (path) setFeedback(`Diagnostics exported to ${path}`); }
    catch (cause) { setError(String(cause)); }
  }
  async function copyDiscordUsername() {
    setContactFeedback(null);
    try {
      await navigator.clipboard.writeText("maxdelta0089");
      setContactFeedback("Discord username copied");
    } catch (cause) {
      setError(`Could not copy Discord username: ${String(cause)}`);
    }
  }
  const link = (label: string, url?: string) => <button className="target-select enabled" disabled={!url} onClick={() => url && openExternalUrl(url).catch((cause) => setError(String(cause)))}>{label}</button>;
  return <>
    <Panel title="Privacy & EVE Safety" keel="nominal" className="dash-hero">
      <div className="privacy-grid"><div><strong>Never automates gameplay</strong><span>RenderNorth provides industrial intelligence and never controls the EVE client.</span></div><div><strong>CCP-authorized read access only</strong><span>Only scopes shown during official EVE SSO authorization are used.</span></div><div><strong>Local-first data</strong><span>Industrial data remains in the local SQLite database. Tokens remain in Windows Credential Manager.</span></div><div><strong>No character credentials stored</strong><span>RenderNorth never receives or stores an EVE username or password.</span></div></div>
    </Panel>
    <Panel title="About RenderNorth Industrial" keel="coolant" className="dash-hero">
      {error && <div className="sd-error"><div className="conflict-desc">{error}</div></div>}
      <div className="about-grid">
        <div><span>Release status</span><strong>{about?.releaseStatus ?? "Open Beta 0.1"}</strong></div>
        <div><span>Version</span><strong>{about?.version ?? "Loading…"}</strong></div>
        <div><span>Build</span><strong>{about?.build && about.build !== "unknown" ? new Date(Number(about.build) * 1000).toLocaleString() : "Unknown"}</strong></div>
        <div><span>Git commit</span><strong>{about?.gitCommit ?? "Loading…"}</strong></div>
        <div><span>Database</span><strong>{about?.databaseVersion ?? "Loading…"}</strong></div>
        <div><span>Migration</span><strong>{about?.migrationVersion ?? "—"}</strong></div>
        <div><span>Rust</span><strong>{about?.rustVersion ?? "Loading…"}</strong></div>
      </div>
      <div className="new-op-actions setup-wrap">{link("RenderNorth Website", about?.website)}{link("Open Setup Guide", about?.setupGuide)}{link("GitHub", about?.github)}{link("Report a Bug", about?.issues)}{link("Join Discord", about?.discordInvite)}<button className="target-select enabled" onClick={diagnostics}>Export Diagnostics</button></div>
      <p className="ph-mission">Diagnostics include OS, build, database and migration versions, connected-character names and enabled scopes, last synchronization times, and market refresh state. They never include access tokens, refresh tokens, Client IDs, or credentials.</p>
      {feedback && <div className="setup-feedback">{feedback}</div>}
    </Panel>
    <Panel title="Support RenderNorth Industrial" keel="furnace" className="dash-hero">
      <p className="ph-mission">If RenderNorth Industrial saves you time or helps your corporation, optional support helps fund continued development, hosting, testing, code signing, and future updates. Support is always appreciated, never required.</p>
      <div className="about-support-grid">
        <div>
          <span>In EVE</span>
          <strong>Character: Maxdelta</strong>
          <span>Accepted: ISK or PLEX</span>
        </div>
        <div>
          <span>Outside EVE</span>
          {link("Buy Maxdelta a Coffee", about?.support)}
        </div>
      </div>
    </Panel>
    <Panel title="Contact & Support" keel="coolant" className="dash-hero">
      <h3 className="about-section-heading">Contact Maxdelta</h3>
      <p className="ph-mission">For beta feedback, bug reports, setup help, and feature suggestions, contact Maxdelta on Discord.</p>
      <div className="about-contact-card">
        <strong>Contact Maxdelta directly</strong>
        <span>Discord username: <code>{about?.discordUsername ?? "maxdelta0089"}</code></span>
        <span>Server: <code>https://discord.gg/XycCz6ppx</code></span>
        <div className="new-op-actions setup-wrap">
          <button className="target-select enabled" onClick={copyDiscordUsername}>Copy Discord Username</button>
          {link("Open Discord Server", about?.discordInvite)}
        </div>
        {contactFeedback && <div className="setup-feedback" role="status">{contactFeedback}</div>}
      </div>
      <p className="data-source">Community contact only. This is not an official CCP support channel.</p>
    </Panel>
  </>;
}
