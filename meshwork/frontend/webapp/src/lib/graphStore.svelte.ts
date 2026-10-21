import { api, ApiError, Status, type NewTask, type Task, type TaskUpdate } from './api';
import { CURRENT_USER } from './mock';
import { shortId } from './status';

export type TaskFilter = 'all' | 'mine' | 'blocked';

const KNOWN_GRAPHS_KEY = 'meshwork:knownGraphs';

/** Graph IDs this browser has opened. There is no "list graphs" endpoint yet. */
export function knownGraphIds(): string[] {
	try {
		const parsed = JSON.parse(localStorage.getItem(KNOWN_GRAPHS_KEY) ?? '[]');
		return Array.isArray(parsed) ? parsed.filter((id) => typeof id === 'string') : [];
	} catch {
		return [];
	}
}

export function rememberKnownGraph(graphId: string) {
	try {
		const ids = knownGraphIds().filter((id) => id !== graphId);
		localStorage.setItem(KNOWN_GRAPHS_KEY, JSON.stringify([graphId, ...ids].slice(0, 20)));
	} catch {
		// storage unavailable, nothing to remember
	}
}

export function forgetKnownGraph(graphId: string) {
	try {
		const ids = knownGraphIds().filter((id) => id !== graphId);
		localStorage.setItem(KNOWN_GRAPHS_KEY, JSON.stringify(ids));
	} catch {
		// storage unavailable
	}
}

/** The tasks of the open graph, shared by the Graph, Board, Table and Task screens. */
class GraphStore {
	graphId = $state('');
	tasks = $state.raw<Task[]>([]);
	loaded = $state(false);
	notFound = $state(false);
	error = $state('');
	filter = $state<TaskFilter>('all');
	/** The new-task dialog, rendered by the graph layout. */
	newTaskOpen = $state(false);

	byId = $derived(new Map(this.tasks.map((task) => [task.id, task])));

	/** task id -> tasks that depend on it (the tasks it blocks). */
	dependents = $derived.by(() => {
		const map = new Map<string, Task[]>();
		for (const task of this.tasks) {
			for (const dep of task.depends_on) {
				map.set(dep, [...(map.get(dep) ?? []), task]);
			}
		}
		return map;
	});

	/** Tasks nothing depends on but that depend on something: the goals of the graph. */
	terminals = $derived(
		this.tasks.filter((t) => t.depends_on.length > 0 && !this.dependents.has(t.id))
	);

	/** The graph has no name in the API, so it is called after its goal. */
	name = $derived(
		this.terminals[0]?.name ??
			(this.tasks.length === 1 ? this.tasks[0].name : `Graph ${this.graphId.slice(0, 8)}`)
	);

	doneCount = $derived(this.tasks.filter((t) => t.status === Status.DONE).length);
	blockedCount = $derived(this.tasks.filter((t) => t.status === Status.BLOCKED).length);
	mineCount = $derived(this.tasks.filter((t) => isMine(t)).length);

	visible = $derived(this.tasks.filter((t) => this.matches(t)));

	matches(task: Task): boolean {
		if (this.filter === 'mine') return isMine(task);
		if (this.filter === 'blocked') return task.status === Status.BLOCKED;
		return true;
	}

	/** Tasks blocking `task` that are not DONE yet. */
	openBlockers(task: Task): Task[] {
		return task.depends_on
			.map((id) => this.byId.get(id))
			.filter((t): t is Task => !!t && t.status !== Status.DONE);
	}

	/** One-line relation summary for board cards: what it waits on or blocks. */
	note(task: Task): { text: string; warn: boolean } {
		return describeRelations(task, this.tasks);
	}

	async load(graphId: string) {
		if (graphId !== this.graphId) {
			this.graphId = graphId;
			this.tasks = [];
			this.loaded = false;
			this.notFound = false;
			this.filter = 'all';
		}
		try {
			this.tasks = await api.getAllTasks(graphId);
			this.error = '';
			rememberKnownGraph(graphId);
		} catch (error) {
			if (error instanceof ApiError && error.status === 404) {
				this.notFound = true;
				forgetKnownGraph(graphId);
			} else {
				this.fail('load tasks', error);
			}
		}
		this.loaded = true;
	}

	fail(action: string, error: unknown) {
		console.error(`Failed to ${action}:`, error);
		this.error = `Failed to ${action}: ${error instanceof Error ? error.message : error}`;
	}

	private replace(task: Task) {
		this.tasks = this.tasks.map((t) => (t.id === task.id ? task : t));
	}

	async addTask(task: NewTask): Promise<Task | null> {
		try {
			const created = await api.addTask(this.graphId, task);
			this.tasks = [...this.tasks, created];
			this.error = '';
			return created;
		} catch (error) {
			this.fail('add task', error);
			return null;
		}
	}

	async updateTask(id: string, patch: TaskUpdate): Promise<Task | null> {
		try {
			const saved = await api.updateTask(this.graphId, id, patch);
			this.replace(saved);
			this.error = '';
			return saved;
		} catch (error) {
			this.fail('save task', error);
			return null;
		}
	}

	async deleteTask(id: string): Promise<boolean> {
		try {
			await api.deleteTask(this.graphId, id);
			// the backend drops the edges of a deleted task; mirror that locally
			this.tasks = this.tasks
				.filter((t) => t.id !== id)
				.map((t) =>
					t.depends_on.includes(id) ? { ...t, depends_on: t.depends_on.filter((d) => d !== id) } : t
				);
			this.error = '';
			return true;
		} catch (error) {
			this.fail('delete task', error);
			return false;
		}
	}

	/** `dependency` blocks `dependee`. */
	async connect(dependency: string, dependee: string): Promise<boolean> {
		try {
			await api.connectNodes(this.graphId, dependency, dependee);
			const target = this.byId.get(dependee);
			if (target && !target.depends_on.includes(dependency)) {
				this.replace({ ...target, depends_on: [...target.depends_on, dependency] });
			}
			this.error = '';
			return true;
		} catch (error) {
			this.fail('connect tasks', error);
			await this.load(this.graphId);
			return false;
		}
	}

	async disconnect(dependency: string, dependee: string): Promise<boolean> {
		try {
			await api.disconnectNodes(this.graphId, dependency, dependee);
			const target = this.byId.get(dependee);
			if (target) {
				this.replace({ ...target, depends_on: target.depends_on.filter((d) => d !== dependency) });
			}
			this.error = '';
			return true;
		} catch (error) {
			this.fail('disconnect tasks', error);
			await this.load(this.graphId);
			return false;
		}
	}
}

export function isMine(task: Task): boolean {
	return task.users.includes(CURRENT_USER.name);
}

export const graphStore = new GraphStore();

/** Longest chain of dependencies, as task IDs from the first blocker to the last task. */
export function criticalPath(tasks: Task[]): string[] {
	const byId = new Map(tasks.map((t) => [t.id, t]));
	const memo = new Map<string, string[]>();
	const visiting = new Set<string>();

	const longest = (id: string): string[] => {
		const cached = memo.get(id);
		if (cached) return cached;
		if (visiting.has(id)) return [id]; // cycle guard, the backend does not forbid cycles yet
		visiting.add(id);
		let best: string[] = [];
		for (const dep of byId.get(id)?.depends_on ?? []) {
			if (!byId.has(dep)) continue;
			const chain = longest(dep);
			if (chain.length > best.length) best = chain;
		}
		visiting.delete(id);
		const result = [...best, id];
		memo.set(id, result);
		return result;
	};

	let best: string[] = [];
	for (const task of tasks) {
		const chain = longest(task.id);
		if (chain.length > best.length) best = chain;
	}
	return best.length > 1 ? best : [];
}

export type GraphSummary = {
	id: string;
	name: string;
	goal: string;
	done: number;
	total: number;
	blocked: number;
};

/** Name and progress of a graph from its tasks. The API has no graph metadata. */
export function summarize(graphId: string, tasks: Task[]): GraphSummary {
	const dependedOn = new Set(tasks.flatMap((t) => t.depends_on));
	const goals = tasks.filter((t) => !dependedOn.has(t.id) && t.depends_on.length > 0);
	const goal = goals[0]?.name ?? tasks[0]?.name ?? 'No tasks yet';
	return {
		id: graphId,
		name: goal,
		goal,
		done: tasks.filter((t) => t.status === Status.DONE).length,
		total: tasks.length,
		blocked: tasks.filter((t) => t.status === Status.BLOCKED).length
	};
}

/** Open tasks of the user that hold up other open tasks. */
export function blockingOthers(tasks: Task[]): { task: Task; blocks: number }[] {
	const byId = new Map(tasks.map((t) => [t.id, t]));
	const result: { task: Task; blocks: number }[] = [];
	for (const task of tasks) {
		if (task.status === Status.DONE || !isMine(task)) continue;
		const blocks = tasks.filter(
			(t) => t.status !== Status.DONE && t.depends_on.includes(task.id) && byId.has(t.id)
		).length;
		if (blocks > 0) result.push({ task, blocks });
	}
	return result;
}

/** What a task waits on or blocks, as one short line. `tasks` is the whole graph of the task. */
export function describeRelations(task: Task, tasks: Task[]): { text: string; warn: boolean } {
	const byId = new Map(tasks.map((t) => [t.id, t]));
	const open = task.depends_on
		.map((id) => byId.get(id))
		.filter((t): t is Task => !!t && t.status !== Status.DONE);
	const dependents = tasks.filter((t) => t.depends_on.includes(task.id));
	const blocks = dependents.filter((t) => t.status !== Status.DONE);
	const ids = (list: Task[]) => list.map((t) => shortId(t)).join(', ');
	if (task.status === Status.BLOCKED && open.length > 0) {
		return { text: `Waits on ${ids(open)}`, warn: true };
	}
	if (task.tags.includes('chore')) return { text: 'Chore', warn: false };
	if (task.status === Status.DONE) return { text: '', warn: false };
	if (open.length > 0) return { text: `Waits on ${ids(open)}`, warn: false };
	if (blocks.length > 0) return { text: `Blocks ${ids(blocks)}`, warn: false };
	if (dependents.length === 0 && task.depends_on.length > 0) {
		return { text: 'Terminal node', warn: false };
	}
	return { text: task.depends_on.length === 0 ? 'No dependencies' : '', warn: false };
}
