import { Status, type Task } from './api';

export type StatusMeta = {
	/** Mixed-case label for prose and selects. */
	label: string;
	/** Upper-case label for pills and column headers. */
	pill: string;
	bg: string;
	fg: string;
	dot: string;
};

const token = (name: string) => `var(--color-st-${name})`;

const meta = (key: string, label: string, pill: string): StatusMeta => ({
	label,
	pill,
	bg: token(`${key}-bg`),
	fg: token(`${key}-fg`),
	dot: token(`${key}-dot`)
});

export const STATUS_META: Record<Status, StatusMeta> = {
	[Status.TODO]: meta('todo', 'To do', 'TODO'),
	[Status.IN_PROGRESS]: meta('progress', 'In progress', 'IN PROGRESS'),
	[Status.REVIEW]: meta('review', 'Review', 'REVIEW'),
	[Status.DONE]: meta('done', 'Done', 'DONE'),
	[Status.BLOCKED]: meta('blocked', 'Blocked', 'BLOCKED')
};

/** Board column order. */
export const STATUS_ORDER: Status[] = [
	Status.TODO,
	Status.IN_PROGRESS,
	Status.REVIEW,
	Status.BLOCKED,
	Status.DONE
];

/** How a task is referred to: its key (`MW-12`). Tasks without a number yet (created by an
 * older backend) fall back to the first chars of the UUID. */
export function shortId(task: (Pick<Task, 'id'> & Partial<Pick<Task, 'key'>>) | string): string {
	if (typeof task !== 'string' && task.key) return task.key;
	const id = typeof task === 'string' ? task : task.id;
	return `T-${id.replace(/-/g, '').slice(0, 4)}`;
}
