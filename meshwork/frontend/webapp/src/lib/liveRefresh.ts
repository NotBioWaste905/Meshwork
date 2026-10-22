/** How often a visible tab re-fetches data that agents may have changed. */
export const POLL_INTERVAL_MS = 12_000;
/** Focus and visibility events often fire together; one refresh is enough. */
const MIN_GAP_MS = 2_000;

/**
 * Calls `refresh` every `intervalMs` while the tab is visible, and right away when the tab
 * becomes visible again or the window regains focus. Hidden tabs do not poll.
 * Returns a cleanup function, so it can be returned from `$effect` or `onMount`.
 */
export function liveRefresh(refresh: () => unknown, intervalMs = POLL_INTERVAL_MS): () => void {
	let last = Date.now();
	let timer: ReturnType<typeof setInterval> | undefined;

	const run = () => {
		if (document.visibilityState !== 'visible' || Date.now() - last < MIN_GAP_MS) return;
		last = Date.now();
		refresh();
	};
	const start = () => {
		stop();
		if (document.visibilityState === 'visible') timer = setInterval(run, intervalMs);
	};
	const stop = () => {
		if (timer !== undefined) clearInterval(timer);
		timer = undefined;
	};
	const onVisibility = () => {
		run();
		start();
	};

	document.addEventListener('visibilitychange', onVisibility);
	window.addEventListener('focus', run);
	start();
	return () => {
		stop();
		document.removeEventListener('visibilitychange', onVisibility);
		window.removeEventListener('focus', run);
	};
}

/** True when two lists hold the same values, ignoring order (edge and assignee lists). */
function sameSet(a: readonly string[], b: readonly string[]): boolean {
	if (a.length !== b.length) return false;
	const set = new Set(a);
	return b.every((x) => set.has(x));
}

/**
 * Merges a freshly fetched list into the current one by `id`: unchanged items keep their old
 * object, and if nothing changed at all the current array itself is returned, so Svelte sees
 * no change and views (the canvas layout in particular) are not rebuilt.
 */
export function mergeById<T extends { id: string }>(current: T[], fresh: T[]): T[] {
	const byId = new Map(current.map((item) => [item.id, item]));
	let changed = current.length !== fresh.length;
	const merged = fresh.map((item) => {
		const old = byId.get(item.id);
		if (old && sameValue(old, item)) return old;
		changed = true;
		return item;
	});
	return changed ? merged : current;
}

function sameValue(a: object, b: object): boolean {
	const ka = Object.keys(a);
	if (ka.length !== Object.keys(b).length) return false;
	return ka.every((key) => {
		const x = (a as Record<string, unknown>)[key];
		const y = (b as Record<string, unknown>)[key];
		// the backend collects edges and assignees in no particular order
		if ((key === 'depends_on' || key === 'users') && Array.isArray(x) && Array.isArray(y)) {
			return sameSet(x, y);
		}
		return JSON.stringify(x) === JSON.stringify(y);
	});
}
