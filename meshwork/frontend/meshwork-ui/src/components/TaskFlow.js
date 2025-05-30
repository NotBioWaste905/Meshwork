import React, { useState, useCallback, useRef, useEffect } from 'react';
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  useNodesState,
  useEdgesState,
  addEdge,
  useReactFlow,
  ReactFlowProvider,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import styled from 'styled-components';

import TaskNode from './TaskNode';
import TaskEditor from './TaskEditor';
import ContextMenu from './ContextMenu';
import { tasksToReactFlowElements } from '../utils/taskUtils';

const FlowContainer = styled.div`
  width: 100%;
  height: 100vh;
  position: relative;
  background: #f8f9fa;
`;

const LoadingOverlay = styled.div`
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background: rgba(255, 255, 255, 0.8);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 999;
  font-size: 16px;
  color: #666;
`;

const Toolbar = styled.div`
  position: absolute;
  top: 20px;
  left: 20px;
  z-index: 100;
  display: flex;
  flex-direction: column;
  gap: 8px;
  background: white;
  padding: 12px;
  border-radius: 8px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  min-width: 200px;
`;

const ToolbarSection = styled.div`
  display: flex;
  flex-direction: column;
  gap: 4px;
`;

const ToolbarLabel = styled.label`
  font-size: 12px;
  font-weight: 500;
  color: #666;
  margin-bottom: 2px;
`;

const ToolbarButton = styled.button`
  background: ${props => props.active ? '#007bff' : 'white'};
  color: ${props => props.active ? 'white' : '#333'};
  border: 1px solid ${props => props.active ? '#007bff' : '#ddd'};
  border-radius: 4px;
  padding: 6px 12px;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.2s ease;
  
  &:hover {
    background: ${props => props.active ? '#0056b3' : '#f8f9fa'};
  }
  
  &:disabled {
    background: #f8f9fa;
    color: #999;
    cursor: not-allowed;
  }
`;

const LayoutSelect = styled.select`
  padding: 6px 8px;
  border: 1px solid #ddd;
  border-radius: 4px;
  font-size: 12px;
  background: white;
  
  &:focus {
    outline: none;
    border-color: #007bff;
  }
`;

const StatusIndicator = styled.div`
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: #666;
  padding: 4px 0;
`;

const StatusDot = styled.div`
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: ${props => props.connected ? '#28a745' : '#dc3545'};
`;

const nodeTypes = {
  taskNode: TaskNode,
};

const defaultEdgeOptions = {
  animated: true,
  style: {
    stroke: '#666',
    strokeWidth: 2,
  },
  markerEnd: {
    type: 'arrowclosed',
    color: '#666',
  },
};

const TaskFlowInner = ({ 
  initialTasks = [], 
  onTaskUpdate, 
  onTaskCreate, 
  onTaskDelete,
  loading = false 
}) => {
  const reactFlowWrapper = useRef(null);
  const connectingNodeId = useRef(null);
  const [nodes, setNodes, onNodesChange] = useNodesState([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);
  const [isEditorOpen, setIsEditorOpen] = useState(false);
  const [editingTask, setEditingTask] = useState(null);
  const [contextMenu, setContextMenu] = useState(null);
  const [isConnecting, setIsConnecting] = useState(false);
  const [layoutType, setLayoutType] = useState('smart');
  const [isConnected, setIsConnected] = useState(false);
  const [isLayouting, setIsLayouting] = useState(false);
  
  const { screenToFlowPosition, getNode, fitView } = useReactFlow();

  // Check backend connection
  useEffect(() => {
    const checkConnection = async () => {
      try {
        const response = await fetch('http://localhost:8000/health');
        setIsConnected(response.ok);
      } catch (error) {
        setIsConnected(false);
      }
    };
    
    checkConnection();
    const interval = setInterval(checkConnection, 30000); // Check every 30 seconds
    
    return () => clearInterval(interval);
  }, []);

  // Initialize and layout nodes from tasks
  useEffect(() => {
    if (initialTasks.length > 0) {
      setIsLayouting(true);
      
      // Small delay to show loading state
      setTimeout(() => {
        const { nodes: layoutNodes, edges: layoutEdges } = tasksToReactFlowElements(
          initialTasks, 
          handleEditTask, 
          handleTaskClick,
          layoutType
        );
        
        setNodes(layoutNodes);
        setEdges(layoutEdges);
        setIsLayouting(false);
        
        // Fit view after layout
        setTimeout(() => {
          fitView({ padding: 0.1 });
        }, 100);
      }, 300);
    }
  }, [initialTasks, layoutType]);

  const handleTaskClick = useCallback((taskId) => {
    if (isConnecting && connectingNodeId.current && connectingNodeId.current !== taskId) {
      // Create dependency connection
      const sourceTask = getNode(connectingNodeId.current);
      const targetTask = getNode(taskId);
      
      if (sourceTask && targetTask) {
        // Check for circular dependency
        const wouldCreateCycle = checkForCircularDependency(
          taskId, 
          connectingNodeId.current, 
          initialTasks
        );
        
        if (wouldCreateCycle) {
          alert('Cannot create dependency: this would create a circular dependency!');
        } else {
          // Add edge
          const newEdge = {
            id: `${connectingNodeId.current}-${taskId}`,
            source: connectingNodeId.current,
            target: taskId,
            type: 'smoothstep',
            animated: true,
            style: {
              stroke: '#666',
              strokeWidth: 2,
            },
            markerEnd: {
              type: 'arrowclosed',
              color: '#666',
            },
          };
          
          setEdges(prev => addEdge(newEdge, prev));
          
          // Update task dependencies
          const updatedTargetTask = {
            ...targetTask.data,
            depends_on: [...(targetTask.data.depends_on || []), connectingNodeId.current]
          };
          
          setNodes(prev => prev.map(node => 
            node.id === taskId 
              ? { ...node, data: updatedTargetTask }
              : node
          ));
          
          if (onTaskUpdate) {
            onTaskUpdate(updatedTargetTask);
          }
        }
      }
      
      setIsConnecting(false);
      connectingNodeId.current = null;
    }
  }, [isConnecting, getNode, setEdges, setNodes, onTaskUpdate, initialTasks]);

  const checkForCircularDependency = (taskId, newDependencyId, tasks) => {
    const visited = new Set();
    
    const dfs = (currentId) => {
      if (visited.has(currentId)) return false;
      if (currentId === taskId) return true;
      
      visited.add(currentId);
      const task = tasks.find(t => t.id === currentId);
      
      if (task && task.depends_on) {
        for (const depId of task.depends_on) {
          if (dfs(depId)) return true;
        }
      }
      
      return false;
    };
    
    return dfs(newDependencyId);
  };

  const handleEditTask = useCallback((taskId) => {
    const node = getNode(taskId);
    if (node) {
      setEditingTask(node.data);
      setIsEditorOpen(true);
    }
  }, [getNode]);

  const handleCreateTask = useCallback((position) => {
    const newTask = {
      id: `task-${Date.now()}`,
      name: "New Task",
      description: "",
      status: 0,
      users: [],
      tags: [],
      depends_on: [],
    };
    
    setEditingTask({ ...newTask, position });
    setIsEditorOpen(true);
  }, []);

  const handleSaveTask = useCallback((taskData) => {
    const isNew = !getNode(taskData.id);
    
    if (isNew) {
      // Create new node with current layout
      const { nodes: newNodes } = tasksToReactFlowElements(
        [...initialTasks, taskData],
        handleEditTask,
        handleTaskClick,
        layoutType
      );
      
      const newNode = newNodes.find(n => n.id === taskData.id);
      if (newNode && editingTask?.position) {
        newNode.position = editingTask.position;
      }
      
      setNodes(prev => [...prev, newNode]);
      
      if (onTaskCreate) {
        onTaskCreate(taskData);
      }
    } else {
      // Update existing node
      setNodes(prev => prev.map(node => 
        node.id === taskData.id 
          ? { 
              ...node, 
              data: { 
                ...taskData, 
                onEdit: handleEditTask,
                onClick: handleTaskClick,
              } 
            }
          : node
      ));
      
      if (onTaskUpdate) {
        onTaskUpdate(taskData);
      }
    }
    
    setIsEditorOpen(false);
    setEditingTask(null);
  }, [getNode, editingTask, handleEditTask, handleTaskClick, setNodes, onTaskCreate, onTaskUpdate, initialTasks, layoutType]);

  const handleDeleteTask = useCallback((taskId) => {
    setNodes(prev => prev.filter(node => node.id !== taskId));
    setEdges(prev => prev.filter(edge => edge.source !== taskId && edge.target !== taskId));
    
    if (onTaskDelete) {
      onTaskDelete(taskId);
    }
  }, [setNodes, setEdges, onTaskDelete]);

  const handleConnect = useCallback((params) => {
    const newEdge = {
      ...params,
      type: 'smoothstep',
      animated: true,
      style: {
        stroke: '#666',
        strokeWidth: 2,
      },
      markerEnd: {
        type: 'arrowclosed',
        color: '#666',
      },
    };
    
    setEdges(prev => addEdge(newEdge, prev));
    
    // Update task dependencies
    const targetNode = getNode(params.target);
    if (targetNode) {
      const updatedTask = {
        ...targetNode.data,
        depends_on: [...(targetNode.data.depends_on || []), params.source]
      };
      
      setNodes(prev => prev.map(node => 
        node.id === params.target 
          ? { ...node, data: updatedTask }
          : node
      ));
      
      if (onTaskUpdate) {
        onTaskUpdate(updatedTask);
      }
    }
  }, [getNode, setEdges, setNodes, onTaskUpdate]);

  const handleNodeContextMenu = useCallback((event, node) => {
    event.preventDefault();
    setContextMenu({
      id: node.id,
      top: event.clientY,
      left: event.clientX,
    });
  }, []);

  const handlePaneContextMenu = useCallback((event) => {
    event.preventDefault();
    setContextMenu({
      top: event.clientY,
      left: event.clientX,
    });
  }, []);

  const handleStartConnection = useCallback((taskId) => {
    setIsConnecting(true);
    connectingNodeId.current = taskId;
  }, []);

  const handleCancelConnection = useCallback(() => {
    setIsConnecting(false);
    connectingNodeId.current = null;
  }, []);

  const handleLayoutChange = useCallback((newLayoutType) => {
    setLayoutType(newLayoutType);
  }, []);

  const handleReLayout = useCallback(() => {
    if (initialTasks.length > 0) {
      setIsLayouting(true);
      setTimeout(() => {
        const { nodes: layoutNodes, edges: layoutEdges } = tasksToReactFlowElements(
          initialTasks, 
          handleEditTask, 
          handleTaskClick,
          layoutType
        );
        
        setNodes(layoutNodes);
        setEdges(layoutEdges);
        setIsLayouting(false);
        
        setTimeout(() => {
          fitView({ padding: 0.1 });
        }, 100);
      }, 100);
    }
  }, [initialTasks, layoutType, handleEditTask, handleTaskClick, setNodes, setEdges, fitView]);

  const closeContextMenu = useCallback(() => {
    setContextMenu(null);
  }, []);

  const handleCloseEditor = useCallback(() => {
    setIsEditorOpen(false);
    setEditingTask(null);
  }, []);

  return (
    <FlowContainer ref={reactFlowWrapper}>
      {(loading || isLayouting) && (
        <LoadingOverlay>
          {loading ? 'Loading tasks...' : 'Applying layout...'}
        </LoadingOverlay>
      )}
      
      <Toolbar>
        <StatusIndicator>
          <StatusDot connected={isConnected} />
          {isConnected ? 'Backend Connected' : 'Backend Offline'}
        </StatusIndicator>
        
        <ToolbarSection>
          <ToolbarButton onClick={() => handleCreateTask()}>
            ➕ New Task
          </ToolbarButton>
          <ToolbarButton 
            active={isConnecting}
            onClick={isConnecting ? handleCancelConnection : undefined}
          >
            {isConnecting ? '❌ Cancel Connect' : '🔗 Connect Mode'}
          </ToolbarButton>
        </ToolbarSection>
        
        <ToolbarSection>
          <ToolbarLabel>Layout Algorithm</ToolbarLabel>
          <LayoutSelect 
            value={layoutType} 
            onChange={(e) => handleLayoutChange(e.target.value)}
          >
            <option value="smart">Smart Layout</option>
            <option value="hierarchical">Hierarchical</option>
            <option value="force">Force Directed</option>
          </LayoutSelect>
          <ToolbarButton onClick={handleReLayout}>
            🔄 Re-layout
          </ToolbarButton>
        </ToolbarSection>
      </Toolbar>

      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={handleConnect}
        onNodeContextMenu={handleNodeContextMenu}
        onPaneContextMenu={handlePaneContextMenu}
        nodeTypes={nodeTypes}
        defaultEdgeOptions={defaultEdgeOptions}
        fitView
        fitViewOptions={{
          padding: 0.1,
        }}
        minZoom={0.1}
        maxZoom={2}
      >
        <Background 
          color="#f0f0f0" 
          gap={20} 
          size={1}
          variant="dots"
        />
        <Controls 
          showInteractive={false}
          showZoom={true}
          showFitView={true}
        />
        <MiniMap 
          nodeColor={(node) => {
            switch (node.data.status) {
              case 0: return '#6c757d';
              case 1: return '#ffc107';
              case 2: return '#28a745';
              case 3: return '#dc3545';
              case 4: return '#dc3545';
              default: return '#6c757d';
            }
          }}
          pannable
          zoomable
          style={{
            backgroundColor: '#f8f9fa',
          }}
        />
      </ReactFlow>

      {contextMenu && (
        <ContextMenu
          {...contextMenu}
          onClose={closeContextMenu}
          onEdit={handleEditTask}
          onCreateTask={() => {
            const position = screenToFlowPosition({
              x: contextMenu.left,
              y: contextMenu.top,
            });
            setEditingTask({ position });
            setIsEditorOpen(true);
            closeContextMenu();
          }}
          onDeleteTask={handleDeleteTask}
          onConnectTask={handleStartConnection}
          isConnecting={isConnecting}
        />
      )}

      <TaskEditor
        task={editingTask}
        isOpen={isEditorOpen}
        onClose={handleCloseEditor}
        onSave={handleSaveTask}
        allTasks={nodes.map(node => node.data)}
      />
    </FlowContainer>
  );
};

const TaskFlow = (props) => {
  return (
    <ReactFlowProvider>
      <TaskFlowInner {...props} />
    </ReactFlowProvider>
  );
};

export default TaskFlow;