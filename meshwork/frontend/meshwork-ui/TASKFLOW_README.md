# TaskFlow Component Documentation

A comprehensive ReactFlow-based component for visualizing and editing task structures with dependencies, built for the Meshwork project.

## Overview

The TaskFlow component provides an interactive, editable task management interface that displays tasks as nodes in a flow diagram, with edges representing dependencies between tasks. It integrates seamlessly with the backend Task model and provides full CRUD operations.

## Features

- **Visual Task Management**: Display tasks as interactive nodes with color-coded status
- **Dependency Visualization**: Show task dependencies as connected edges
- **Inline Editing**: Edit tasks directly within the flow interface
- **Context Menus**: Right-click operations for creating, editing, and deleting tasks
- **Connection Mode**: Interactive mode for creating dependencies between tasks
- **Auto-layout**: Automatic positioning of tasks with force-directed layout
- **Real-time Updates**: Live synchronization with backend API
- **Responsive Design**: Works on different screen sizes
- **Status Management**: Visual indicators for TODO, IN_PROGRESS, DONE, REVIEW, BLOCKED

## Components

### 1. TaskFlow (Main Component)

The primary component that orchestrates the entire task flow interface.

```jsx
import TaskFlow from './components/TaskFlow';

<TaskFlow
  initialTasks={tasks}
  onTaskCreate={handleTaskCreate}
  onTaskUpdate={handleTaskUpdate}
  onTaskDelete={handleTaskDelete}
  loading={false}
/>
```

**Props:**
- `initialTasks`: Array of task objects
- `onTaskCreate`: Callback for creating new tasks
- `onTaskUpdate`: Callback for updating existing tasks
- `onTaskDelete`: Callback for deleting tasks
- `loading`: Boolean to show loading state

### 2. TaskNode

Custom ReactFlow node component representing individual tasks.

**Features:**
- Color-coded by status
- Displays task name, description (truncated), and metadata
- Shows assigned users, dependencies count, and tags
- Hover effects and selection states
- Edit button overlay

### 3. TaskEditor

Modal component for creating and editing tasks.

**Features:**
- Form validation
- Tag and user management with chip interface
- Status dropdown
- Dependency management (future enhancement)
- Responsive design

### 4. ContextMenu

Right-click context menu for task operations.

**Features:**
- Edit task
- Duplicate task
- Delete task
- Create new task
- Connect tasks (dependency creation)

## Backend Integration

### Task Model Mapping

The component maps to the backend Task model:

```python
class Task(BaseModel):
    id: str
    name: str
    description: str
    depends_on: list[str]
    users: list[str]
    tags: list[str]
    status: Status  # 0=TODO, 1=IN_PROGRESS, 2=DONE, 3=REVIEW, 4=BLOCKED
```

### API Service

Uses `taskApi.js` for backend communication:

```javascript
import taskApi from './services/taskApi';

// Get all tasks
const tasks = await taskApi.getTasks();

// Create task
const newTask = await taskApi.createTask(taskData);

// Update task
const updatedTask = await taskApi.updateTask(taskId, taskData);

// Delete task
await taskApi.deleteTask(taskId);
```

## Usage Examples

### Basic Setup

```jsx
import React, { useState, useEffect } from 'react';
import TaskFlow from './components/TaskFlow';
import taskApi from './services/taskApi';

function App() {
  const [tasks, setTasks] = useState([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    loadTasks();
  }, []);

  const loadTasks = async () => {
    try {
      const fetchedTasks = await taskApi.getTasks();
      setTasks(fetchedTasks);
    } catch (error) {
      console.error('Failed to load tasks:', error);
    } finally {
      setLoading(false);
    }
  };

  const handleTaskCreate = async (taskData) => {
    const newTask = await taskApi.createTask(taskData);
    setTasks(prev => [...prev, newTask]);
  };

  const handleTaskUpdate = async (taskData) => {
    const updatedTask = await taskApi.updateTask(taskData.id, taskData);
    setTasks(prev => prev.map(task => 
      task.id === taskData.id ? updatedTask : task
    ));
  };

  const handleTaskDelete = async (taskId) => {
    await taskApi.deleteTask(taskId);
    setTasks(prev => prev.filter(task => task.id !== taskId));
  };

  return (
    <TaskFlow
      initialTasks={tasks}
      onTaskCreate={handleTaskCreate}
      onTaskUpdate={handleTaskUpdate}
      onTaskDelete={handleTaskDelete}
      loading={loading}
    />
  );
}
```

### Sample Task Data

```javascript
const sampleTask = {
  id: 'task-1',
  name: 'Backend API Development',
  description: 'Develop REST API endpoints for task management',
  status: 1, // IN_PROGRESS
  users: ['john.doe', 'jane.smith'],
  tags: ['backend', 'api', 'python'],
  depends_on: ['task-planning']
};
```

## Customization

### Styling

The component uses styled-components for styling. Key style customizations:

```css
/* TaskNode colors by status */
.task-node-todo { background: #f8f9fa; border: #6c757d; }
.task-node-progress { background: #fff3cd; border: #ffc107; }
.task-node-done { background: #d1edff; border: #28a745; }
.task-node-review { background: #f8d7da; border: #dc3545; }
.task-node-blocked { background: #f5c6cb; border: #dc3545; }
```

### Node Types

Extend with custom node types:

```javascript
const customNodeTypes = {
  taskNode: TaskNode,
  milestoneNode: MilestoneNode, // Custom milestone node
  groupNode: GroupNode, // Custom group node
};

<ReactFlow nodeTypes={customNodeTypes} />
```

## Utilities

### Task Utilities (`utils/taskUtils.js`)

Helper functions for task management:

- `generateTaskId()`: Generate unique task IDs
- `validateTask(task)`: Validate task data
- `canMoveToStatus(task, status)`: Check status transitions
- `getDependentTasks(taskId, allTasks)`: Find dependent tasks
- `hasCircularDependency()`: Detect circular dependencies
- `calculateTaskProgress(tasks)`: Calculate completion percentage
- `filterTasks(tasks, criteria)`: Filter tasks by criteria
- `sortTasksByDependencies(tasks)`: Topological sort
- `getTaskStatistics(tasks)`: Generate task statistics

## Keyboard Shortcuts

- `Ctrl/Cmd + Click`: Multi-select nodes
- `Delete`: Delete selected nodes
- `Escape`: Cancel current operation
- `Space + Drag`: Pan the canvas
- `Ctrl/Cmd + Scroll`: Zoom in/out

## Browser Compatibility

- Chrome 60+
- Firefox 60+
- Safari 12+
- Edge 79+

## Performance Considerations

- Virtualization for large datasets (500+ tasks)
- Debounced API calls for real-time updates
- Memoized components to prevent unnecessary re-renders
- Lazy loading for complex task details

## Troubleshooting

### Common Issues

1. **Tasks not displaying**: Check `initialTasks` prop and console for errors
2. **API calls failing**: Verify backend URL in `REACT_APP_API_URL`
3. **Dependencies not showing**: Ensure task IDs in `depends_on` exist
4. **Performance issues**: Consider implementing pagination for large datasets

### Debug Mode

Enable debug logging:

```javascript
localStorage.setItem('taskflow-debug', 'true');
```

## Contributing

When contributing to the TaskFlow component:

1. Follow the existing component structure
2. Add proper TypeScript types (future enhancement)
3. Include unit tests for new features
4. Update this documentation
5. Ensure responsive design compatibility

## Dependencies

- React 18+
- @xyflow/react 12+
- styled-components 6+
- React hooks for state management

## Future Enhancements

- [ ] Drag & drop file attachments
- [ ] Real-time collaboration
- [ ] Gantt chart view
- [ ] Task templates
- [ ] Bulk operations
- [ ] Advanced filtering
- [ ] Timeline view
- [ ] Export functionality
- [ ] Mobile app support
- [ ] Offline mode