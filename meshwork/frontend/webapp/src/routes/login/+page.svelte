<script lang="ts">
	import { goto } from '$app/navigation';
	import MockBadge from '$lib/components/MockBadge.svelte';

	let mode = $state<'in' | 'up'>('in');
	let name = $state('');
	let email = $state('');
	let password = $state('');
	let error = $state('');

	function submit(event: SubmitEvent) {
		event.preventDefault();
		error = '';
		if (mode === 'up' && password.length < 10) {
			error = 'Use at least 10 characters for the password';
			return;
		}
		// MOCK: the backend has no auth yet (TODO.md, "Дыры API для нового UI"), so just enter the app.
		goto('/graphs');
	}

	const tab = (active: boolean) =>
		`flex-1 h-9 border-0 rounded-lg text-sm cursor-pointer ${
			active
				? 'bg-surface text-ink font-semibold shadow-[0_1px_2px_rgba(18,22,28,.12)]'
				: 'bg-transparent text-ink-3 font-medium'
		}`;
</script>

<svelte:head><title>{mode === 'in' ? 'Sign in' : 'Sign up'} · Meshwork</title></svelte:head>

<div class="bg-canvas flex min-h-screen flex-wrap">
	<main
		class="box-border flex min-w-0 flex-[1_1_520px] flex-col items-center justify-center px-6 py-12"
	>
		<div class="flex w-full max-w-[400px] flex-col gap-7">
			<div class="flex items-center gap-3">
				<div class="bg-accent flex size-10 items-center justify-center rounded-xl">
					<svg
						width="22"
						height="22"
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
				</div>
				<span class="text-[22px] font-semibold tracking-tight">Meshwork</span>
			</div>

			<div class="flex flex-col gap-2">
				<h1 class="m-0 text-[32px] leading-[1.15] font-semibold tracking-tight">
					{mode === 'in' ? 'Welcome back' : 'Create your account'}
				</h1>
				<p class="text-muted m-0 text-base leading-normal">
					{mode === 'in'
						? 'Sign in to see what is blocking what.'
						: 'Start with one goal and map the work that leads to it.'}
				</p>
			</div>

			<div
				role="tablist"
				aria-label="Sign in or sign up"
				class="bg-surface-2 flex gap-1 rounded-[10px] p-1"
			>
				<button
					type="button"
					role="tab"
					aria-selected={mode === 'in'}
					class={tab(mode === 'in')}
					onclick={() => (mode = 'in')}>Sign in</button
				>
				<button
					type="button"
					role="tab"
					aria-selected={mode === 'up'}
					class={tab(mode === 'up')}
					onclick={() => (mode = 'up')}>Sign up</button
				>
			</div>

			<form class="flex flex-col gap-4" onsubmit={submit}>
				{#if error}
					<div class="bg-st-blocked-bg text-danger rounded-lg p-3 text-sm" role="alert">
						{error}
					</div>
				{/if}
				{#if mode === 'up'}
					<div class="flex flex-col gap-1.5">
						<label for="name" class="text-sm font-medium">Full name</label>
						<input
							id="name"
							type="text"
							class="field"
							placeholder="Andrei Chirkin"
							bind:value={name}
						/>
					</div>
				{/if}
				<div class="flex flex-col gap-1.5">
					<label for="email" class="text-sm font-medium">Work email</label>
					<input
						id="email"
						type="email"
						class="field"
						placeholder="you@company.com"
						required
						bind:value={email}
					/>
				</div>
				<div class="flex flex-col gap-1.5">
					<div class="flex items-baseline justify-between">
						<label for="pw" class="text-sm font-medium">Password</label>
						{#if mode === 'in'}<a href="#forgot" class="text-sm">Forgot password?</a>{/if}
					</div>
					<input
						id="pw"
						type="password"
						class="field"
						placeholder="At least 10 characters"
						required
						bind:value={password}
					/>
				</div>
				<button type="submit" class="btn btn-primary mt-1 h-12 text-base">
					{mode === 'in' ? 'Sign in' : 'Create account'}
				</button>
			</form>

			<p class="text-muted m-0 flex flex-wrap items-center gap-2 text-sm leading-normal">
				<MockBadge note="There is no auth API yet: submitting just opens the app" />
				Every change to a task is logged with who, what and when. Signing in means you agree to that.
			</p>
		</div>
	</main>

	<aside
		class="bg-rail box-border flex min-h-[420px] min-w-0 flex-[1_1_520px] flex-col items-center justify-center gap-10 px-8 py-12 text-white"
	>
		<svg
			viewBox="0 0 560 360"
			class="h-auto w-full max-w-[560px]"
			role="img"
			aria-label="Illustration of tasks as connected nodes leading to one goal"
		>
			<defs>
				<marker
					id="ah"
					viewBox="0 0 10 10"
					refX="9"
					refY="5"
					markerWidth="7"
					markerHeight="7"
					orient="auto"><path d="M0 0L10 5L0 10z" fill="#7C8796" /></marker
				>
			</defs>
			<g fill="none" stroke="#5E6A7A" stroke-width="2" marker-end="url(#ah)">
				<path d="M150 90 C190 90 190 150 230 150" />
				<path d="M150 230 C190 230 190 170 230 170" />
				<path d="M350 160 L400 160" />
				<path d="M150 300 C300 300 300 190 400 180" stroke-dasharray="4 5" />
			</g>
			<g font-family="IBM Plex Sans, sans-serif" font-size="14" font-weight="500">
				<rect
					x="20"
					y="60"
					width="130"
					height="60"
					rx="12"
					fill="#1C2430"
					stroke="#2F7D5B"
					stroke-width="2"
				/>
				<circle cx="38" cy="90" r="5" fill="#34C28A" /><text x="52" y="95" fill="#E7EBF0"
					>Storage</text
				>
				<rect
					x="20"
					y="200"
					width="130"
					height="60"
					rx="12"
					fill="#1C2430"
					stroke="#3E63D8"
					stroke-width="2"
				/>
				<circle cx="38" cy="230" r="5" fill="#6C8CF5" /><text x="52" y="235" fill="#E7EBF0"
					>Backend</text
				>
				<rect
					x="20"
					y="270"
					width="130"
					height="60"
					rx="12"
					fill="#1C2430"
					stroke="#3A4452"
					stroke-width="2"
				/>
				<circle cx="38" cy="300" r="5" fill="#8995A5" /><text x="52" y="305" fill="#E7EBF0"
					>Press kit</text
				>
				<rect
					x="230"
					y="125"
					width="120"
					height="60"
					rx="12"
					fill="#1C2430"
					stroke="#D9622B"
					stroke-width="2"
				/>
				<circle cx="248" cy="155" r="5" fill="#F0885A" /><text x="262" y="160" fill="#E7EBF0"
					>Integrate</text
				>
				<rect x="400" y="120" width="140" height="72" rx="14" fill="#2349C7" />
				<text x="420" y="162" fill="#fff" font-size="16" font-weight="600">Ship auth</text>
			</g>
		</svg>
		<div class="flex max-w-[440px] flex-col gap-3 text-center">
			<h2 class="m-0 text-[28px] leading-tight font-semibold tracking-tight">
				Dependencies visible by default
			</h2>
			<p class="m-0 text-base leading-normal text-[#b4bdc9]">
				Every task is a node. Every blocker is an edge. See why work is stuck without another sync
				call.
			</p>
		</div>
	</aside>
</div>
