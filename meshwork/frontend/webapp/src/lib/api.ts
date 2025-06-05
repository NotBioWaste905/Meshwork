// API types based on your backend models
export interface Task {
	id: string;
	name: string;
	description: string;
	depends_on: string[];
	users: string[];
	tags: string[];
	status: Status;
	graphId?: string;
}

export enum Status {
	'TODO' = 0,
	'IN_PROGRESS' = 1,
	'DONE' = 2,
	'REVIEW' = 3,
	'BLOCKED' = 4
}

export interface TaskGraph {
	id: string;
	name: string;
}

const API_BASE_URL = 'http://localhost:8000'; // Adjust this to your FastAPI server port
const EXAMPLE_GRAPH_ID = 'abc123';

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
	async getAllTasks(graphId: string = EXAMPLE_GRAPH_ID): Promise<Task[]> {
		const response = await fetch(`${this.baseUrl}/v0/${graphId}/all_tasks`);
		return response.json();
	}

	async getTask(graphId: string = EXAMPLE_GRAPH_ID, taskId: string): Promise<Task> {
		const response = await fetch(`${this.baseUrl}/v0/${graphId}/task/${taskId}`);
		return response.json();
	}

	async addTask(
		graphId: string = EXAMPLE_GRAPH_ID,
		task: Partial<Task>
	): Promise<{ message: string }> {
		const response = await fetch(`${this.baseUrl}/v0/${graphId}/task/add/`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify(task)
		});
		return response.json();
	}

	async updateTask(
		graphId: string = EXAMPLE_GRAPH_ID,
		taskId: string,
		task: Partial<Task>
	): Promise<{ message: string }> {
		const response = await fetch(`${this.baseUrl}/v0/${graphId}/task/${taskId}`, {
			method: 'PUT',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify(task)
		});
		return response.json();
	}

	async connectNodes(
		graphId: string = EXAMPLE_GRAPH_ID,
		source: string,
		target: string
	): Promise<{ message: string }> {
		const response = await fetch(`${this.baseUrl}/v0/${graphId}/connect_nodes/`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({
				node_dependency: source,
				node_dependee: target
			})
		});
		return response.json();
	}

	async deleteTask(
		graphId: string = EXAMPLE_GRAPH_ID,
		taskId: string
	): Promise<{ message: string }> {
		const response = await fetch(`${this.baseUrl}/v0/${graphId}/task/${taskId}`, {
			method: 'DELETE'
		});
		return response.json();
	}

	async disconnectNodes(
		graphId: string = EXAMPLE_GRAPH_ID,
		source: string,
		target: string
	): Promise<{ message: string }> {
		const response = await fetch(`${this.baseUrl}/v0/${graphId}/disconnect_nodes/`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({
				node_dependency: source,
				node_dependee: target
			})
		});
		return response.json();
	}
}

export const api = new MeshworkAPI();
