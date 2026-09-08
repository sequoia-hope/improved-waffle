/**
 * Git remote parsing — the JS mirror of the Rust locator rules
 * (`crates/file-format/src/sources.rs`, `specs/waffle_v4_document_model.md`
 * §2.4): a `Git` locator's `remote` is an HTTPS clone URL, normalized without a
 * trailing `.git`, and `host` names the API adapter (§7.2).
 */

/**
 * Normalize an HTTPS clone URL: lowercase scheme+host, drop a trailing `.git`
 * and trailing slashes. Readers accept either form; writers emit this one.
 * @param {string} remote
 * @returns {string}
 */
export function normalizeRemote(remote) {
	let s = String(remote ?? '').trim();
	s = s.replace(/\/+$/, '');
	if (s.toLowerCase().endsWith('.git')) s = s.slice(0, -4);
	s = s.replace(/\/+$/, '');
	const m = /^(https?):\/\/([^/]+)(\/.*)?$/i.exec(s);
	if (!m) return s;
	return `${m[1].toLowerCase()}://${m[2].toLowerCase()}${m[3] ?? ''}`;
}

/**
 * Split a normalized remote into its host origin and repository path.
 * `https://github.com/acme/parts` → `{ origin: 'https://github.com', host: 'github.com', repoPath: 'acme/parts', owner: 'acme', repo: 'parts' }`.
 * GitLab subgroups keep the full path (`group/sub/repo`); `owner` is then
 * everything before the last segment.
 * @param {string} remote
 * @returns {{origin: string, host: string, repoPath: string, owner: string, repo: string} | null}
 */
export function parseRemote(remote) {
	const n = normalizeRemote(remote);
	const m = /^https:\/\/([^/]+)\/(.+)$/i.exec(n);
	if (!m) return null;
	const host = m[1];
	const repoPath = m[2].replace(/^\/+|\/+$/g, '');
	const segs = repoPath.split('/').filter(Boolean);
	if (segs.length < 2) return null;
	if (segs.some((s) => s === '..' || s === '.')) return null;
	return {
		origin: `https://${host}`,
		host,
		repoPath: segs.join('/'),
		owner: segs.slice(0, -1).join('/'),
		repo: segs[segs.length - 1]
	};
}

/**
 * Which API adapter serves a remote. Mirrors `GitHost::infer` in Rust; an
 * explicit `host` on the locator always wins over this inference.
 * @param {string} remote
 * @returns {'github'|'gitlab'|'gitea'|'generic'}
 */
export function inferHost(remote) {
	const parsed = parseRemote(remote);
	const host = (parsed?.host ?? '').toLowerCase();
	if (host === 'github.com' || host.endsWith('.github.com')) return 'github';
	if (host === 'gitlab.com' || host.includes('gitlab')) return 'gitlab';
	if (host.includes('gitea') || host.includes('forgejo') || host === 'codeberg.org') return 'gitea';
	return 'generic';
}

/**
 * True when the string is a git object id (40-hex SHA-1, or 64-hex SHA-256).
 * @param {string} s
 */
export function isCommitSha(s) {
	return /^[0-9a-f]{40}$/i.test(s) || /^[0-9a-f]{64}$/i.test(s);
}

/**
 * Validate a git ref name the way the Rust loader does (§6): no `..`, no
 * leading `-`, no control characters, no whitespace.
 * @param {string} name
 */
export function isValidRefName(name) {
	if (typeof name !== 'string' || name.length === 0) return false;
	if (name.startsWith('-') || name.includes('..')) return false;
	// eslint-disable-next-line no-control-regex
	if (/[\x00-\x1f\x7f\s~^:?*[\\]/.test(name)) return false;
	return true;
}

/**
 * Validate a repository-relative path (§6): `/`-separated, no leading `/`,
 * no `..` or empty segments.
 * @param {string} path
 */
export function isValidRepoPath(path) {
	if (typeof path !== 'string' || path.length === 0 || path.startsWith('/')) return false;
	const segs = path.split('/');
	return segs.every((s) => s.length > 0 && s !== '..' && s !== '.');
}
