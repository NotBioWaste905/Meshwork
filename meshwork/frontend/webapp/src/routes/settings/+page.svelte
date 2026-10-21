<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import Avatar from '$lib/components/Avatar.svelte';
	import MockBadge from '$lib/components/MockBadge.svelte';
	import { CURRENT_USER, NOTIFICATION_PREFS, type NotificationPref } from '$lib/mock';

	const PREFS_KEY = 'meshwork:notificationPrefs';

	let name = $state(CURRENT_USER.name);
	let email = $state(CURRENT_USER.email);
	let saved = $state(false);
	let prefs = $state<NotificationPref[]>(NOTIFICATION_PREFS.map((p) => ({ ...p })));

	// Preferences are a per-browser convenience until there is a user API.
	onMount(() => {
		try {
			const stored = JSON.parse(localStorage.getItem(PREFS_KEY) ?? '{}');
			prefs = prefs.map((p) => ({
				...p,
				on: typeof stored[p.id] === 'boolean' ? stored[p.id] : p.on
			}));
		} catch {
			// storage unavailable, keep the defaults
		}
	});

	function togglePref(id: string, on: boolean) {
		prefs = prefs.map((p) => (p.id === id ? { ...p, on } : p));
		try {
			localStorage.setItem(
				PREFS_KEY,
				JSON.stringify(Object.fromEntries(prefs.map((p) => [p.id, p.on])))
			);
		} catch {
			// storage unavailable
		}
	}

	function save(event: SubmitEvent) {
		event.preventDefault();
		// MOCK: there is no user API, nothing is stored.
		saved = true;
	}

	const nav = 'flex h-11 items-center rounded-[10px] px-3.5 text-[15px] no-underline';
</script>

<svelte:head><title>Settings · Meshwork</title></svelte:head>

<main class="box-border flex flex-col gap-7 px-10 pt-8 pb-12">
	<h1 class="m-0 text-3xl font-semibold tracking-tight">Settings</h1>
	<div class="flex flex-wrap items-start gap-8">
		<nav aria-label="Settings sections" class="flex flex-[0_0_200px] flex-col gap-0.5">
			<a
				href="#profile"
				aria-current="page"
				class="{nav} border-line bg-surface text-ink border font-semibold">Profile</a
			>
			<a href="#notifications" class="{nav} text-ink-3 font-medium">Notifications</a>
			<span class="{nav} text-faint cursor-not-allowed font-medium" title="Not available yet"
				>Members and roles</span
			>
			<span class="{nav} text-faint cursor-not-allowed font-medium" title="Not available yet"
				>API tokens</span
			>
		</nav>

		<div class="flex max-w-[720px] min-w-0 flex-[1_1_480px] flex-col gap-5">
			<section id="profile" aria-labelledby="pf" class="card flex flex-col gap-[18px] p-6">
				<h2 id="pf" class="m-0 flex items-center gap-2 text-lg font-semibold">
					Profile <MockBadge note="Demo data: there is no user API, changes are not stored" />
				</h2>
				<div class="flex items-center gap-4">
					<Avatar name={CURRENT_USER.name} size={64} />
					<button type="button" class="btn btn-secondary btn-sm" disabled>Change photo</button>
				</div>
				<form class="flex flex-col gap-[18px]" onsubmit={save}>
					<div class="grid grid-cols-[repeat(auto-fit,minmax(220px,1fr))] gap-4">
						<div class="flex flex-col gap-1.5">
							<label for="fn" class="text-sm font-medium">Full name</label>
							<input id="fn" type="text" class="field" bind:value={name} />
						</div>
						<div class="flex flex-col gap-1.5">
							<label for="em" class="text-sm font-medium">Email</label>
							<input id="em" type="email" class="field" bind:value={email} />
						</div>
					</div>
					<div class="flex items-center gap-3">
						<button type="submit" class="btn btn-primary">Save changes</button>
						{#if saved}
							<span class="text-muted text-sm" role="status"
								>Demo only: the profile is not stored anywhere yet.</span
							>
						{/if}
					</div>
				</form>
			</section>

			<section id="notifications" aria-labelledby="nt" class="card flex flex-col gap-1.5 p-6">
				<h2 id="nt" class="m-0 mb-2 flex items-center gap-2 text-lg font-semibold">
					Notify me when <MockBadge
						note="Kept in this browser only: nothing sends notifications yet"
					/>
				</h2>
				{#each prefs as p (p.id)}
					<div class="border-line-soft flex items-center gap-3.5 border-t py-3">
						<input
							id={p.id}
							type="checkbox"
							checked={p.on}
							onchange={(e) => togglePref(p.id, e.currentTarget.checked)}
						/>
						<label for={p.id} class="flex flex-1 flex-col gap-0.5 text-[15px] font-medium"
							>{p.title}<span class="text-muted text-[13px] font-normal">{p.hint}</span></label
						>
					</div>
				{/each}
			</section>

			<section
				aria-labelledby="ss"
				class="card flex flex-wrap items-center justify-between gap-4 p-6"
			>
				<div class="flex flex-col gap-1">
					<h2 id="ss" class="m-0 text-lg font-semibold">Session</h2>
					<span class="text-muted text-sm">Signed in on this device</span>
				</div>
				<button type="button" class="btn btn-secondary text-danger" onclick={() => goto('/login')}
					>Sign out</button
				>
			</section>
		</div>
	</div>
</main>
