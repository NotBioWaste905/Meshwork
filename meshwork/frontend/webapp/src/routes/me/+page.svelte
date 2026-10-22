<script lang="ts">
	import Avatar from '$lib/components/Avatar.svelte';
	import OwnTokens from '$lib/components/OwnTokens.svelte';
	import { session } from '$lib/session.svelte';

	let token = $state('');
	let error = $state('');
	let saving = $state(false);

	async function signIn(event: SubmitEvent) {
		event.preventDefault();
		if (saving) return;
		saving = true;
		error = await session.signIn(token);
		saving = false;
		if (!error) token = '';
	}
</script>

<svelte:head><title>You · Meshwork</title></svelte:head>

<main class="box-border flex max-w-2xl flex-col gap-8 px-10 pt-8 pb-12">
	<header class="flex flex-col gap-1">
		<h1 class="m-0 text-3xl font-semibold tracking-tight">You</h1>
		<p class="text-muted m-0 text-[15px]">
			Who this browser acts as. Tasks you create get you as reporter, and the Mine filter and
			Waiting on you follow you.
		</p>
	</header>

	{#if !session.ready}
		<p class="text-muted m-0">Checking…</p>
	{:else if session.me}
		<section class="card flex flex-wrap items-center gap-4 p-6" aria-label="Signed in as">
			<Avatar name={session.me.name} size={48} agent={session.me.kind === 'agent'} />
			<div class="flex min-w-0 flex-1 flex-col gap-0.5">
				<strong class="text-lg font-semibold">{session.me.name}</strong>
				<span class="text-muted text-sm"
					>{session.me.kind === 'agent' ? 'Agent' : 'Human'} · {session.me.task_count}
					{session.me.task_count === 1 ? 'task' : 'tasks'} assigned</span
				>
			</div>
			<button type="button" class="btn btn-secondary btn-sm" onclick={() => session.signOut()}
				>Sign out</button
			>
		</section>
		<OwnTokens />
		<p class="text-muted m-0 text-sm">
			People and agents are managed on the <a href="/actors">Actors</a> screen: add, rename, merge duplicates,
			delete.
		</p>
	{:else}
		{#if session.expired}
			<div class="bg-st-blocked-bg text-danger rounded-lg p-3 text-sm" role="alert">
				The saved token no longer works (revoked?), so you were signed out.
			</div>
		{/if}
		<form class="card flex flex-col gap-4 p-6" onsubmit={signIn}>
			<div class="flex flex-col gap-1.5">
				<label for="me-token" class="text-sm font-medium">Actor token</label>
				<input
					id="me-token"
					type="password"
					class="field font-mono"
					placeholder="mw_…"
					autocomplete="off"
					spellcheck="false"
					bind:value={token}
				/>
			</div>
			{#if error}
				<div class="bg-st-blocked-bg text-danger rounded-lg p-3 text-sm" role="alert">{error}</div>
			{/if}
			<div>
				<button type="submit" class="btn btn-primary" disabled={saving || !token.trim()}
					>{saving ? 'Checking…' : 'Sign in'}</button
				>
			</div>
			<p class="text-muted m-0 text-sm leading-relaxed">
				Tokens are issued on the server, for an existing actor:
				<code>scripts/mcp-token.sh &lt;your name&gt; ~/meshwork.token</code>
				or <code>docker compose exec -T backend meshwork-token create &lt;your name&gt;</code>. The
				token stays in this browser only.
			</p>
		</form>
	{/if}
</main>
