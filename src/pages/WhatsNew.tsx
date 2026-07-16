import { useEffect, useState } from "react";
import { Link, useSearchParams } from "react-router-dom";
import { Panel } from "../components/Panel";
import { getUpdateState, markCurrentReleaseViewed, openExternalUrl, type UpdateState } from "../lib/backend";
import { CURRENT_RELEASE, populatedCategories, RELEASE_CATALOG, type ReleaseEntry } from "../lib/releaseHistory";

function Category({ release, category }: { release: ReleaseEntry; category: ReturnType<typeof populatedCategories>[number] }) {
  const items = release[category.key] ?? [];
  return <section className={`release-category release-${category.key}`}>
    <h3>{category.label}</h3>
    <ul>{items.map(item => <li key={item}>{item}</li>)}</ul>
  </section>;
}

function ReleaseBody({ release }: { release: ReleaseEntry }) {
  return <>
    <p className="release-summary">{release.summary}</p>
    <div className="release-categories">
      {populatedCategories(release).map(category => <Category key={category.key} release={release} category={category} />)}
    </div>
  </>;
}

export function WhatsNewPage() {
  const [update, setUpdate] = useState<UpdateState | null>(null);
  const [searchParams] = useSearchParams();
  const requestedRelease = searchParams.get("release");
  useEffect(() => {
    markCurrentReleaseViewed().then(() => window.dispatchEvent(new Event("rendernorth-release-viewed")))
      .catch(error => console.error("failed to mark release notes viewed", error));
    getUpdateState().then(setUpdate).catch(() => undefined);
  }, []);

  const selectedBundledRelease = requestedRelease
    ? RELEASE_CATALOG.releases.find(release => release.version === requestedRelease)
    : null;
  const primaryRelease = selectedBundledRelease ?? CURRENT_RELEASE;
  const previous = RELEASE_CATALOG.releases.filter(release => release.version !== primaryRelease.version);
  return <div className="dash release-page">
    {update?.status === "update_available" && !update.releaseInCatalog && <Panel title="Update Available" keel="nominal" className="dash-hero">
      <div className="release-hero">
        <div>
          <div className="release-version">Version {update.latestVersion}</div>
          <h2>{update.latestReleaseTitle ?? "New RenderNorth Industrial release"}</h2>
          {update.releaseDate && <div className="release-meta"><span>Published {new Date(update.releaseDate).toLocaleDateString()}</span></div>}
        </div>
        <button className="target-select enabled" disabled={!update.releaseUrl} onClick={() => update.releaseUrl && openExternalUrl(update.releaseUrl)}>Open Release Notes</button>
      </div>
      <p className="release-summary">{update.summary ?? "Detailed notes for this newer release are available on the official GitHub release page."}</p>
      <p className="data-source">These remote notes are separate from the bundled history for the installed build.</p>
    </Panel>}
    <Panel title="What's New" keel="furnace" className="dash-hero">
      <div className="release-hero">
        <div>
          <div className="release-version">Version {primaryRelease.version}</div>
          <h2>{primaryRelease.name}</h2>
          <div className="release-meta"><span>{primaryRelease.channel}</span><span>Released {primaryRelease.date}</span></div>
        </div>
        <Link className="target-select enabled release-settings-link" to="/settings">About & Support</Link>
      </div>
      <ReleaseBody release={primaryRelease} />
    </Panel>
    <Panel title="Previous Releases" keel="coolant" className="dash-hero">
      {previous.length === 0
        ? <p className="ph-mission">This is the first public RenderNorth Industrial release. Future releases will remain available here.</p>
        : previous.map(release => <details className="release-previous" key={release.version}>
            <summary><strong>{release.version} · {release.name}</strong><span>{release.channel} · {release.date}</span></summary>
            <ReleaseBody release={release} />
          </details>)}
    </Panel>
  </div>;
}
