/**
 * Saved git storage providers (`specs/waffle_v4_document_model.md` §7.1–7.3):
 * a repository the user writes documents to, on any supported host. Configs
 * live in localStorage; the host token lives in the per-host token store
 * (`git/tokens.js`), never in the config.
 */

import { inferHost, normalizeRemote, parseRemote } from './git/remote.js';

const LS_PROVIDERS = 'waffle-git-providers';

/**
 * @typedef {Object} GitProviderConfig
 * @property {string} id - provider id (`git:<host>/<owner>/<repo>`)
 * @property {'github'|'gitlab'|'gitea'|'generic'} kind
 * @property {string} remote - HTTPS clone URL (normalized)
 * @property {string} branch
 * @property {string} folder - repo-relative folder ('' = root), with trailing `/` when set
 * @property {string} label
 */

function storage() {
	return typeof localStorage === 'undefined' ? null : localStorage;
}

/** The provider id for a remote. @param {string} remote */
export function providerIdFor(remote) {
	const p = parseRemote(remote);
	return p ? `git:${p.host}/${p.repoPath}` : `git:${normalizeRemote(remote)}`;
}

/** Display label: `GitLab · group/repo`. */
export function providerLabelFor(kind, remote) {
	const p = parseRemote(remote);
	const names = { github: 'GitHub', gitlab: 'GitLab', gitea: 'Gitea', generic: 'Git' };
	return `${names[kind] ?? 'Git'} · ${p ? p.repoPath : remote}`;
}

/** Normalize a folder: '' or `a/b/`. @param {string} folder */
export function normalizeFolder(folder) {
	const f = String(folder ?? '').trim().replace(/^\/+|\/+$/g, '');
	return f ? `${f}/` : '';
}

/**
 * Build a config from user input.
 * @param {{remote: string, branch?: string, folder?: string, kind?: string}} input
 * @returns {GitProviderConfig}
 */
export function makeGitProviderConfig({ remote, branch = 'main', folder = '', kind }) {
	const normalized = normalizeRemote(remote);
	const k = kind && kind !== 'auto' ? kind : inferHost(normalized);
	return {
		id: providerIdFor(normalized),
		kind: k,
		remote: normalized,
		branch: (branch || 'main').trim(),
		folder: normalizeFolder(folder),
		label: providerLabelFor(k, normalized)
	};
}

/** @returns {GitProviderConfig[]} */
export function listGitProviderConfigs() {
	const ls = storage();
	if (!ls) return [];
	try {
		const raw = ls.getItem(LS_PROVIDERS);
		const list = raw ? JSON.parse(raw) : [];
		return Array.isArray(list) ? list : [];
	} catch {
		return [];
	}
}

/** @param {GitProviderConfig} cfg */
export function saveGitProviderConfig(cfg) {
	const ls = storage();
	if (!ls) return;
	const list = listGitProviderConfigs().filter((c) => c.id !== cfg.id);
	list.push(cfg);
	ls.setItem(LS_PROVIDERS, JSON.stringify(list));
}

/** @param {string} id */
export function removeGitProviderConfig(id) {
	const ls = storage();
	if (!ls) return;
	ls.setItem(LS_PROVIDERS, JSON.stringify(listGitProviderConfigs().filter((c) => c.id !== id)));
}
