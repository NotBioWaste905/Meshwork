// Mirrors meshwork/backend_rs/src/schemas.rs
export enum Status {
	TODO = 'TODO',
	IN_PROGRESS = 'IN_PROGRESS',
	DONE = 'DONE',
	REVIEW = 'REVIEW',
	BLOCKED = 'BLOCKED'
}

export enum Priority {
	LOW = 'LOW',
	NORMAL = 'NORMAL',
	HIGH = 'HIGH'
}

// A type alias rather than an interface so it fits xyflow's Record<string, unknown> node data.
export type Task = {
	id: string;
	name: string;
	description: string;
	/** IDs of the tasks blocking this one. */
	depends_on: string[];
	/** IDs of the assigned actors. On input an actor may also be given by name. */
	users: string[];
	/** ID of the actor who asked for the task and checks the result; null when nobody. */
	reporter: string | null;
	tags: string[];
	status: Status;
	priority: Priority;
	/** Link to the task's pull request, an http(s) URL; empty when there is none. */
	pr_url: string;
	graph_id: string;
};

/** Body for creating a task: the server assigns the ID, the graph comes from the URL. */
export type NewTask = Pick<Task, 'name'> &
	Partial<
		Pick<
			Task,
			| 'description'
			| 'depends_on'
			| 'users'
			| 'tags'
			| 'status'
			| 'priority'
			| 'pr_url'
			| 'reporter'
		>
	>;

/** Body for editing a task: omitted fields stay unchanged, lists are replaced. */
export type TaskUpdate = Partial<Omit<Task, 'id' | 'graph_id' | 'reporter'>> & {
	/** Actor ID or name; '' removes the reporter. */
	reporter?: string;
};

/** Graph metadata with task counters. `name` is the goal; empty for graphs created without one. */
export type Graph = {
	id: string;
	name: string;
	task_count: number;
	done_count: number;
	blocked_count: number;
};

export type ActorKind = 'human' | 'agent';

/** Someone who works on tasks, a person or an AI agent. Global, shared by all graphs. */
export type Actor = {
	/** A UUID; actors migrated from free-text assignees keep that text as their ID. */
	id: string;
	/** Unique among actors. */
	name: string;
	kind: ActorKind;
	/** Number of tasks assigned to the actor, across all graphs. */
	task_count: number;
};

/** A note an actor left on a task. */
export type Comment = {
	id: string;
	/** Markdown. */
	body: string;
	/** Actor ID of the author; null once that actor was deleted. */
	author: string | null;
	/** Milliseconds since the epoch. */
	created_at: number;
};

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
	/** Actor token sent with every request (see session.svelte.ts); '' sends none. */
	private token = '';
	/** Requests wait for this: the check of a stored token, which may drop a revoked one. */
	private ready: Promise<unknown> = Promise.resolve();

	constructor(baseUrl: string = API_BASE_URL) {
		this.baseUrl = baseUrl;
	}

	setToken(token: string) {
		this.token = token;
	}

	/** Holds requests until `check` settles. */
	waitFor(check: Promise<unknown>) {
		this.ready = check.catch(() => {});
	}

	/** `token` given: sent as is, without waiting (used to check tokens). */
	private async request<T>(
		method: string,
		path: string,
		body?: unknown,
		token?: string
	): Promise<T> {
		if (token === undefined) {
			await this.ready;
			token = this.token;
		}
		const headers: Record<string, string> = {};
		if (body !== undefined) headers['Content-Type'] = 'application/json';
		if (token) headers['Authorization'] = `Bearer ${token}`;
		const response = await fetch(`${this.baseUrl}${path}`, {
			method,
			headers,
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
	createGraph(name?: string): Promise<{ ID: string; name: string }> {
		return this.request('POST', '/v0/create_graph', name === undefined ? undefined : { name });
	}

	/** All graphs, newest first. */
	listGraphs(): Promise<Graph[]> {
		return this.request('GET', '/v0/graphs');
	}

	getGraph(graphId: string): Promise<Graph> {
		return this.request('GET', `/v0/${graphId}`);
	}

	renameGraph(graphId: string, name: string): Promise<Graph> {
		return this.request('PUT', `/v0/${graphId}`, { name });
	}

	/** The actor of a token: the given one, or the one in use. 401 without a valid token. */
	me(token?: string): Promise<Actor> {
		return this.request('GET', '/v0/me', undefined, token ?? this.token);
	}

	// Actor operations
	/** All actors, by name. */
	listActors(): Promise<Actor[]> {
		return this.request('GET', '/v0/actors');
	}

	getActor(actorId: string): Promise<Actor> {
		return this.request('GET', `/v0/actors/${actorId}`);
	}

	/** 409 when the name is taken. */
	addActor(actor: { name: string; kind?: ActorKind }): Promise<Actor> {
		return this.request('POST', '/v0/actors', actor);
	}

	updateActor(actorId: string, patch: Partial<Pick<Actor, 'name' | 'kind'>>): Promise<Actor> {
		return this.request('PUT', `/v0/actors/${actorId}`, patch);
	}

	/** 409 while the actor is assigned to tasks, unless `unassign` drops those assignments. */
	deleteActor(actorId: string, unassign = false): Promise<{ message: string }> {
		return this.request('DELETE', `/v0/actors/${actorId}?unassign=${unassign}`);
	}

	/** Moves the actor's tasks to `into` (an ID or name) and deletes it; returns the one kept. */
	mergeActor(actorId: string, into: string): Promise<Actor> {
		return this.request('POST', `/v0/actors/${actorId}/merge`, { into });
	}

	// Task operations
	getAllTasks(graphId: string): Promise<Task[]> {
		return this.request('GET', `/v0/${graphId}/all_tasks`);
	}

	/** The frontier: TODO tasks whose dependencies are all DONE. */
	getReadyTasks(graphId: string, unassignedOnly = false): Promise<Task[]> {
		return this.request('GET', `/v0/${graphId}/ready_tasks?unassigned_only=${unassignedOnly}`);
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

	/** Oldest first. */
	listComments(graphId: string, taskId: string): Promise<Comment[]> {
		return this.request('GET', `/v0/${graphId}/task/${taskId}/comments`);
	}

	/** Signed as the token's actor; 401 when signed out. */
	addComment(graphId: string, taskId: string, body: string): Promise<Comment> {
		return this.request('POST', `/v0/${graphId}/task/${taskId}/comments`, { body });
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
