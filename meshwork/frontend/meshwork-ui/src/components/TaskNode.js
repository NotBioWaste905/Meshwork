import React, { useState, useCallback } from 'react';
import { Handle, Position } from '@xyflow/react';
import styled from 'styled-components';

const NodeContainer = styled.div`
  background: ${props => {
    switch (props.status) {
      case 0: return '#f8f9fa'; // TODO - light gray
      case 1: return '#fff3cd'; // IN_PROGRESS - light yellow
      case 2: return '#d1edff'; // DONE - light blue
      case 3: return '#f8d7da'; // REVIEW - light red
      case 4: return '#f5c6cb'; // BLOCKED - light pink
      default: return '#f8f9fa';
    }
  }};
  border: 2px solid ${props => {
    switch (props.status) {
      case 0: return '#6c757d'; // TODO - gray
      case 1: return '#ffc107'; // IN_PROGRESS - yellow
      case 2: return '#28a745'; // DONE - green
      case 3: return '#dc3545'; // REVIEW - red
      case 4: return '#dc3545'; // BLOCKED - red
      default: return '#6c757d';
    }
  }};
  border-radius: 8px;
  padding: 12px;
  min-width: 200px;
  max-width: 300px;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', sans-serif;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
  cursor: pointer;
  transition: all 0.2s ease;

  &:hover {
    box-shadow: 0 4px 8px rgba(0, 0, 0, 0.15);
    transform: translateY(-1px);
  }

  &.selected {
    border-color: #007bff;
    box-shadow: 0 0 0 2px rgba(0, 123, 255, 0.25);
  }
`;

const TaskHeader = styled.div`
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
`;

const TaskTitle = styled.h4`
  margin: 0;
  font-size: 14px;
  font-weight: 600;
  color: #333;
  word-wrap: break-word;
`;

const StatusBadge = styled.span`
  font-size: 10px;
  padding: 2px 6px;
  border-radius: 12px;
  font-weight: 500;
  color: white;
  background: ${props => {
    switch (props.status) {
      case 0: return '#6c757d'; // TODO
      case 1: return '#ffc107'; // IN_PROGRESS
      case 2: return '#28a745'; // DONE
      case 3: return '#dc3545'; // REVIEW
      case 4: return '#dc3545'; // BLOCKED
      default: return '#6c757d';
    }
  }};
`;

const TaskDescription = styled.p`
  margin: 0 0 8px 0;
  font-size: 12px;
  color: #666;
  line-height: 1.4;
  word-wrap: break-word;
`;

const TaskMeta = styled.div`
  display: flex;
  flex-direction: column;
  gap: 4px;
`;

const MetaRow = styled.div`
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 10px;
  color: #888;
`;

const Tag = styled.span`
  background: #e9ecef;
  color: #495057;
  padding: 1px 4px;
  border-radius: 3px;
  font-size: 9px;
`;

const EditButton = styled.button`
  position: absolute;
  top: 4px;
  right: 4px;
  background: rgba(0, 0, 0, 0.1);
  border: none;
  border-radius: 50%;
  width: 20px;
  height: 20px;
  cursor: pointer;
  font-size: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0;
  transition: opacity 0.2s ease;

  ${NodeContainer}:hover & {
    opacity: 1;
  }

  &:hover {
    background: rgba(0, 0, 0, 0.2);
  }
`;

const statusLabels = {
  0: 'TODO',
  1: 'IN PROGRESS',
  2: 'DONE',
  3: 'REVIEW',
  4: 'BLOCKED'
};

const TaskNode = ({ data, selected }) => {
  const [isEditing, setIsEditing] = useState(false);

  const handleEdit = useCallback((e) => {
    e.stopPropagation();
    setIsEditing(true);
    if (data.onEdit) {
      data.onEdit(data.id);
    }
  }, [data]);

  const handleClick = useCallback((e) => {
    if (data.onClick) {
      data.onClick(data.id);
    }
  }, [data]);

  return (
    <NodeContainer 
      status={data.status} 
      className={selected ? 'selected' : ''}
      onClick={handleClick}
      style={{ position: 'relative' }}
    >
      <Handle
        type="target"
        position={Position.Top}
        style={{
          background: '#555',
          width: 8,
          height: 8,
          borderRadius: '50%'
        }}
      />
      
      <EditButton onClick={handleEdit}>
        ✎
      </EditButton>

      <TaskHeader>
        <TaskTitle>{data.name || 'Untitled Task'}</TaskTitle>
        <StatusBadge status={data.status}>
          {statusLabels[data.status]}
        </StatusBadge>
      </TaskHeader>

      {data.description && (
        <TaskDescription>
          {data.description.length > 100 
            ? `${data.description.substring(0, 100)}...` 
            : data.description
          }
        </TaskDescription>
      )}

      <TaskMeta>
        {data.users && data.users.length > 0 && (
          <MetaRow>
            <span>👤</span>
            <span>{data.users.length} assignee{data.users.length > 1 ? 's' : ''}</span>
          </MetaRow>
        )}
        
        {data.depends_on && data.depends_on.length > 0 && (
          <MetaRow>
            <span>🔗</span>
            <span>{data.depends_on.length} dependency{data.depends_on.length > 1 ? 'ies' : 'y'}</span>
          </MetaRow>
        )}

        {data.tags && data.tags.length > 0 && (
          <MetaRow style={{ flexWrap: 'wrap' }}>
            {data.tags.slice(0, 3).map((tag, index) => (
              <Tag key={index}>{tag}</Tag>
            ))}
            {data.tags.length > 3 && (
              <Tag>+{data.tags.length - 3}</Tag>
            )}
          </MetaRow>
        )}
      </TaskMeta>

      <Handle
        type="source"
        position={Position.Bottom}
        style={{
          background: '#555',
          width: 8,
          height: 8,
          borderRadius: '50%'
        }}
      />
    </NodeContainer>
  );
};

export default TaskNode;