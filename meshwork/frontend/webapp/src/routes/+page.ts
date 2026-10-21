import { redirect } from '@sveltejs/kit';

// No landing page in the new design: the app starts at the graph list.
export function load() {
	redirect(307, '/graphs');
}
