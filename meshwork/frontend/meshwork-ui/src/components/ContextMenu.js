import { useCallback } from "react";
import { useReactFlow } from "@xyflow/react";

export default function ContextMenu({
  id,
  top,
  left,
  right,
  bottom,
  onClose,
  ...props
}) {
  const { getNode, setNodes, addNodes, setEdges, screenToFlowPosition } =
    useReactFlow();

  // Handle duplication of an existing node
  const duplicateNode = useCallback(() => {
    if (!id) return;

    const node = getNode(id);
    if (!node) return;

    const position = {
      x: node.position.x + 50,
      y: node.position.y + 50,
    };

    addNodes({
      ...node,
      selected: false,
      dragging: false,
      id: `${node.id}-copy-${Date.now()}`,
      position,
    });
    onClose?.();
  }, [id, getNode, addNodes, onClose]);

  // Handle creation of a new node at clicked position
  const createNode = useCallback(() => {
    const position = screenToFlowPosition({
      x: left,
      y: top,
    });

    const newNode = {
      id: `node-${Date.now()}`,
      position,
      data: { label: `New Node` },
    };

    addNodes(newNode);
    onClose?.();
  }, [top, left, screenToFlowPosition, addNodes, onClose]);

  // Handle deletion of an existing node
  const deleteNode = useCallback(() => {
    if (!id) return;

    setNodes((nodes) => nodes.filter((node) => node.id !== id));
    setEdges((edges) =>
      edges.filter((edge) => edge.source !== id && edge.target !== id),
    );
    onClose?.();
  }, [id, setNodes, setEdges, onClose]);

  return (
    <div
      style={{
        top,
        left,
        right,
        bottom,
        position: "absolute",
        zIndex: 10,
        background: "white",
        border: "1px solid #ccc",
        borderRadius: "4px",
        boxShadow: "0 2px 10px rgba(0,0,0,0.1)",
        padding: "8px 0",
        minWidth: "160px",
      }}
      className="context-menu"
      {...props}
    >
      {id ? (
        <>
          <p style={{ margin: "0.5em" }}>
            <small>node: {id}</small>
          </p>
          <button
            onClick={duplicateNode}
            style={{
              width: "100%",
              textAlign: "left",
              padding: "8px 16px",
              background: "none",
              border: "none",
              cursor: "pointer",
            }}
          >
            Duplicate node
          </button>
          <button
            onClick={deleteNode}
            style={{
              width: "100%",
              textAlign: "left",
              padding: "8px 16px",
              background: "none",
              border: "none",
              cursor: "pointer",
              color: "red",
            }}
          >
            Delete node
          </button>
        </>
      ) : (
        <>
          <button
            onClick={createNode}
            style={{
              width: "100%",
              textAlign: "left",
              padding: "8px 16px",
              background: "none",
              border: "none",
              cursor: "pointer",
            }}
          >
            Create new node
          </button>
        </>
      )}
    </div>
  );
}
