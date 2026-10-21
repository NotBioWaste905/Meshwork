// Mirrors meshwork/backend_rs/src/schemas.rs
export enum Status {
	TODO = 'TODO',
	IN_PROGRESS = 'IN_PROGRESS',
	DONE = 'DONE',
	REVIEW = 'REVIEW',
	BLOCKED = 'BLOCKED'
}

// A type alias rather than an interface so it fits xyflow's Record<string, unknown> node data.
export type Task = {
	id: string;
	name: string;
	description: string;
	/** IDs of the tasks blocking this one. */
	depends_on: string[];
	users: string[];
	tags: string[];
	status: Status;
	graph_id: string;
};

/** Body for creating a task: the server assigns the ID, the graph comes from the URL. */
export type NewTask = Pick<Task, 'name'> &
	Partial<Pick<Task, 'description' | 'depends_on' | 'users' | 'tags' | 'status'>>;

/** Body for editing a task: omitted fields stay unchanged, lists are replaced. */
export type TaskUpdate = Partial<Omit<Task, 'id' | 'graph_id'>>;

export const API_BASE_URL: string = import.meta.env.VITE_API_URL ?? 'http://localhost:8000';

export class ApiError extends Error {
	constructor(
		public status: number,
		message: string
	) {
		super(message);
	}
}

export class MeshworkAPI {
	private baseUrl: string;

	constructor(baseUrl: string = API_BASE_URL) {
		this.baseUrl = baseUrl;
	}

	private async request<T>(method: string, path: string, body?: unknown): Promise<T> {
		const response = await fetch(`${this.baseUrl}${path}`, {
			method,
			headers: body === undefined ? undefined : { 'Content-Type': 'application/json' },
			body: body === undefined ? undefined : JSON.stringify(body)
		});
		const text = await response.text();
		const data = text ? JSON.parse(text) : null;
		if (!response.ok) {
			throw new ApiError(response.status, data?.message ?? text ?? response.statusText);
		}
		return data as T;
	}

	// Graph operations
	createGraph(): Promise<{ ID: string }> {
		return this.request('POST', '/v0/create_graph');
	}

	// Task operations
	getAllTasks(graphId: string): Promise<Task[]> {
		return this.request('GET', `/v0/${graphId}/all_tasks`);
	}

	getTask(graphId: string, taskId: string): Promise<Task> {
		return this.request('GET', `/v0/${graphId}/task/${taskId}`);
	}

	addTask(graphId: string, task: NewTask): Promise<Task> {
		return this.request('POST', `/v0/${graphId}/task/add/`, task);
	}

	updateTask(graphId: string, taskId: string, task: TaskUpdate): Promise<Task> {
		return this.request('PUT', `/v0/${graphId}/task/${taskId}`, task);
	}

	deleteTask(graphId: string, taskId: string): Promise<{ message: string }> {
		return this.request('DELETE', `/v0/${graphId}/task/${taskId}`);
	}

	/** `dependency` blocks `dependee`; on the canvas the edge goes dependency → dependee. */
	connectNodes(
		graphId: string,
		dependency: string,
		dependee: string
	): Promise<{ message: string }> {
		return this.request('POST', `/v0/${graphId}/connect_nodes/`, {
			node_dependency: dependency,
			node_dependee: dependee
		});
	}

	disconnectNodes(
		graphId: string,
		dependency: string,
		dependee: string
	): Promise<{ message: string }> {
		return this.request('POST', `/v0/${graphId}/disconnect_nodes/`, {
			node_dependency: dependency,
			node_dependee: dependee
		});
	}
}

export const api = new MeshworkAPI();
