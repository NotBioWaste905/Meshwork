import type { Priority, Status, TaskEvent } from './api';
import { PRIORITY_LABEL, prLabel } from './priority';
import { shortId, STATUS_META } from './status';

/** How the log's IDs are shown: names of actors and tasks, from the open graph. */
export type Names = {
	actor: (id: string) => string;
	task: (id: string) => string;
};

const list = (value: unknown): string[] => (Array.isArray(value) ? value.map(String) : []);
const text = (value: unknown): string => (typeof value === 'string' ? value : '');

/** "moved it from To do to In progress": what the event did, after the actor's name. */
export function describe(event: TaskEvent, names: Names): string {
	if (event.kind === 'created') return 'created the task';
	if (event.kind === 'blocker_added') return `added blocker ${names.task(text(event.to))}`;
	if (event.kind === 'blocker_removed') return `removed blocker ${names.task(text(event.to))}`;

	const from = event.from;
	const to = event.to;
	switch (event.field) {
		case 'name':
			return `renamed it from “${text(from)}” to “${text(to)}”`;
		case 'description':
			return 'edited the description';
		case 'status':
			return `moved it from ${STATUS_META[from as Status]?.label ?? text(from)} to ${
				STATUS_META[to as Status]?.label ?? text(to)
			}`;
		case 'priority':
			return `set priority to ${PRIORITY_LABEL[to as Priority] ?? text(to)}`;
		case 'pr_url':
			return text(to) ? `linked PR ${prLabel(text(to))}` : 'removed the PR link';
		case 'reporter':
			return to ? `made ${names.actor(text(to))} the reporter` : 'removed the reporter';
		case 'users': {
			const before = list(from);
			const after = list(to);
			const added = after.filter((u) => !before.includes(u)).map(names.actor);
			const removed = before.filter((u) => !after.includes(u)).map(names.actor);
			const parts = [
				added.length ? `assigned ${added.join(', ')}` : '',
				removed.length ? `unassigned ${removed.join(', ')}` : ''
			].filter(Boolean);
			return parts.join(' and ') || 'changed the assignees';
		}
		case 'tags': {
			const after = list(to);
			return after.length ? `set tags to ${after.join(', ')}` : 'removed all tags';
		}
		default:
			return `changed ${event.field ?? 'something'}`;
	}
}

/** A task name for the log: its short ID and name, or just the ID once it is deleted. */
export function taskLabel(id: string, name: string | undefined): string {
	return name ? `${shortId(id)} ${name}` : shortId(id);
}
