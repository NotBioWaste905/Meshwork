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

	async load(graphId: string) {
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
			const created = await api.addActor({ name, kind });
			this.actors = [...this.actors, created].sort((a, b) => a.name.localeCompare(b.name));
			this.error = '';
			return created;
		} catch (error) {
			this.fail('add actor', error);
			return null;
		}
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
