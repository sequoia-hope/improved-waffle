<script>
	/**
	 * Banner for a Part open in the context of an assembly (v4 Phase 3d-4,
	 * in-context editing): names the assembly and the instance being edited,
	 * offers to re-take the context snapshot (the other instances may have
	 * moved or changed) and to leave the context. The other instances render
	 * as ghosts; a sketch started on one of their faces references it.
	 */
	import {
		getEditContext,
		getDocumentTabs,
		updateEditContext,
		exitEditContext
	} from '$lib/engine/store.svelte.js';

	let ctx = $derived(getEditContext());
	let assemblyName = $derived(getDocumentTabs().find((t) => t.id === ctx?.assembly_tab_id)?.name ?? ctx?.assembly_tab_id ?? '');
	let busy = $state(false);

	async function run(fn) {
		busy = true;
		try {
			await fn();
		} finally {
			busy = false;
		}
	}
</script>

{#if ctx}
	<div class="context-banner" data-testid="context-banner">
		<span class="label">In context</span>
		<span class="where" data-testid="context-banner-where">
			editing <strong data-testid="context-banner-instance">{ctx.instance_name}</strong> in <strong data-testid="context-banner-assembly">{assemblyName}</strong>
			· {ctx.instances.length} ghost {ctx.instances.length === 1 ? 'instance' : 'instances'}
		</span>
		{#if ctx.errors.length > 0}
			<span class="problems" data-testid="context-banner-errors" title={ctx.errors.join('\n')}>{ctx.errors.length} error{ctx.errors.length === 1 ? '' : 's'}</span>
		{/if}
		<button class="btn" data-testid="context-banner-update" onclick={() => run(updateEditContext)} disabled={busy} title="Re-take the snapshot of the other instances">
			Update context
		</button>
		<button class="btn" data-testid="context-banner-exit" onclick={() => run(exitEditContext)} disabled={busy} title="Keep editing this part on its own">
			Exit context
		</button>
	</div>
{/if}

<style>
	.context-banner {
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
		color: var(--accent, #89b4fa);
	}
	.where {
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.where strong {
		color: var(--text-primary, #cdd6f4);
		font-weight: 600;
	}
	.problems {
		color: var(--error, #f38ba8);
	}
	.btn {
		background: var(--bg-secondary, #1e1e2e);
		color: var(--text-primary, #cdd6f4);
		border: 1px solid var(--border-color, #45475a);
		border-radius: 4px;
		padding: 2px 10px;
		font-size: 12px;
		cursor: pointer;
	}
	.btn:hover:not(:disabled) {
		border-color: var(--accent, #89b4fa);
	}
	.btn:disabled {
		opacity: 0.5;
		cursor: default;
	}
</style>
