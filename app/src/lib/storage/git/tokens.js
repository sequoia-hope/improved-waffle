/**
 * Per-host access tokens (`specs/waffle_v4_document_model.md` §7.2): one
 * entry per host origin, replacing the single GitHub token. The legacy
 * `waffle-github-token` written by `github-auth.js` is still honored for
 * `https://github.com` so existing sign-ins keep working.
 */

const LS_HOSTS = 'waffle-host-tokens';
const LS_LEGACY_GITHUB_TOKEN = 'waffle-github-token';
const LS_LEGACY_GITHUB_USER = 'waffle-github-user';

/** @typedef {{host_url: string, token: string, login?: string, kind?: string}} HostToken */

function storage() {
	return typeof localStorage === 'undefined' ? null : localStorage;
}

/** Normalize a host origin (`https://gitlab.com`). @param {string} hostUrl */
export function normalizeHostUrl(hostUrl) {
	const m = /^(https?):\/\/([^/]+)/i.exec(String(hostUrl ?? '').trim());
	return m ? `${m[1].toLowerCase()}://${m[2].toLowerCase()}` : String(hostUrl ?? '').trim().toLowerCase();
}

/** @returns {HostToken[]} */
export function listHostTokens() {
	const ls = storage();
	if (!ls) return [];
	/** @type {HostToken[]} */
	let entries = [];
	try {
		const raw = ls.getItem(LS_HOSTS);
		entries = raw ? JSON.parse(raw) : [];
		if (!Array.isArray(entries)) entries = [];
	} catch {
		entries = [];
	}
	const legacy = ls.getItem(LS_LEGACY_GITHUB_TOKEN);
	if (legacy && !entries.some((e) => e.host_url === 'https://github.com')) {
		entries = [
			...entries,
			{
				host_url: 'https://github.com',
				token: legacy,
				login: ls.getItem(LS_LEGACY_GITHUB_USER) || undefined,
				kind: 'github'
			}
		];
	}
	return entries;
}

/**
 * The token for a host origin, or null.
 * @param {string} hostUrl
 * @returns {string|null}
 */
export function getHostToken(hostUrl) {
	const want = normalizeHostUrl(hostUrl);
	return listHostTokens().find((e) => normalizeHostUrl(e.host_url) === want)?.token ?? null;
}

/**
 * Store (or replace) a host's token.
 * @param {string} hostUrl
 * @param {string} token
 * @param {{login?: string, kind?: string}} [meta]
 */
export function setHostToken(hostUrl, token, meta = {}) {
	const ls = storage();
	if (!ls) return;
	const want = normalizeHostUrl(hostUrl);
	const entries = listHostTokens().filter((e) => normalizeHostUrl(e.host_url) !== want);
	entries.push({ host_url: want, token, ...meta });
	ls.setItem(LS_HOSTS, JSON.stringify(entries));
}

/** Forget a host's token. @param {string} hostUrl */
export function removeHostToken(hostUrl) {
	const ls = storage();
	if (!ls) return;
	const want = normalizeHostUrl(hostUrl);
	const entries = listHostTokens().filter((e) => normalizeHostUrl(e.host_url) !== want);
	ls.setItem(LS_HOSTS, JSON.stringify(entries));
	if (want === 'https://github.com') {
		ls.removeItem(LS_LEGACY_GITHUB_TOKEN);
	}
}
