<script>
	import { goto } from '$app/navigation';
	import { base } from '$app/paths';
	import { page } from '$app/stores';
	import { getStore } from '$lib/storage/index.js';
	import { onMount } from 'svelte';

	const id = $derived($page.params.id);

	onMount(async () => {
		const store = getStore();
		const doc = await store.get(id);
		if (doc) {
			sessionStorage.setItem('waffle-active-doc', doc.id);
			sessionStorage.setItem('waffle-active-json', doc.json);
			// A linked (share-link) record stays read-only when reopened.
			if (doc.link) sessionStorage.setItem('waffle-active-link', JSON.stringify(doc.link));
			else sessionStorage.removeItem('waffle-active-link');
		}
		goto(`${base}/`, { replaceState: true });
	});
</script>

<div class="loading-page">
	<p>Loading document...</p>
</div>

<style>
	.loading-page {
		height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--text-secondary, #a6adc8);
		background: var(--bg-primary, #1e1e2e);
	}
</style>
