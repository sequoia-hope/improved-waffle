<script>
	/**
	 * Read-only banner for a document opened from a share link
	 * (specs/waffle_v4_document_model.md §7.1): shows where it came from and
	 * at which commit, and offers the one way to edit it — a fork into the
	 * user's own storage (new document id; relative links rebased to the
	 * pinned commit).
	 */
	import { getDocumentLink, forkLinkedDocument } from '$lib/engine/store.svelte.js';
	import { describeLocator } from '$lib/storage/git/locator.js';

	let link = $derived(getDocumentLink());
	let busy = $state(false);

	async function fork() {
		busy = true;
		try {
			await forkLinkedDocument();
		} finally {
			busy = false;
		}
	}
</script>

{#if link}
	<div class="linked-banner" data-testid="linked-doc-banner">
		<span class="label">Linked · read-only</span>
		<span class="where" data-testid="linked-doc-locator" title={describeLocator(link.locator)}>
			{describeLocator(link.locator)}
		</span>
		{#if link.resolved?.commit}
			<span class="commit" data-testid="linked-doc-commit">commit {link.resolved.commit.slice(0, 7)}</span>
		{/if}
		<button class="fork-btn" data-testid="linked-doc-fork" onclick={fork} disabled={busy}>
			Fork to edit
		</button>
	</div>
{/if}

<style>
	.linked-banner {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 4px 12px;
		font-size: 12px;
		background: var(--bg-tertiary, #313244);
		border-bottom: 1px solid var(--border-color, #45475a);
		color: var(--text-secondary, #a6adc8);
	}
	.label {
		font-weight: 600;
		color: var(--warning, #f9e2af);
	}
	.where {
		font-family: monospace;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		flex: 1;
		min-width: 0;
	}
	.commit {
		font-family: monospace;
	}
	.fork-btn {
		padding: 2px 10px;
		border-radius: 4px;
		border: 1px solid var(--accent, #89b4fa);
		background: transparent;
		color: var(--accent, #89b4fa);
		cursor: pointer;
	}
	.fork-btn:disabled {
		opacity: 0.5;
	}
</style>
