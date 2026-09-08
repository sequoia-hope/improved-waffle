<script>
	/**
	 * Connect a git repository on GitLab, Gitea/Forgejo (or GitHub via a
	 * personal access token) as a document storage provider
	 * (specs/waffle_v4_document_model.md §7.1–7.3). The token is stored per
	 * host (`git/tokens.js`); the repository must already exist.
	 */
	import { GitProvider } from '$lib/storage/git-provider.js';
	import { makeGitProviderConfig, saveGitProviderConfig } from '$lib/storage/providers.js';
	import { inferHost, parseRemote } from '$lib/storage/git/remote.js';
	import { getHostToken, setHostToken } from '$lib/storage/git/tokens.js';
	import { registerProvider, setActiveProvider } from '$lib/storage/index.js';

	let { visible = false, onclose, onconnect } = $props();

	let remote = $state('');
	let branch = $state('main');
	let folder = $state('');
	let token = $state('');
	let error = $state('');
	let busy = $state(false);

	let parsed = $derived(parseRemote(remote));
	let kind = $derived(parsed ? inferHost(remote) : null);
	let hasStoredToken = $derived(parsed ? !!getHostToken(parsed.origin) : false);

	$effect(() => {
		if (!visible) {
			remote = '';
			branch = 'main';
			folder = '';
			token = '';
			error = '';
			busy = false;
		}
	});

	async function connect(e) {
		e?.preventDefault?.();
		error = '';
		if (!parsed) {
			error = 'Enter the repository\'s HTTPS clone URL (https://host/owner/repo)';
			return;
		}
		busy = true;
		try {
			const cfg = makeGitProviderConfig({ remote, branch, folder });
			if (token.trim()) setHostToken(parsed.origin, token.trim(), { kind: cfg.kind });
			const provider = new GitProvider(cfg);
			await provider.ensureRepo();
			saveGitProviderConfig(cfg);
			registerProvider(provider);
			setActiveProvider(cfg.id);
			onconnect?.(cfg);
			onclose?.();
		} catch (err) {
			error = err?.message || String(err);
		} finally {
			busy = false;
		}
	}

	function handleKeydown(e) {
		if (e.key === 'Escape') {
			e.preventDefault();
			onclose?.();
		}
	}
</script>

{#if visible}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="overlay" onkeydown={handleKeydown} onclick={(e) => { if (e.target === e.currentTarget) onclose?.(); }}>
		<form class="dialog" data-testid="git-connect-dialog" onsubmit={connect}>
			<h2>Connect a git repository</h2>
			<p class="hint">
				Documents are saved as <code>.waffle</code> files in the repository (with an index file).
				GitLab, Gitea/Forgejo and GitHub are supported; the repository must already exist.
			</p>
			<label for="git-remote">Repository URL</label>
			<input id="git-remote" data-testid="git-connect-remote" type="url" bind:value={remote} placeholder="https://gitlab.com/group/parts" autocomplete="off" />
			{#if parsed}
				<p class="detect" data-testid="git-connect-kind">Host: {kind} · {parsed.repoPath}</p>
			{/if}
			<div class="row">
				<div>
					<label for="git-branch">Branch</label>
					<input id="git-branch" data-testid="git-connect-branch" type="text" bind:value={branch} />
				</div>
				<div>
					<label for="git-folder">Folder (optional)</label>
					<input id="git-folder" data-testid="git-connect-folder" type="text" bind:value={folder} placeholder="cad/" />
				</div>
			</div>
			<label for="git-token">Personal access token {hasStoredToken ? '(stored for this host — leave blank to keep)' : '(write access)'}</label>
			<input id="git-token" data-testid="git-connect-token" type="password" bind:value={token} autocomplete="off" />
			{#if error}
				<p class="error" data-testid="git-connect-error">{error}</p>
			{/if}
			<div class="actions">
				<button type="button" class="btn" data-testid="git-connect-close" onclick={() => onclose?.()} disabled={busy}>Cancel</button>
				<button type="submit" class="btn btn-primary" data-testid="git-connect-btn" disabled={busy || !parsed || (!token.trim() && !hasStoredToken)}>
					{busy ? 'Connecting…' : 'Connect'}
				</button>
			</div>
		</form>
	</div>
{/if}

<style>
	.overlay {
		position: fixed;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		background: rgba(0, 0, 0, 0.5);
		z-index: 300;
	}
	.dialog {
		width: min(560px, 92vw);
		padding: 24px;
		border-radius: 8px;
		border: 1px solid var(--border-color, #45475a);
		background: var(--bg-secondary, #181825);
		color: var(--text-primary, #cdd6f4);
	}
	h2 {
		margin: 0 0 8px;
		font-size: 16px;
	}
	.hint,
	.detect {
		font-size: 12px;
		color: var(--text-secondary, #a6adc8);
		margin: 0 0 10px;
	}
	label {
		display: block;
		font-size: 12px;
		margin: 10px 0 4px;
	}
	input {
		width: 100%;
		box-sizing: border-box;
		padding: 8px;
		border-radius: 4px;
		border: 1px solid var(--border-color, #45475a);
		background: var(--bg-primary, #1e1e2e);
		color: inherit;
	}
	.row {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 12px;
	}
	.error {
		color: var(--error, #f38ba8);
		font-size: 12px;
	}
	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
		margin-top: 16px;
	}
	.btn {
		padding: 8px 14px;
		border-radius: 4px;
		border: 1px solid var(--border-color, #45475a);
		background: transparent;
		color: inherit;
		cursor: pointer;
	}
	.btn-primary {
		background: var(--accent, #89b4fa);
		color: var(--bg-primary, #1e1e2e);
		border-color: transparent;
	}
	.btn:disabled {
		opacity: 0.5;
		cursor: default;
	}
</style>
