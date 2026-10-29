import {
	api,
	ApiError,
	Status,
	type Actor,
	type ActorKind,
	type Graph,
	type NewTask,
	type Task,
	type TaskUpdate
} from './api';
import { mergeById } from './liveRefresh';
import { session } from './session.svelte';
import { shortId } from './status';

export type TaskFilter = 'all' | 'mine' | 'ready' | 'blocked';

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

	/** Everyone who can be assigned, by name. Global, not per graph. */
	actors = $state.raw<Actor[]>([]);
	actorsById = $derived(new Map(this.actors.map((a) => [a.id, a])));

	/** Distinct assignee IDs of the graph's tasks, by actor name. */
	users = $derived(
		[...new Set(this.tasks.flatMap((t) => t.users))].sort((a, b) =>
			this.actorName(a).localeCompare(this.actorName(b))
		)
	);

	/** Display name of an actor; the ID itself if the actor is not loaded. */
	actorName(id: string): string {
		return this.actorsById.get(id)?.name ?? id;
	}

	doneCount = $derived(this.tasks.filter((t) => t.status === Status.DONE).length);
	blockedCount = $derived(this.tasks.filter((t) => t.status === Status.BLOCKED).length);
	/** The frontier, as GET ready_tasks has it: TODO tasks whose blockers are all DONE. */
	ready = $derived(
		new Set(
			this.tasks
				.filter(
					(t) =>
						t.status === Status.TODO &&
						t.depends_on.every((id) => this.byId.get(id)?.status === Status.DONE)
				)
				.map((t) => t.id)
		)
	);
	/** Tasks assigned to the signed-in actor; 0 when signed out. */
	mineCount = $derived(this.tasks.filter((t) => isMine(t)).length);

	visible = $derived(this.tasks.filter((t) => this.matches(t)));

	matches(task: Task): boolean {
		if (this.filter === 'blocked') return task.status === Status.BLOCKED;
		if (this.filter === 'ready') return this.ready.has(task.id);
		// signed out meanwhile: show everything rather than nothing
		if (this.filter === 'mine' && session.me) return isMine(task);
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

	/** Saves where nodes sit on the canvas, so the layout survives reloads and other browsers. */
	async saveLayout(positions: Iterable<[string, { x: number; y: number }]>) {
		const body = Object.fromEntries(positions);
		if (Object.keys(body).length === 0) return;
		try {
			await api.saveLayout(this.graphId, body);
		} catch (error) {
			// the canvas still shows the new layout; it is just not saved
			console.warn('Cannot save the layout:', error);
		}
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
			const [graph, tasks, actors] = await Promise.all([
				api.getGraph(graphId),
				api.getAllTasks(graphId),
				api.listActors()
			]);
			// another graph opened, an own write overlapped, or an edit began meanwhile
			if (graphId !== this.graphId || epoch !== this.epoch || this.holds.size > 0) return;
			if (graph.name !== this.graph?.name || graph.key !== this.graph?.key) this.graph = graph;
			this.tasks = mergeById(this.tasks, tasks);
			this.actors = mergeById(this.actors, actors);
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
			const [graph, tasks, actors] = await Promise.all([
				api.getGraph(graphId),
				api.getAllTasks(graphId),
				api.listActors()
			]);
			this.graph = graph;
			this.tasks = tasks;
			this.actors = actors;
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

	async addActor(name: string, kind: ActorKind): Promise<Actor | null> {
		try {
			const created = await this.write(() => api.addActor({ name, kind }));
			this.actors = [...this.actors, created].sort((a, b) => a.name.localeCompare(b.name));
			this.error = '';
			return created;
		} catch (error) {
			this.fail('add actor', error);
			return null;
		}
	}

	/** Changes the prefix of the task keys; every task's key changes with it. */
	async setKey(key: string): Promise<boolean> {
		try {
			const graphId = this.graphId;
			const [graph, tasks] = await this.write(async () => {
				const graph = await api.setGraphKey(graphId, key);
				return [graph, await api.getAllTasks(graphId)] as const;
			});
			if (graphId !== this.graphId) return false;
			this.graph = graph;
			this.tasks = mergeById(this.tasks, tasks);
			this.error = '';
			return true;
		} catch (error) {
			this.fail('change the task key', error);
			return false;
		}
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

/** Assigned to the signed-in actor. */
export function isMine(task: Task): boolean {
	return !!session.me && task.users.includes(session.me.id);
}

/** Why a task is waiting on the signed-in actor, or null: an open task of theirs that blocks
 * other open tasks, or a task in review they reported. */
export function waitingReason(task: Task, tasks: Task[]): string | null {
	const me = session.me;
	if (!me || task.status === Status.DONE) return null;
	if (task.status === Status.REVIEW && task.reporter === me.id) return 'Waiting for your review';
	if (!task.users.includes(me.id)) return null;
	const blocks = tasks.filter(
		(t) => t.status !== Status.DONE && t.depends_on.includes(task.id)
	).length;
	return blocks > 0 ? `Blocks ${blocks} ${blocks === 1 ? 'task' : 'tasks'}` : null;
}
