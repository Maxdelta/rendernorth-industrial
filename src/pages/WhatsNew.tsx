import { useEffect } from "react";
import { Link } from "react-router-dom";
import { Panel } from "../components/Panel";
import { markCurrentReleaseViewed } from "../lib/backend";
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
  useEffect(() => {
    markCurrentReleaseViewed().then(() => window.dispatchEvent(new Event("rendernorth-release-viewed")))
      .catch(error => console.error("failed to mark release notes viewed", error));
  }, []);

  const previous = RELEASE_CATALOG.releases.slice(1);
  return <div className="dash release-page">
    <Panel title="What's New" keel="furnace" className="dash-hero">
      <div className="release-hero">
        <div>
          <div className="release-version">Version {CURRENT_RELEASE.version}</div>
          <h2>{CURRENT_RELEASE.name}</h2>
          <div className="release-meta"><span>{CURRENT_RELEASE.channel}</span><span>Released {CURRENT_RELEASE.date}</span></div>
        </div>
        <Link className="target-select enabled release-settings-link" to="/settings">About & Support</Link>
      </div>
      <ReleaseBody release={CURRENT_RELEASE} />
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
