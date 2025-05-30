import React, { useCallback } from "react";
import { useReactFlow } from "@xyflow/react";
import styled from "styled-components";

const MenuContainer = styled.div`
  position: absolute;
  z-index: 1000;
  background: white;
  border: 1px solid #e0e0e0;
  border-radius: 6px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  padding: 4px 0;
  min-width: 180px;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', sans-serif;
`;

const MenuItem = styled.button`
  width: 100%;
  text-align: left;
  padding: 8px 16px;
  background: none;
  border: none;
  cursor: pointer;
  font-size: 13px;
  color: ${props => props.danger ? '#dc3545' : '#333'};
  display: flex;
  align-items: center;
  gap: 8px;
  
  &:hover {
    background: ${props => props.danger ? '#f8d7da' : '#f8f9fa'};
  }
  
  &:disabled {
    color: #999;
    cursor: not-allowed;
    
    &:hover {
      background: none;
    }
  }
`;

const MenuSeparator = styled.div`
  height: 1px;
  background: #e0e0e0;
  margin: 4px 0;
`;

const MenuHeader = styled.div`
  padding: 4px 16px;
  font-size: 11px;
  color: #666;
  font-weight: 500;
  text-transform: uppercase;
  letter-spacing: 0.5px;
`;

export default function ContextMenu({
  id,
  top,
  left,
  right,
  bottom,
  onClose,
  onEdit,
  onCreateTask,
  onDeleteTask,
  onDuplicateTask,
  onConnectTask,
  isConnecting,
  ...props
}) {
  const { getNode, setNodes, addNodes, setEdges, screenToFlowPosition } =
    useReactFlow();

  const handleEdit = useCallback(() => {
    if (onEdit && id) {
      onEdit(id);
    }
    onClose?.();
  }, [id, onEdit, onClose]);

  const handleDuplicate = useCallback(() => {
    if (!id) return;

    const node = getNode(id);
    if (!node) return;

    const position = {
      x: node.position.x + 50,
      y: node.position.y + 50,
    };

    const newTaskData = {
      ...node.data,
      name: `${node.data.name} (Copy)`,
      id: `task-${Date.now()}`,
      depends_on: [], // Clear dependencies for the copy
    };

    const newNode = {
      ...node,
      selected: false,
      dragging: false,
      id: newTaskData.id,
      position,
      data: newTaskData,
    };

    addNodes(newNode);
    
    if (onDuplicateTask) {
      onDuplicateTask(newTaskData);
    }
    
    onClose?.();
  }, [id, getNode, addNodes, onDuplicateTask, onClose]);

  const handleCreateTask = useCallback(() => {
    const position = screenToFlowPosition({
      x: left || 0,
      y: top || 0,
    });

    const newTaskId = `task-${Date.now()}`;
    const newTaskData = {
      id: newTaskId,
      name: "New Task",
      description: "",
      status: 0,
      users: [],
      tags: [],
      depends_on: [],
    };

    const newNode = {
      id: newTaskId,
      type: 'taskNode',
      position,
      data: newTaskData,
    };

    addNodes(newNode);
    
    if (onCreateTask) {
      onCreateTask(newTaskData);
    }
    
    onClose?.();
  }, [top, left, screenToFlowPosition, addNodes, onCreateTask, onClose]);

  const handleDelete = useCallback(() => {
    if (!id) return;

    setNodes((nodes) => nodes.filter((node) => node.id !== id));
    setEdges((edges) =>
      edges.filter((edge) => edge.source !== id && edge.target !== id)
    );
    
    if (onDeleteTask) {
      onDeleteTask(id);
    }
    
    onClose?.();
  }, [id, setNodes, setEdges, onDeleteTask, onClose]);

  const handleConnect = useCallback(() => {
    if (onConnectTask && id) {
      onConnectTask(id);
    }
    onClose?.();
  }, [id, onConnectTask, onClose]);

  return (
    <MenuContainer
      style={{
        top,
        left,
        right,
        bottom,
      }}
      {...props}
    >
      {id ? (
        <>
          <MenuHeader>Task Actions</MenuHeader>
          <MenuItem onClick={handleEdit}>
            <span>✎</span>
            Edit Task
          </MenuItem>
          <MenuItem onClick={handleDuplicate}>
            <span>⧉</span>
            Duplicate Task
          </MenuItem>
          <MenuItem 
            onClick={handleConnect}
            disabled={isConnecting}
          >
            <span>🔗</span>
            {isConnecting ? 'Connecting...' : 'Connect to Task'}
          </MenuItem>
          <MenuSeparator />
          <MenuItem onClick={handleDelete} danger>
            <span>🗑</span>
            Delete Task
          </MenuItem>
        </>
      ) : (
        <>
          <MenuHeader>Create</MenuHeader>
          <MenuItem onClick={handleCreateTask}>
            <span>➕</span>
            New Task
          </MenuItem>
        </>
      )}
    </MenuContainer>
  );
}