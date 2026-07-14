import { useState } from "react";
import { formatQty, formatVolume, type RequirementTreeNode } from "../lib/backend";

interface TreeNodeRowProps {
  node: RequirementTreeNode;
  depth: number;
  validationMode?: boolean;
}

function tone(node: RequirementTreeNode): "nominal" | "furnace" | "alert" {
  if (node.isSatisfied) return "nominal";
  return node.coverageFraction >= 0.6 ? "furnace" : "alert";
}

export function TreeNodeRow({ node, depth, validationMode = false }: TreeNodeRowProps) {
  const [expanded, setExpanded] = useState(depth < 2);
  const hasChildren = node.children.length > 0;

  return (
    <div className="tree-node">
      <div className="tree-node-row" style={{ paddingLeft: depth * 4 }}>
        <button className="tree-toggle" onClick={() => setExpanded((v) => !v)} disabled={!hasChildren}>
          {hasChildren ? (expanded ? "▾" : "▸") : "·"}
        </button>
        <div className="inv-name">
          {node.typeName}
          {!node.isLeaf && (
            <span className="bts-result-meta" style={{ marginLeft: 8 }}>
              {node.runs} run{node.runs === 1 ? "" : "s"} · ME {node.meApplied} ({node.meSource})
            </span>
          )}
        </div>
        <div className="inv-qty">
          {formatQty(node.neededQuantity)}
          <div className="bts-result-meta">{formatVolume(node.requiredVolumeM3)}</div>
        </div>
        <div className="inv-qty">
          {formatQty(node.ownedQuantity)}
          {node.ownedQuantity > 0 && <span className="bts-result-meta"> · {node.ownedSource}</span>}
          <div className="bts-result-meta">{formatVolume(node.ownedVolumeM3)}</div>
        </div>
        <div className="inv-qty">{node.reservedQuantity > 0 ? formatQty(node.reservedQuantity) : "—"}</div>
        <div className="inv-qty">{formatQty(node.availableQuantity)}</div>
        <div className="inv-qty">{Math.round(node.coverageFraction * 100)}%</div>
        <div className={`inv-status ${tone(node)}`}>{node.isSatisfied ? "OK" : "Short"}<div className="bts-result-meta">Missing {formatVolume(node.missingVolumeM3)}</div></div>
      </div>
      {validationMode && (
        <div className="validation-detail" style={{ paddingLeft: depth * 4 + 20 }}>
          <span>Blueprint: {node.blueprintTypeId !== null ? `type ${node.blueprintTypeId}` : "— (leaf, no blueprint)"}</span>
          <span>Activity: {node.activity}</span>
          <span>Required: {formatQty(node.neededQuantity)}</span>
          <span>Required volume: {formatVolume(node.requiredVolumeM3)}</span>
          <span>Owned volume: {formatVolume(node.ownedVolumeM3)}</span>
          <span>Missing volume: {formatVolume(node.missingVolumeM3)}</span>
          <span>Path: {node.calculationPath}</span>
        </div>
      )}
      {hasChildren && expanded && (
        <div className="tree-children">
          {node.children.map((child) => (
            <TreeNodeRow key={`${child.typeId}-${depth}`} node={child} depth={depth + 1} validationMode={validationMode} />
          ))}
        </div>
      )}
    </div>
  );
}
