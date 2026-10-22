<script lang="ts" module>
	import DOMPurify from 'dompurify';
	import { marked } from 'marked';
	import { browser } from '$app/environment';

	// Descriptions are written by agents and anyone with API access: the HTML is always sanitized,
	// and links open in a new tab without access to this one.
	if (browser) {
		DOMPurify.addHook('afterSanitizeAttributes', (node) => {
			if (node.tagName === 'A' && node.getAttribute('href')) {
				node.setAttribute('target', '_blank');
				node.setAttribute('rel', 'noopener noreferrer');
			}
		});
	}

	/** GitHub-flavoured Markdown; single newlines stay line breaks, as in the plain-text view before. */
	function render(source: string): string {
		const html = marked.parse(source, { gfm: true, breaks: true, async: false });
		return DOMPurify.sanitize(html);
	}
</script>

<script lang="ts">
	let { source, class: className = '' }: { source: string; class?: string } = $props();

	let html = $derived(browser ? render(source) : '');
</script>

{#if browser}
	<div class="prose prose-meshwork {className}">
		<!-- eslint-disable-next-line svelte/no-at-html-tags -- sanitized by DOMPurify above -->
		{@html html}
	</div>
{:else}
	<!-- DOMPurify needs a DOM: the server renders the text as is. -->
	<p class="whitespace-pre-line {className}">{source}</p>
{/if}
