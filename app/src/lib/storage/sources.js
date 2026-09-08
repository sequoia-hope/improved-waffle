/**
 * Source content resolution for the app (`specs/waffle_v4_document_model.md`
 * §2.3 resolution order, §2.4 pin/update semantics, Phase 2 P2-3): the engine
 * lists the document's `sources` with availability; for each one it lacks,
 * the host looks in the content cache by `content_hash`, else fetches through
 * the locator — always at the RECORDED commit when there is one, never at a
 * moving ref name ("update to tip" is an explicit action) — verifies the
 * hash, caches, and hands the bytes to the engine (`ProvideSource`).
 */

import { cacheGet, cachePut } from './git/cache.js';
import { checkContentHash, contentHashFromBlobSha, gitBlobSha1 } from './git/hash.js';
import { GitHostError, adapterFor, fetchUrlLocator } from './git/hosts.js';
import { locatorFromLegacySrc, resolveRelative } from './git/locator.js';
import { parseRemote } from './git/remote.js';
import { getHostToken } from './git/tokens.js';

/**
 * Fetch a Git locator's content at a known commit, or resolve its ref first.
 * @param {{remote:string, path:string, ref:any, host?:string}} locator
 * @param {string|null} knownCommit - `resolved.commit` when recorded
 * @returns {Promise<{text: string, commit: string, blobSha: string|null}>}
 */
export async function fetchGitAt(locator, knownCommit) {
	const origin = parseRemote(locator.remote)?.origin;
	const token = origin ? getHostToken(origin) : null;
	const adapter = adapterFor(locator);
	const commit = knownCommit
		? knownCommit.toLowerCase()
		: (await adapter.resolveRef(locator.remote, locator.ref, token)).commit;
	const { text, blobSha } = await adapter.fetchBlob(locator.remote, commit, locator.path, token);
	return { text, commit, blobSha };
}

/**
 * Resolve one source's content.
 * @param {{id:string, name:string, locator:any, content_hash?:string|null, resolved?:{commit:string}|null}} status
 * @param {any} docLocation - the document's own Git locator (for `Relative`), or null
 * @returns {Promise<{text: string, resolvedCommit: string|null, from: 'cache'|'git'|'url'}>}
 */
export async function resolveSourceContent(status, docLocation) {
	const hash = status.content_hash ?? null;
	if (hash) {
		const cached = await cacheGet(hash);
		if (cached != null) {
			return { text: cached, resolvedCommit: status.resolved?.commit ?? null, from: 'cache' };
		}
	}
	let locator = status.locator;
	if (locator?.type === 'Relative') {
		const abs = resolveRelative(docLocation, locator.path);
		if (!abs) {
			throw new GitHostError(
				`"${status.name}" is a relative link (${locator.path}) but this document has no git location`,
				'unresolvable'
			);
		}
		locator = abs;
	}
	if (locator?.type === 'Git') {
		const { text, commit, blobSha } = await fetchGitAt(locator, status.resolved?.commit ?? null);
		const actual = blobSha ? contentHashFromBlobSha(blobSha) : await gitBlobSha1(text);
		if (hash && (await checkContentHash(hash, text)) === 'mismatch') {
			// Fetched at a fixed commit: a different hash means the record and
			// the repository disagree — never silently take either (§4 inv. 4/5).
			throw new GitHostError(
				`"${status.name}" at ${commit.slice(0, 7)} does not match the recorded content hash`,
				'invalid'
			);
		}
		await cachePut(actual, text);
		return { text, resolvedCommit: commit, from: 'git' };
	}
	if (locator?.type === 'Url') {
		const { text } = await fetchUrlLocator(locator.url);
		await cachePut(await gitBlobSha1(text), text);
		return { text, resolvedCommit: null, from: 'url' };
	}
	if (locator?.type === 'Local') {
		throw new GitHostError(`"${status.name}" lives in another browser's storage (${locator.provider}); pack the document to share it`, 'unresolvable');
	}
	if (locator?.type === 'Embedded') {
		throw new GitHostError(`"${status.name}" is embedded but its content is missing from the file`, 'invalid');
	}
	throw new GitHostError(`"${status.name}": unsupported locator kind ${locator?.type ?? '?'}`, 'unresolvable');
}

/**
 * Turn a pasted link (a GitHub/GitLab/Gitea file URL, a raw URL, an `/open`
 * share link, or any https URL) into a locator for a linked STEP import.
 * @param {string} url
 * @returns {any|null}
 */
export function locatorForImportLink(url) {
	let u;
	try {
		u = new URL(String(url).trim());
	} catch {
		return null;
	}
	if (u.pathname.endsWith('/open') && u.searchParams.get('remote')) {
		// An app share link for a file in a repo.
		return {
			type: 'Git',
			remote: u.searchParams.get('remote'),
			path: u.searchParams.get('path'),
			ref: { type: 'Branch', name: u.searchParams.get('ref') || 'main' }
		};
	}
	return locatorFromLegacySrc(u.toString());
}
