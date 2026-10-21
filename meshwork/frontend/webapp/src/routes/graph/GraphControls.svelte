<script lang="ts">
	import { Panel, useSvelteFlow, useViewport } from '@xyflow/svelte';

	let { onLayout }: { onLayout: () => void } = $props();

	const { zoomIn, zoomOut, fitView } = useSvelteFlow();
	const viewport = useViewport();
</script>

<Panel position="bottom-left">
	<div
		role="group"
		aria-label="Zoom"
		class="border-line bg-surface flex items-center overflow-hidden rounded-[10px] border"
	>
		<button type="button" class="ctl w-10 text-lg" aria-label="Zoom out" onclick={() => zoomOut()}
			>−</button
		>
		<span class="text-ink-3 px-2.5 text-[13px] font-medium"
			>{Math.round(viewport.current.zoom * 100)}%</span
		>
		<button type="button" class="ctl w-10 text-lg" aria-label="Zoom in" onclick={() => zoomIn()}
			>+</button
		>
		<button
			type="button"
			class="ctl border-line border-l px-3.5 text-[13px] font-medium"
			onclick={() => fitView({ duration: 200 })}>Fit</button
		>
		<button
			type="button"
			class="ctl border-line border-l px-3.5 text-[13px] font-medium"
			onclick={onLayout}>Auto layout</button
		>
	</div>
</Panel>

<style>
	.ctl {
		height: 2.5rem;
		background: var(--color-surface);
		color: var(--color-ink);
		cursor: pointer;
		transition:
			background-color 120ms ease,
			color 120ms ease;
	}
	.ctl:hover {
		background: var(--color-canvas);
		color: var(--color-accent);
	}
	.ctl:active {
		background: var(--color-surface-2);
		color: var(--color-accent-dark);
	}
	/* the group clips its overflow, so draw the focus ring inside the button */
	.ctl:focus-visible {
		outline: 2px solid var(--color-accent);
		outline-offset: -2px;
	}
</style>
