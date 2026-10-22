<script lang="ts">
	import { api, type OwnToken } from '$lib/api';
	import { session } from '$lib/session.svelte';

	/** The signed-in actor's tokens: list, issue another, revoke. */
	let tokens = $state<OwnToken[]>([]);
	let label = $state('');
	/** A token just issued: shown once, until dismissed. */
	let issued = $state<{ token: string; label: string } | null>(null);
	let copied = $state(false);
	let busy = $state(false);
	let error = $state('');

	async function load() {
		try {
			tokens = await api.listOwnTokens();
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	$effect(() => {
		load();
	});

	async function issue(event: SubmitEvent) {
		event.preventDefault();
		if (busy) return;
		busy = true;
		try {
			const result = await api.issueOwnToken(label.trim());
			issued = { token: result.token, label: result.info.label };
			copied = false;
			label = '';
			error = '';
			await load();
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
		busy = false;
	}

	async function revoke(token: OwnToken) {
		const what = token.label ? `"${token.label}"` : 'this token';
		const warning = token.current
			? `Revoke ${what}? It is the one this browser uses, so you will be signed out.`
			: `Revoke ${what}? Anything using it stops working.`;
		if (busy || !confirm(warning)) return;
		busy = true;
		try {
			await api.revokeOwnToken(token.id);
			if (token.current) {
				session.signOut();
				return;
			}
			await load();
			error = '';
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			busy = false;
		}
	}

	async function copy() {
		if (!issued) return;
		try {
			await navigator.clipboard.writeText(issued.token);
			copied = true;
		} catch {
			// clipboard blocked: the field is selectable
		}
	}

	const dateFormat = new Intl.DateTimeFormat(undefined, { dateStyle: 'medium' });
</script>

<section aria-labelledby="tokens" class="card flex flex-col gap-4 p-6">
	<div class="flex flex-col gap-1">
		<h2 id="tokens" class="m-0 text-[17px] font-semibold">Your tokens</h2>
		<p class="text-muted m-0 text-sm">
			Each device or agent acting as you should have its own, so you can revoke one without the
			others. A token is shown only once, when it is issued.
		</p>
	</div>

	{#if issued}
		<div class="bg-st-done-bg flex flex-col gap-2 rounded-lg p-4" role="status">
			<strong class="text-st-done-fg text-sm font-semibold"
				>New token{issued.label ? ` "${issued.label}"` : ''}: copy it now, it will not be shown
				again</strong
			>
			<div class="flex flex-wrap items-center gap-2">
				<input
					class="field min-w-0 flex-1 font-mono text-sm"
					readonly
					aria-label="New token"
					value={issued.token}
					onfocus={(e) => e.currentTarget.select()}
				/>
				<button type="button" class="btn btn-secondary btn-sm" onclick={copy}
					>{copied ? 'Copied' : 'Copy'}</button
				>
				<button type="button" class="btn btn-secondary btn-sm" onclick={() => (issued = null)}
					>Done</button
				>
			</div>
		</div>
	{/if}

	<ul class="m-0 list-none divide-y divide-[var(--color-line-soft)] p-0" aria-label="Tokens">
		{#each tokens as t (t.id)}
			<li class="flex flex-wrap items-center gap-x-4 gap-y-1 py-3">
				<span class="min-w-0 flex-1 text-[15px] font-medium"
					>{t.label || 'Unlabelled'}{#if t.current}<span
							class="bg-st-progress-bg text-st-progress-fg ml-2 rounded-full px-2 py-0.5 text-xs font-semibold"
							>This browser</span
						>{/if}</span
				>
				<span class="text-muted text-sm">Issued {dateFormat.format(t.created_at)}</span>
				<span class="text-faint font-mono text-xs" title="Token ID, for meshwork-token revoke"
					>{t.id.slice(0, 8)}</span
				>
				<button
					type="button"
					class="btn btn-secondary btn-sm text-danger"
					disabled={busy}
					onclick={() => revoke(t)}>Revoke</button
				>
			</li>
		{/each}
	</ul>

	<form class="flex flex-wrap items-center gap-3" onsubmit={issue}>
		<input
			type="text"
			class="field w-64 text-[15px]"
			placeholder="Label, e.g. work laptop"
			aria-label="New token label"
			maxlength="100"
			autocomplete="off"
			bind:value={label}
		/>
		<button type="submit" class="btn btn-primary" disabled={busy}>Issue a token</button>
	</form>

	{#if error}
		<div class="bg-st-blocked-bg text-danger rounded-lg p-3 text-sm" role="alert">{error}</div>
	{/if}
</section>
