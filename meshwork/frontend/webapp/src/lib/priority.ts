import { Priority } from './api';

export const PRIORITY_LABEL: Record<Priority, string> = {
	[Priority.HIGH]: 'High',
	[Priority.NORMAL]: 'Normal',
	[Priority.LOW]: 'Low'
};

/** Select and sort order: most urgent first. */
export const PRIORITY_ORDER: Priority[] = [Priority.HIGH, Priority.NORMAL, Priority.LOW];

/** Short text for a PR link: `owner/repo#12` for GitHub pull requests, else host and path. */
export function prLabel(url: string): string {
	try {
		const u = new URL(url);
		const gh = u.hostname === 'github.com' && u.pathname.match(/^\/([^/]+\/[^/]+)\/pull\/(\d+)/);
		if (gh) return `${gh[1]}#${gh[2]}`;
		return `${u.hostname}${u.pathname.replace(/\/$/, '')}`;
	} catch {
		return url;
	}
}
