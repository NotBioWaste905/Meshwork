<script lang="ts">
	import { page } from '$app/state';

	let path = $derived(page.url.pathname);
	let onGraphs = $derived(path.startsWith('/graph'));
	let onActors = $derived(path.startsWith('/actors'));

	const item = 'flex size-11 items-center justify-center rounded-[10px]';
</script>

<nav
	aria-label="Primary"
	class="bg-rail sticky top-0 box-border flex h-screen w-16 flex-none flex-col items-center gap-2 py-4"
>
	<a
		href="/graphs"
		aria-label="Meshwork home"
		class="bg-accent mb-4 flex size-9 items-center justify-center rounded-[10px]"
	>
		<svg
			width="20"
			height="20"
			viewBox="0 0 24 24"
			fill="none"
			stroke="#fff"
			stroke-width="2"
			stroke-linecap="round"
			aria-hidden="true"
		>
			<circle cx="6" cy="12" r="2.5" /><circle cx="18" cy="6" r="2.5" /><circle
				cx="18"
				cy="18"
				r="2.5"
			/>
			<path d="M8.3 11l7.4-4M8.3 13l7.4 4" />
		</svg>
	</a>
	<a
		href="/graphs"
		aria-label="Graphs"
		aria-current={onGraphs ? 'page' : undefined}
		class="{item} {onGraphs ? 'bg-rail-active text-white' : 'text-rail-ink hover:text-white'}"
	>
		<svg
			width="22"
			height="22"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			stroke-width="1.8"
			stroke-linecap="round"
			stroke-linejoin="round"
			aria-hidden="true"
		>
			<rect x="4" y="4" width="7" height="7" rx="1.5" /><rect
				x="13"
				y="4"
				width="7"
				height="7"
				rx="1.5"
			/>
			<rect x="4" y="13" width="7" height="7" rx="1.5" /><rect
				x="13"
				y="13"
				width="7"
				height="7"
				rx="1.5"
			/>
		</svg>
	</a>
	<a
		href="/actors"
		aria-label="Actors"
		aria-current={onActors ? 'page' : undefined}
		class="{item} {onActors ? 'bg-rail-active text-white' : 'text-rail-ink hover:text-white'}"
	>
		<svg
			width="22"
			height="22"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			stroke-width="1.8"
			stroke-linecap="round"
			stroke-linejoin="round"
			aria-hidden="true"
		>
			<circle cx="9" cy="8" r="3.5" /><path d="M2.5 20c0-3.6 2.9-6 6.5-6s6.5 2.4 6.5 6" />
			<path d="M16 4.6a3.5 3.5 0 0 1 0 6.8M18.5 14.4c1.9.8 3 2.9 3 5.6" />
		</svg>
	</a>
</nav>
