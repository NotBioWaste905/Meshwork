import React, { useState, useEffect } from "react";
import styled from "styled-components";
import TaskFlow from "./components/TaskFlow";
import taskApi from "./services/taskApi";

const AppContainer = styled.div`
  width: 100vw;
  height: 100vh;
  font-family:
    -apple-system, BlinkMacSystemFont, "Segoe UI", "Roboto", sans-serif;
`;

const ErrorMessage = styled.div`
  position: fixed;
  top: 20px;
  right: 20px;
  background: #dc3545;
  color: white;
  padding: 12px 16px;
  border-radius: 6px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  z-index: 1001;
  max-width: 300px;
  font-size: 14px;
  cursor: pointer;
  animation: slideInRight 0.3s ease-out;

  @keyframes slideInRight {
    from {
      transform: translateX(100%);
      opacity: 0;
    }
    to {
      transform: translateX(0);
      opacity: 1;
    }
  }
`;

const SuccessMessage = styled.div`
  position: fixed;
  top: 20px;
  right: 20px;
  background: #28a745;
  color: white;
  padding: 12px 16px;
  border-radius: 6px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  z-index: 1001;
  max-width: 300px;
  font-size: 14px;
  cursor: pointer;
  animation: slideInRight 0.3s ease-out;

  @keyframes slideInRight {
    from {
      transform: translateX(100%);
      opacity: 0;
    }
    to {
      transform: translateX(0);
      opacity: 1;
    }
  }
`;

const InitialLoadingOverlay = styled.div`
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background: #f8f9fa;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  font-size: 16px;
  color: #666;
`;

const LoadingSpinner = styled.div`
  width: 40px;
  height: 40px;
  border: 3px solid #e3e3e3;
  border-top: 3px solid #007bff;
  border-radius: 50%;
  animation: spin 1s linear infinite;
  margin-bottom: 16px;

  @keyframes spin {
    0% {
      transform: rotate(0deg);
    }
    100% {
      transform: rotate(360deg);
    }
  }
`;

// Sample data for fallback when backend is not available
const sampleTasks = [
  {
    id: "task-1",
    name: "Project Planning",
    description:
      "Create initial project plan and define requirements for the task management system",
    status: 2, // DONE
    users: ["john.doe", "jane.smith"],
    tags: ["planning", "requirements"],
    depends_on: [],
  },
  {
    id: "task-2",
    name: "Backend API Development",
    description: "Develop REST API endpoints for task management using FastAPI",
    status: 1, // IN_PROGRESS
    users: ["mike.wilson"],
    tags: ["backend", "api", "python"],
    depends_on: ["task-1"],
  },
  {
    id: "task-3",
    name: "Frontend UI Components",
    description:
      "Create React components for task visualization using ReactFlow",
    status: 1, // IN_PROGRESS
    users: ["sarah.connor"],
    tags: ["frontend", "react", "ui"],
    depends_on: ["task-1"],
  },
  {
    id: "task-4",
    name: "Database Schema",
    description:
      "Design and implement database schema for tasks and dependencies",
    status: 2, // DONE
    users: ["david.brown"],
    tags: ["database", "schema"],
    depends_on: ["task-1"],
  },
  {
    id: "task-5",
    name: "Authentication System",
    description: "Implement user authentication and authorization",
    status: 4, // TODO
    users: ["mike.wilson"],
    tags: ["auth", "security"],
    depends_on: ["task-2", "task-4"],
  },
  {
    id: "task-6",
    name: "Task Flow Integration",
    description: "Integrate ReactFlow with backend API for real-time updates",
    status: 0, // TODO
    users: ["sarah.connor"],
    tags: ["integration", "frontend"],
    depends_on: ["task-2", "task-3"],
  },
  {
    id: "task-7",
    name: "Testing & QA",
    description: "Write unit tests and perform quality assurance testing",
    status: 0, // TODO
    users: ["jane.smith"],
    tags: ["testing", "qa"],
    depends_on: ["task-5", "task-6"],
  },
  {
    id: "task-8",
    name: "Documentation",
    description: "Create user documentation and API documentation",
    status: 3, // REVIEW
    users: ["john.doe"],
    tags: ["docs", "documentation"],
    depends_on: ["task-6"],
  },
];

function App() {
  const [tasks, setTasks] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [success, setSuccess] = useState(null);
  const [backendAvailable, setBackendAvailable] = useState(false);
  const [initialLoadComplete, setInitialLoadComplete] = useState(false);

  useEffect(() => {
    loadTasks();
  }, []);

  useEffect(() => {
    if (error) {
      const timer = setTimeout(() => setError(null), 5000);
      return () => clearTimeout(timer);
    }
  }, [error]);

  useEffect(() => {
    if (success) {
      const timer = setTimeout(() => setSuccess(null), 3000);
      return () => clearTimeout(timer);
    }
  }, [success]);

  const loadTasks = async () => {
    try {
      setLoading(true);

      // First check if backend is available
      try {
        await taskApi.healthCheck();
        setBackendAvailable(true);

        // insert sampleTasks tasks into database
        for (const task of sampleTasks) {
          await taskApi.createTask(task);
        }

        // Backend is available, try to fetch tasks
        const fetchedTasks = await taskApi.getTasks();

        if (Array.isArray(fetchedTasks) && fetchedTasks.length > 0) {
          setTasks(fetchedTasks);
          console.log("Loaded tasks from backend:", fetchedTasks.length);
        } else {
          // Backend is available but no tasks, use sample data
          setTasks(sampleTasks);
          console.log(
            "Backend available but no tasks found, using sample data",
          );
        }
      } catch (backendError) {
        console.warn(
          "Backend not available, using sample data:",
          backendError.message,
        );
        setBackendAvailable(false);
        setTasks(sampleTasks);
      }
    } catch (err) {
      console.error("Failed to load tasks:", err);
      setError(`Failed to load tasks: ${err.message}`);
      setTasks(sampleTasks); // Fallback to sample data
    } finally {
      setLoading(false);
      setInitialLoadComplete(true);
    }
  };

  const handleTaskCreate = async (taskData) => {
    try {
      setLoading(true);

      if (backendAvailable) {
        const newTask = await taskApi.createTask(taskData);
        setTasks((prev) => [...prev, newTask]);
        setSuccess("Task created successfully!");
        console.log("Task created via API:", newTask.id);
      } else {
        // Backend not available, update locally
        setTasks((prev) => [...prev, taskData]);
        setSuccess("Task created locally (backend offline)");
        console.log("Task created locally:", taskData.id);
      }
    } catch (err) {
      console.warn("Failed to create task via API, updating locally:", err);
      setTasks((prev) => [...prev, taskData]);
      setError("Failed to save to backend, changes saved locally");
    } finally {
      setLoading(false);
    }
  };

  const handleTaskUpdate = async (taskData) => {
    try {
      setLoading(true);

      if (backendAvailable) {
        const updatedTask = await taskApi.updateTask(taskData.id, taskData);
        setTasks((prev) =>
          prev.map((task) => (task.id === taskData.id ? updatedTask : task)),
        );
        setSuccess("Task updated successfully!");
        console.log("Task updated via API:", updatedTask.id);
      } else {
        // Backend not available, update locally
        setTasks((prev) =>
          prev.map((task) => (task.id === taskData.id ? taskData : task)),
        );
        setSuccess("Task updated locally (backend offline)");
        console.log("Task updated locally:", taskData.id);
      }
    } catch (err) {
      console.warn("Failed to update task via API, updating locally:", err);
      setTasks((prev) =>
        prev.map((task) => (task.id === taskData.id ? taskData : task)),
      );
      setError("Failed to save to backend, changes saved locally");
    } finally {
      setLoading(false);
    }
  };

  const handleTaskDelete = async (taskId) => {
    try {
      setLoading(true);

      if (backendAvailable) {
        await taskApi.deleteTask(taskId);
        setTasks((prev) => prev.filter((task) => task.id !== taskId));
        setSuccess("Task deleted successfully!");
        console.log("Task deleted via API:", taskId);
      } else {
        // Backend not available, update locally
        setTasks((prev) => prev.filter((task) => task.id !== taskId));
        setSuccess("Task deleted locally (backend offline)");
        console.log("Task deleted locally:", taskId);
      }
    } catch (err) {
      console.warn("Failed to delete task via API, updating locally:", err);
      setTasks((prev) => prev.filter((task) => task.id !== taskId));
      setError("Failed to delete from backend, task removed locally");
    } finally {
      setLoading(false);
    }
  };

  const handleRetryConnection = async () => {
    await loadTasks();
  };

  if (!initialLoadComplete) {
    return (
      <AppContainer>
        <InitialLoadingOverlay>
          <LoadingSpinner />
          <div>Connecting to backend...</div>
          <div style={{ fontSize: "12px", marginTop: "8px", opacity: 0.7 }}>
            {backendAvailable ? "Backend connected" : "Using offline mode"}
          </div>
        </InitialLoadingOverlay>
      </AppContainer>
    );
  }

  return (
    <AppContainer>
      <TaskFlow
        initialTasks={tasks}
        onTaskCreate={handleTaskCreate}
        onTaskUpdate={handleTaskUpdate}
        onTaskDelete={handleTaskDelete}
        loading={loading}
        backendAvailable={backendAvailable}
        onRetryConnection={handleRetryConnection}
      />

      {error && (
        <ErrorMessage onClick={() => setError(null)}>
          {error}
          <div style={{ fontSize: "11px", marginTop: "4px", opacity: 0.8 }}>
            Click to dismiss
          </div>
        </ErrorMessage>
      )}

      {success && (
        <SuccessMessage onClick={() => setSuccess(null)}>
          {success}
          <div style={{ fontSize: "11px", marginTop: "4px", opacity: 0.8 }}>
            Click to dismiss
          </div>
        </SuccessMessage>
      )}
    </AppContainer>
  );
}

export default App;
