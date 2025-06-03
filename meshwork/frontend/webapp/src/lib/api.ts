// API types based on your backend models
export interface Task {
	id: string;
	name: string;
	description: string;
	depends_on: string[];
	users: string[];
	tags: string[];
	status: 'TODO' | 'IN_PROGRESS' | 'DONE' | 'REVIEW' | 'BLOCKED';
}

export interface TaskGraph {
	id: string;
	name: string;
}

const API_BASE_URL = 'http://localhost:8000'; // Adjust this to your FastAPI server port

export class MeshworkAPI {
	private baseUrl: string;

	constructor(baseUrl: string = API_BASE_URL) {
		this.baseUrl = baseUrl;
	}

	// Graph operations
	async createGraph(): Promise<{ ID: string }> {
		const response = await fetch(`${this.baseUrl}/v0/create_graph`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' }
		});
		return response.json();
	}

	// Task operations
	async getAllTasks(graphId: string): Promise<Task[]> {
		const response = await fetch(`${this.baseUrl}/v0/${graphId}/all_tasks`);
		return response.json();
	}

	async getTask(graphId: string, taskId: string): Promise<Task> {
		const response = await fetch(`${this.baseUrl}/v0/${graphId}/task/${taskId}`);
		return response.json();
	}

	async addTask(graphId: string, task: Partial<Task>): Promise<{ message: string }> {
		const response = await fetch(`${this.baseUrl}/v0/${graphId}/task/add/`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify(task)
		});
		return response.json();
	}

	async updateTask(graphId: string, taskId: string, task: Partial<Task>): Promise<{ message: string }> {
		const response = await fetch(`${this.baseUrl}/v0/${graphId}/task/${taskId}`, {
			method: 'PUT',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify(task)
		});
		return response.json();
	}

	async deleteTask(graphId: string, taskId: string): Promise<{ message: string }> {
		const response = await fetch(`${this.baseUrl}/v0/${graphId}/task/${taskId}`, {
			method: 'DELETE'
		});
		return response.json();
	}
}

export const api = new MeshworkAPI();