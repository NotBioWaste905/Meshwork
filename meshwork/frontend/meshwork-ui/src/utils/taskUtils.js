// Task Status Constants
export const TaskStatus = {
  TODO: 0,
  IN_PROGRESS: 1,
  DONE: 2,
  REVIEW: 3,
  BLOCKED: 4
};

export const StatusLabels = {
  [TaskStatus.TODO]: 'TODO',
  [TaskStatus.IN_PROGRESS]: 'IN PROGRESS',
  [TaskStatus.DONE]: 'DONE',
  [TaskStatus.REVIEW]: 'REVIEW',
  [TaskStatus.BLOCKED]: 'BLOCKED'
};

export const StatusColors = {
  [TaskStatus.TODO]: {
    background: '#f8f9fa',
    border: '#6c757d',
    text: '#333'
  },
  [TaskStatus.IN_PROGRESS]: {
    background: '#fff3cd',
    border: '#ffc107',
    text: '#333'
  },
  [TaskStatus.DONE]: {
    background: '#d1edff',
    border: '#28a745',
    text: '#333'
  },
  [TaskStatus.REVIEW]: {
    background: '#f8d7da',
    border: '#dc3545',
    text: '#333'
  },
  [TaskStatus.BLOCKED]: {
    background: '#f5c6cb',
    border: '#dc3545',
    text: '#333'
  }
};

// Generate unique task ID
export const generateTaskId = () => {
  return `task-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
};

// Create a new empty task
export const createEmptyTask = (overrides = {}) => {
  return {
    id: generateTaskId(),
    name: '',
    description: '',
    status: TaskStatus.TODO,
    users: [],
    tags: [],
    depends_on: [],
    ...overrides
  };
};

// Validate task data
export const validateTask = (task) => {
  const errors = [];
  
  if (!task.name || task.name.trim() === '') {
    errors.push('Task name is required');
  }
  
  if (task.name && task.name.length > 100) {
    errors.push('Task name must be less than 100 characters');
  }
  
  if (task.description && task.description.length > 1000) {
    errors.push('Task description must be less than 1000 characters');
  }
  
  if (!Object.values(TaskStatus).includes(task.status)) {
    errors.push('Invalid task status');
  }
  
  if (!Array.isArray(task.users)) {
    errors.push('Users must be an array');
  }
  
  if (!Array.isArray(task.tags)) {
    errors.push('Tags must be an array');
  }
  
  if (!Array.isArray(task.depends_on)) {
    errors.push('Dependencies must be an array');
  }
  
  return {
    isValid: errors.length === 0,
    errors
  };
};

// Check if a task can be moved to a specific status
export const canMoveToStatus = (task, newStatus, allTasks = []) => {
  switch (newStatus) {
    case TaskStatus.IN_PROGRESS:
      // Can move to in progress if all dependencies are done
      if (task.depends_on && task.depends_on.length > 0) {
        const dependencies = allTasks.filter(t => task.depends_on.includes(t.id));
        return dependencies.every(dep => dep.status === TaskStatus.DONE);
      }
      return true;
      
    case TaskStatus.DONE:
      // Can only mark as done if currently in progress or review
      return task.status === TaskStatus.IN_PROGRESS || task.status === TaskStatus.REVIEW;
      
    case TaskStatus.REVIEW:
      // Can move to review from in progress or done
      return task.status === TaskStatus.IN_PROGRESS || task.status === TaskStatus.DONE;
      
    case TaskStatus.BLOCKED:
      // Can be blocked from any status except done
      return task.status !== TaskStatus.DONE;
      
    case TaskStatus.TODO:
      // Can move back to todo from any status except done
      return task.status !== TaskStatus.DONE;
      
    default:
      return false;
  }
};

// Get tasks that depend on a specific task
export const getDependentTasks = (taskId, allTasks) => {
  return allTasks.filter(task => 
    task.depends_on && task.depends_on.includes(taskId)
  );
};

// Get all dependencies for a task (recursive)
export const getAllDependencies = (taskId, allTasks, visited = new Set()) => {
  if (visited.has(taskId)) {
    return []; // Circular dependency protection
  }
  
  visited.add(taskId);
  const task = allTasks.find(t => t.id === taskId);
  
  if (!task || !task.depends_on) {
    return [];
  }
  
  const directDependencies = task.depends_on;
  const allDependencies = [...directDependencies];
  
  directDependencies.forEach(depId => {
    const subDependencies = getAllDependencies(depId, allTasks, visited);
    subDependencies.forEach(subDep => {
      if (!allDependencies.includes(subDep)) {
        allDependencies.push(subDep);
      }
    });
  });
  
  return allDependencies;
};

// Check for circular dependencies
export const hasCircularDependency = (taskId, newDependencyId, allTasks) => {
  const dependenciesOfNew = getAllDependencies(newDependencyId, allTasks);
  return dependenciesOfNew.includes(taskId);
};

// Calculate task completion percentage
export const calculateTaskProgress = (tasks) => {
  if (tasks.length === 0) return 0;
  
  const completedTasks = tasks.filter(task => task.status === TaskStatus.DONE).length;
  return Math.round((completedTasks / tasks.length) * 100);
};

// Get tasks by status
export const getTasksByStatus = (tasks, status) => {
  return tasks.filter(task => task.status === status);
};

// Get overdue tasks (if you add due dates later)
export const getOverdueTasks = (tasks) => {
  const now = new Date();
  return tasks.filter(task => 
    task.dueDate && 
    new Date(task.dueDate) < now && 
    task.status !== TaskStatus.DONE
  );
};

// Sort tasks by priority/dependencies (topological sort)
export const sortTasksByDependencies = (tasks) => {
  const sorted = [];
  const visiting = new Set();
  const visited = new Set();
  
  const visit = (taskId) => {
    if (visited.has(taskId)) return;
    if (visiting.has(taskId)) return; // Circular dependency
    
    visiting.add(taskId);
    const task = tasks.find(t => t.id === taskId);
    
    if (task && task.depends_on) {
      task.depends_on.forEach(depId => visit(depId));
    }
    
    visiting.delete(taskId);
    visited.add(taskId);
    
    if (task) {
      sorted.push(task);
    }
  };
  
  tasks.forEach(task => visit(task.id));
  
  return sorted;
};

// Filter tasks by criteria
export const filterTasks = (tasks, criteria) => {
  return tasks.filter(task => {
    // Filter by status
    if (criteria.status !== undefined && task.status !== criteria.status) {
      return false;
    }
    
    // Filter by user
    if (criteria.user && (!task.users || !task.users.includes(criteria.user))) {
      return false;
    }
    
    // Filter by tag
    if (criteria.tag && (!task.tags || !task.tags.includes(criteria.tag))) {
      return false;
    }
    
    // Filter by search term
    if (criteria.search) {
      const searchTerm = criteria.search.toLowerCase();
      const matchesName = task.name.toLowerCase().includes(searchTerm);
      const matchesDescription = task.description && task.description.toLowerCase().includes(searchTerm);
      const matchesTags = task.tags && task.tags.some(tag => tag.toLowerCase().includes(searchTerm));
      
      if (!matchesName && !matchesDescription && !matchesTags) {
        return false;
      }
    }
    
    return true;
  });
};

// Get task statistics
export const getTaskStatistics = (tasks) => {
  const total = tasks.length;
  const byStatus = Object.values(TaskStatus).reduce((acc, status) => {
    acc[status] = tasks.filter(task => task.status === status).length;
    return acc;
  }, {});
  
  const completion = calculateTaskProgress(tasks);
  
  const userStats = tasks.reduce((acc, task) => {
    if (task.users) {
      task.users.forEach(user => {
        if (!acc[user]) {
          acc[user] = { total: 0, completed: 0 };
        }
        acc[user].total++;
        if (task.status === TaskStatus.DONE) {
          acc[user].completed++;
        }
      });
    }
    return acc;
  }, {});
  
  const tagStats = tasks.reduce((acc, task) => {
    if (task.tags) {
      task.tags.forEach(tag => {
        acc[tag] = (acc[tag] || 0) + 1;
      });
    }
    return acc;
  }, {});
  
  return {
    total,
    byStatus,
    completion,
    userStats,
    tagStats
  };
};

// Build dependency graph
const buildDependencyGraph = (tasks) => {
  const graph = new Map();
  const inDegree = new Map();
  
  // Initialize graph
  tasks.forEach(task => {
    graph.set(task.id, []);
    inDegree.set(task.id, 0);
  });
  
  // Build edges and calculate in-degrees
  tasks.forEach(task => {
    if (task.depends_on) {
      task.depends_on.forEach(depId => {
        if (graph.has(depId)) {
          graph.get(depId).push(task.id);
          inDegree.set(task.id, inDegree.get(task.id) + 1);
        }
      });
    }
  });
  
  return { graph, inDegree };
};

// Calculate hierarchical levels using topological sort
const calculateHierarchicalLevels = (tasks) => {
  const { graph, inDegree } = buildDependencyGraph(tasks);
  const levels = new Map();
  const queue = [];
  
  // Find all nodes with no incoming edges (level 0)
  tasks.forEach(task => {
    if (inDegree.get(task.id) === 0) {
      levels.set(task.id, 0);
      queue.push(task.id);
    }
  });
  
  // Process queue to assign levels
  while (queue.length > 0) {
    const currentId = queue.shift();
    const currentLevel = levels.get(currentId);
    
    graph.get(currentId).forEach(neighborId => {
      const newDegree = inDegree.get(neighborId) - 1;
      inDegree.set(neighborId, newDegree);
      
      if (newDegree === 0) {
        levels.set(neighborId, currentLevel + 1);
        queue.push(neighborId);
      }
    });
  }
  
  // Handle any remaining nodes (circular dependencies)
  tasks.forEach(task => {
    if (!levels.has(task.id)) {
      levels.set(task.id, 0);
    }
  });
  
  return levels;
};

// Improved hierarchical layout with better spacing
export const hierarchicalLayout = (tasks, width = 1200, height = 800) => {
  if (tasks.length === 0) return [];
  
  const levels = calculateHierarchicalLevels(tasks);
  const maxLevel = Math.max(...Array.from(levels.values()));
  const levelHeight = height / (maxLevel + 2); // +2 for padding
  
  // Group tasks by level
  const tasksByLevel = new Map();
  tasks.forEach(task => {
    const level = levels.get(task.id);
    if (!tasksByLevel.has(level)) {
      tasksByLevel.set(level, []);
    }
    tasksByLevel.get(level).push(task);
  });
  
  const positions = [];
  
  // Position tasks level by level
  for (let level = 0; level <= maxLevel; level++) {
    const tasksInLevel = tasksByLevel.get(level) || [];
    const levelWidth = width * 0.9; // 90% of total width
    const taskSpacing = tasksInLevel.length > 1 ? levelWidth / (tasksInLevel.length - 1) : 0;
    const startX = (width - levelWidth) / 2;
    
    tasksInLevel.forEach((task, index) => {
      const x = tasksInLevel.length === 1 
        ? width / 2 
        : startX + (index * taskSpacing);
      const y = (level + 1) * levelHeight;
      
      positions.push({
        id: task.id,
        position: { x: x - 125, y } // -125 to center the 250px wide node
      });
    });
  }
  
  return positions;
};

// Force-directed layout with improved physics
export const forceDirectedLayout = (tasks, width = 1200, height = 800, iterations = 150) => {
  if (tasks.length === 0) return [];
  
  // Initialize random positions
  const nodes = tasks.map(task => ({
    id: task.id,
    x: Math.random() * width,
    y: Math.random() * height,
    vx: 0,
    vy: 0,
    task
  }));
  
  const nodeMap = new Map(nodes.map(node => [node.id, node]));
  
  // Parameters
  const repulsionStrength = 8000;
  const attractionStrength = 0.1;
  const damping = 0.8;
  const minDistance = 250;
  const idealEdgeLength = 200;
  
  for (let i = 0; i < iterations; i++) {
    // Reset forces
    nodes.forEach(node => {
      node.vx *= damping;
      node.vy *= damping;
    });
    
    // Repulsion between all nodes
    for (let j = 0; j < nodes.length; j++) {
      for (let k = j + 1; k < nodes.length; k++) {
        const nodeA = nodes[j];
        const nodeB = nodes[k];
        
        const dx = nodeB.x - nodeA.x;
        const dy = nodeB.y - nodeA.y;
        const distance = Math.sqrt(dx * dx + dy * dy) || 1;
        
        if (distance < minDistance) {
          const force = repulsionStrength / (distance * distance);
          const fx = (dx / distance) * force;
          const fy = (dy / distance) * force;
          
          nodeA.vx -= fx;
          nodeA.vy -= fy;
          nodeB.vx += fx;
          nodeB.vy += fy;
        }
      }
    }
    
    // Attraction between connected nodes (dependencies)
    tasks.forEach(task => {
      if (task.depends_on) {
        task.depends_on.forEach(depId => {
          const taskNode = nodeMap.get(task.id);
          const depNode = nodeMap.get(depId);
          
          if (taskNode && depNode) {
            const dx = depNode.x - taskNode.x;
            const dy = depNode.y - taskNode.y;
            const distance = Math.sqrt(dx * dx + dy * dy) || 1;
            
            const force = attractionStrength * (distance - idealEdgeLength);
            const fx = (dx / distance) * force;
            const fy = (dy / distance) * force;
            
            taskNode.vx += fx;
            taskNode.vy += fy;
            depNode.vx -= fx;
            depNode.vy -= fy;
          }
        });
      }
    });
    
    // Apply forces and boundary constraints
    nodes.forEach(node => {
      node.x += node.vx;
      node.y += node.vy;
      
      // Keep nodes within bounds with margin
      const margin = 100;
      node.x = Math.max(margin, Math.min(width - margin, node.x));
      node.y = Math.max(margin, Math.min(height - margin, node.y));
    });
  }
  
  return nodes.map(node => ({
    id: node.id,
    position: { x: node.x - 125, y: node.y } // -125 to center the 250px wide node
  }));
};

// Smart layout that chooses the best algorithm based on task structure
export const smartLayout = (tasks, width = 1200, height = 800) => {
  if (tasks.length === 0) return [];
  
  // Calculate dependency metrics
  const totalDependencies = tasks.reduce((sum, task) => 
    sum + (task.depends_on ? task.depends_on.length : 0), 0
  );
  const avgDependencies = totalDependencies / tasks.length;
  const maxDependencyChain = calculateMaxDependencyChain(tasks);
  
  // Use hierarchical layout for well-structured dependency chains
  if (maxDependencyChain > 2 && avgDependencies > 0.3) {
    return hierarchicalLayout(tasks, width, height);
  }
  
  // Use force-directed layout for less structured or sparse graphs
  return forceDirectedLayout(tasks, width, height);
};

// Calculate the maximum dependency chain length
const calculateMaxDependencyChain = (tasks) => {
  const visited = new Set();
  let maxChain = 0;
  
  const dfs = (taskId, depth = 0) => {
    if (visited.has(taskId)) return depth;
    
    visited.add(taskId);
    const task = tasks.find(t => t.id === taskId);
    
    if (!task || !task.depends_on || task.depends_on.length === 0) {
      return depth;
    }
    
    let maxDepth = depth;
    task.depends_on.forEach(depId => {
      const childDepth = dfs(depId, depth + 1);
      maxDepth = Math.max(maxDepth, childDepth);
    });
    
    return maxDepth;
  };
  
  tasks.forEach(task => {
    visited.clear();
    const chainLength = dfs(task.id);
    maxChain = Math.max(maxChain, chainLength);
  });
  
  return maxChain;
};

// Convert tasks to ReactFlow nodes and edges with improved layout
export const tasksToReactFlowElements = (tasks, onEdit, onClick, layoutType = 'smart') => {
  let positions;
  
  switch (layoutType) {
    case 'hierarchical':
      positions = hierarchicalLayout(tasks);
      break;
    case 'force':
      positions = forceDirectedLayout(tasks);
      break;
    case 'smart':
    default:
      positions = smartLayout(tasks);
      break;
  }
  
  const positionMap = new Map(positions.map(p => [p.id, p.position]));
  
  const nodes = tasks.map((task) => ({
    id: task.id,
    type: 'taskNode',
    position: positionMap.get(task.id) || { x: 0, y: 0 },
    data: {
      ...task,
      onEdit,
      onClick,
    },
  }));

  const edges = [];
  tasks.forEach(task => {
    if (task.depends_on && task.depends_on.length > 0) {
      task.depends_on.forEach(depId => {
        if (tasks.find(t => t.id === depId)) {
          edges.push({
            id: `${depId}-${task.id}`,
            source: depId,
            target: task.id,
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
          });
        }
      });
    }
  });

  return { nodes, edges };
};