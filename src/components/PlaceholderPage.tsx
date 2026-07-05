import { Panel } from "./Panel";

interface PlaceholderPageProps {
  title: string;
  mission: string;
  planned: string[];
  sprint: string;
}

export function PlaceholderPage({ title, mission, planned, sprint }: PlaceholderPageProps) {
  return (
    <div className="placeholder">
      <Panel title={title} keel="coolant">
        <p className="ph-mission">{mission}</p>
        <ul className="ph-list">
          {planned.map((item) => (
            <li key={item}>{item}</li>
          ))}
        </ul>
        <div className="ph-sprint">Scheduled: {sprint}</div>
      </Panel>
    </div>
  );
}
