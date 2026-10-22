<script lang="ts">
	let {
		name,
		size = 24,
		ring = false,
		agent = false
	}: {
		name: string;
		size?: number;
		/** White ring, for overlapping avatar stacks. */
		ring?: boolean;
		/** AI agents get a rounded square instead of a circle. */
		agent?: boolean;
	} = $props();

	const PALETTE = ['#0F766E', '#7A3FC4', '#B4530A', '#2349C7', '#A13A6B', '#3E6B1F'];

	let initials = $derived.by(() => {
		const words = name.trim().split(/\s+/).filter(Boolean);
		const letters = words.length > 1 ? words[0][0] + words[1][0] : name.trim().slice(0, 2);
		return letters.toUpperCase();
	});
	let color = $derived(
		PALETTE[[...name].reduce((sum, ch) => sum + ch.charCodeAt(0), 0) % PALETTE.length]
	);
</script>

<span
	class="inline-flex flex-none items-center justify-center font-semibold text-white {agent
		? 'rounded-[30%]'
		: 'rounded-full'}"
	class:border-2={ring}
	class:border-white={ring}
	style="width:{size}px;height:{size}px;background:{color};font-size:{Math.max(
		9,
		Math.round(size * 0.42)
	)}px"
	title={agent ? `${name} (agent)` : name}
	aria-label={agent ? `${name} (agent)` : name}>{initials}</span
>
