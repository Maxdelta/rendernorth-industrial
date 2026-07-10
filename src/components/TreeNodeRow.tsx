import { useState } from "react";
import { formatQty, type RequirementTreeNode } from "../lib/backend";

interface TreeNodeRowProps {
  node: RequirementTreeNode;
  depth: number;
}

function tone(node: RequirementTreeNode): "nominal" | "furnace" | "alert" {
  if (node.isSatisfied) return "nominal";
  return node.coverageFraction >= 0.6 ? "furnace" : "alert";
}

export function TreeNodeRow({ node, depth }: TreeNodeRowProps) {
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
        <div className="inv-qty">{formatQty(node.neededQuantity)}</div>
        <div className="inv-qty">{formatQty(node.ownedQuantity)}</div>
        <div className="inv-qty">{node.reservedQuantity > 0 ? formatQty(node.reservedQuantity) : "—"}</div>
        <div className="inv-qty">{formatQty(node.availableQuantity)}</div>
        <div className="inv-qty">{Math.round(node.coverageFraction * 100)}%</div>
        <div className={`inv-status ${tone(node)}`}>{node.isSatisfied ? "OK" : "Short"}</div>
      </div>
      {hasChildren && expanded && (
        <div className="tree-children">
          {node.children.map((child) => (
            <TreeNodeRow key={`${child.typeId}-${depth}`} node={child} depth={depth + 1} />
          ))}
        </div>
      )}
    </div>
  );
}
