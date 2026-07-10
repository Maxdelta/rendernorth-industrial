import { useEffect, useState } from "react";
import { searchEveTypes, getLatestImport, type TypeSearchResult } from "../lib/backend";

interface BuildTargetSearchProps {
  onSelect: (result: TypeSearchResult | null) => void;
  selected: TypeSearchResult | null;
}

/** Fast text search over imported static data. No hardcoded list of demo targets. */
export function BuildTargetSearch({ onSelect, selected }: BuildTargetSearchProps) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<TypeSearchResult[]>([]);
  const [searched, setSearched] = useState(false);
  const [hasStaticData, setHasStaticData] = useState<boolean | null>(null);

  useEffect(() => {
    getLatestImport().then((s) => setHasStaticData(s !== null));
  }, []);

  useEffect(() => {
    if (query.trim().length < 2) {
      setResults([]);
      setSearched(false);
      return;
    }
    let live = true;
    const handle = setTimeout(() => {
      searchEveTypes(query.trim(), 25).then((r) => {
        if (live) {
          setResults(r);
          setSearched(true);
        }
      });
    }, 200);
    return () => {
      live = false;
      clearTimeout(handle);
    };
  }, [query]);

  if (hasStaticData === false) {
    return (
      <div className="bts-empty-state">
        No static data imported yet. Go to <strong>Settings → Static Data</strong> to import a local SDE CSV bundle
        before searching for a build target.
      </div>
    );
  }

  if (selected) {
    return (
      <div className="bts-selected">
        <div>
          <div className="bts-selected-name">{selected.name}</div>
          <div className="bts-selected-meta">
            {selected.categoryName} &rsaquo; {selected.groupName}
            {selected.isManufacturable ? " · manufacturable" : " · not manufacturable"}
          </div>
        </div>
        <button className="target-select enabled" onClick={() => onSelect(null)}>
          Change
        </button>
      </div>
    );
  }

  return (
    <div className="bts-wrap">
      <input
        className="sd-path-input"
        type="text"
        placeholder="Search imported types by name…"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
      />
      {searched && (
        <div className="bts-results">
          {results.length === 0 ? (
            <div className="ph-mission" style={{ padding: 10 }}>No matching type found in imported static data.</div>
          ) : (
            results.map((r) => (
              <button
                key={r.typeId}
                className="bts-result-row"
                disabled={!r.isManufacturable}
                title={r.isManufacturable ? undefined : "Not manufacturable — no blueprint produces this type in the imported data"}
                onClick={() => r.isManufacturable && onSelect(r)}
              >
                <span className="bts-result-name">{r.name}</span>
                <span className="bts-result-meta">
                  {r.categoryName} &rsaquo; {r.groupName}
                </span>
                <span className={r.isManufacturable ? "op-status nominal" : "op-status coolant"}>
                  {r.isManufacturable ? "Manufacturable" : "Not manufacturable"}
                </span>
              </button>
            ))
          )}
        </div>
      )}
    </div>
  );
}
