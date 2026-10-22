import { api, ApiError, Status, type Graph, type NewTask, type Task, type TaskUpdate } from './api';
import { mergeById } from './liveRefresh';
import { shortId } from './status';

export type TaskFilter = 'all' | 'blocked';

/** The tasks of the open graph, shared by the Graph, Board, Table and Task screens. */
class GraphStore {
	graphId = $state('');
	/** Graph metadata; null until loaded. Its counters are not kept up to date here. */
	graph = $state.raw<Graph | null>(null);
	tasks = $state.raw<Task[]>([]);
	loaded = $state(false);
	notFound = $state(false);
	error = $state('');
	filter = $state<TaskFilter>('all');
	/** The new-task dialog, rendered by the graph layout. */
	newTaskOpen = $state(false);

	/** Reasons background refresh is paused: open edit forms, a drag on the canvas. */
	private holds = new Set<symbol>();
	/** Own requests in flight; a refresh racing them could bring back stale data. */
	private pending = 0;
	/** Bumped by every own write, so a refresh that overlapped one is dropped. */
	private epoch = 0;
	private refreshing = false;

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

	/** The graph's name; graphs created without one are called after their goal task. */
	name = $derived(
		this.graph?.name ||
			(this.terminals[0]?.name ??
				(this.tasks.length === 1 ? this.tasks[0].name : `Graph ${this.graphId.slice(0, 8)}`))
	);

	/** Distinct assignees of the graph's tasks. Users are free-form names, there is no user API. */
	users = $derived([...new Set(this.tasks.flatMap((t) => t.users))].sort());

	doneCount = $derived(this.tasks.filter((t) => t.status === Status.DONE).length);
	blockedCount = $derived(this.tasks.filter((t) => t.status === Status.BLOCKED).length);

	visible = $derived(this.tasks.filter((t) => this.matches(t)));

	matches(task: Task): boolean {
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

	/**
	 * Pauses background refresh until the returned function is called. Use it around in-progress
	 * edits, e.g. `$effect(() => { if (editing) return graphStore.hold(); })`.
	 */
	hold(): () => void {
		const key = Symbol();
		this.holds.add(key);
		return () => this.holds.delete(key);
	}

	/**
	 * Quietly re-fetches the open graph to pick up changes made elsewhere (agents via MCP/REST,
	 * other tabs). Skipped while an edit holds it or an own write is in flight; unchanged tasks
	 * keep their objects, so a refresh that finds nothing new triggers no re-render.
	 */
	async refresh() {
		const graphId = this.graphId;
		if (!graphId || !this.loaded || this.notFound || this.refreshing) return;
		if (this.holds.size > 0 || this.pending > 0) return;
		const epoch = this.epoch;
		this.refreshing = true;
		try {
			const [graph, tasks] = await Promise.all([api.getGraph(graphId), api.getAllTasks(graphId)]);
			// another graph opened, an own write overlapped, or an edit began meanwhile
			if (graphId !== this.graphId || epoch !== this.epoch || this.holds.size > 0) return;
			if (graph.name !== this.graph?.name) this.graph = graph;
			this.tasks = mergeById(this.tasks, tasks);
		} catch (error) {
			// background refresh: keep the current data, the next tick tries again
			console.warn('Background refresh failed:', error);
		} finally {
			this.refreshing = false;
		}
	}

	/** Wraps an own API call so background refreshes neither race nor overwrite it. */
	private async write<T>(request: () => Promise<T>): Promise<T> {
		this.pending++;
		this.epoch++;
		try {
			return await request();
		} finally {
			this.pending--;
			this.epoch++;
		}
	}

	async load(graphId: string) {
		this.epoch++; // a refresh still in flight must not land on top of this load
		if (graphId !== this.graphId) {
			this.graphId = graphId;
			this.graph = null;
			this.tasks = [];
			this.loaded = false;
			this.notFound = false;
			this.filter = 'all';
		}
		try {
			const [graph, tasks] = await Promise.all([api.getGraph(graphId), api.getAllTasks(graphId)]);
			this.graph = graph;
			this.tasks = tasks;
			this.error = '';
		} catch (error) {
			if (error instanceof ApiError && error.status === 404) {
				this.notFound = true;
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
			const created = await this.write(() => api.addTask(this.graphId, task));
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
			const saved = await this.write(() => api.updateTask(this.graphId, id, patch));
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
			await this.write(() => api.deleteTask(this.graphId, id));
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
			await this.write(() => api.connectNodes(this.graphId, dependency, dependee));
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
			await this.write(() => api.disconnectNodes(this.graphId, dependency, dependee));
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

export const graphStore = new GraphStore();

/** Longest chain of dependencies, as task IDs from the first blocker to the last task. */
export function criticalPath(tasks: Task[]): string[] {
	const byId = new Map(tasks.map((t) => [t.id, t]));
	const memo = new Map<string, string[]>();
	const visiting = new Set<string>();

	const longest = (id: string): string[] => {
		const cached = memo.get(id);
		if (cached) return cached;
		if (visiting.has(id)) return [id]; // cycle guard; the backend rejects cycles, but stay safe
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
