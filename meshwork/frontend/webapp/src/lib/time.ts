const dateTimeFormat = new Intl.DateTimeFormat(undefined, {
	dateStyle: 'medium',
	timeStyle: 'short'
});

/** Full local date and time, for tooltips. */
export function formatDateTime(ms: number): string {
	return dateTimeFormat.format(ms);
}

/** "just now", "5 min ago", "3 h ago", "2 d ago"; the date once it is a month old. */
export function ago(ms: number): string {
	const minutes = Math.round((Date.now() - ms) / 60_000);
	if (minutes < 1) return 'just now';
	if (minutes < 60) return `${minutes} min ago`;
	const hours = Math.round(minutes / 60);
	if (hours < 24) return `${hours} h ago`;
	const days = Math.round(hours / 24);
	return days < 30 ? `${days} d ago` : dateTimeFormat.format(ms);
}
