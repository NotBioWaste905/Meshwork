const API_BASE_URL = process.env.REACT_APP_API_URL || "http://localhost:8000";

class TaskApiService {
  async request(endpoint, options = {}) {
    const url = `${API_BASE_URL}${endpoint}`;
    const config = {
      headers: {
        "Content-Type": "application/json",
        ...options.headers,
      },
      ...options,
    };

    if (config.body && typeof config.body === "object") {
      config.body = JSON.stringify(config.body);
    }

    try {
      const response = await fetch(url, config);

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      const contentType = response.headers.get("content-type");
      if (contentType && contentType.includes("application/json")) {
        return await response.json();
      }

      return await response.text();
    } catch (error) {
      console.error(`API request failed: ${endpoint}`, error);
      throw error;
    }
  }

  // Get all tasks
  async getTasks() {
    return this.request("/v0/all_tasks");
  }

  // Get a specific task by ID
  async getTask(taskId) {
    return this.request(`/v0/task/${taskId}`);
  }

  // Create a new task
  async createTask(taskData) {
    return this.request("/v0/task/add/", {
      method: "POST",
      body: taskData,
    });
  }

  // Update an existing task
  async updateTask(taskId, taskData) {
    return this.request(`/v0/task/${taskId}`, {
      method: "PUT",
      body: taskData,
    });
  }

  // Delete a task
  async deleteTask(taskId) {
    return this.request(`/v0/task/${taskId}`, {
      method: "DELETE",
    });
  }

  // Health check
  async healthCheck() {
    return this.request("/health");
  }
}

// Create and export a singleton instance
const taskApi = new TaskApiService();

export default taskApi;

// Export individual methods for convenience
export const {
  getTasks,
  getTask,
  createTask,
  updateTask,
  deleteTask,
  healthCheck,
} = taskApi;
