/**
 * Locator helpers (`specs/waffle_v4_document_model.md` §2.4, §7.4): the app's
 * share link IS a locator, `Relative` links resolve against the document's own
 * location, and the legacy `?src=<raw url>` share link (emitted by the GitHub
 * provider, never handled until Phase 2) maps onto a `Git` locator when the
 * raw URL is a known host's raw-file shape.
 */

import { inferHost, isCommitSha, isValidRefName, isValidRepoPath, normalizeRemote } from './remote.js';

/**
 * Classify a ref string as a `GitRef`: a 40/64-hex sha ⇒ `Commit`; a
 * `refs/tags/x` or `tags/x` ⇒ `Tag`; `refs/heads/x` or anything else ⇒
 * `Branch`. (A bare name cannot be told branch from tag without the host;
 * branch is the floating default and hosts resolve either.)
 * @param {string} ref
 * @returns {{type:'Commit', sha:string}|{type:'Branch', name:string}|{type:'Tag', name:string}|null}
 */
export function parseRef(ref) {
	const s = String(ref ?? '').trim();
	if (!s) return null;
	if (isCommitSha(s)) return { type: 'Commit', sha: s.toLowerCase() };
	let m = /^refs\/tags\/(.+)$/.exec(s) || /^tags\/(.+)$/.exec(s);
	if (m) return isValidRefName(m[1]) ? { type: 'Tag', name: m[1] } : null;
	m = /^refs\/heads\/(.+)$/.exec(s);
	const name = m ? m[1] : s;
	return isValidRefName(name) ? { type: 'Branch', name } : null;
}

/** The URL form of a GitRef. @param {any} ref */
export function refToParam(ref) {
	if (!ref) return '';
	if (ref.type === 'Commit') return ref.sha;
	if (ref.type === 'Tag') return `refs/tags/${ref.name}`;
	return ref.name;
}

/**
 * The app's share link for a Git locator (§7.4):
 * `${origin}${base}/open?remote=…&path=…&ref=…`.
 * @param {{remote:string, path:string, ref:any}} locator
 * @param {string} appOrigin - e.g. `window.location.origin + base`
 */
export function buildOpenLink(locator, appOrigin) {
	const params = new URLSearchParams();
	params.set('remote', normalizeRemote(locator.remote));
	params.set('path', locator.path);
	params.set('ref', refToParam(locator.ref));
	return `${appOrigin.replace(/\/+$/, '')}/open?${params.toString()}`;
}

/**
 * Parse `/open` search params into a locator, or return `{error}`.
 * Accepts `remote`+`path`+`ref` (Git), `url` (Url), or the legacy `src`.
 * @param {URLSearchParams} params
 * @returns {{locator: any} | {error: string}}
 */
export function parseOpenParams(params) {
	const src = params.get('src');
	if (src) {
		const loc = locatorFromLegacySrc(src);
		return loc ? { locator: loc } : { error: `cannot interpret share link ${src}` };
	}
	const url = params.get('url');
	if (url) {
		if (!/^https:\/\//i.test(url)) return { error: 'url must be https' };
		return { locator: { type: 'Url', url } };
	}
	const remote = params.get('remote');
	const path = params.get('path');
	const refStr = params.get('ref') || 'main';
	if (!remote || !path) return { error: 'share link needs remote and path' };
	const normalized = normalizeRemote(remote);
	if (!/^https:\/\//i.test(normalized)) return { error: 'remote must be an https clone URL' };
	if (!isValidRepoPath(path)) return { error: `invalid path ${path}` };
	const ref = parseRef(refStr);
	if (!ref) return { error: `invalid ref ${refStr}` };
	const host = params.get('host');
	const locator = { type: 'Git', remote: normalized, path, ref };
	if (host && ['github', 'gitlab', 'gitea', 'generic'].includes(host)) locator.host = host;
	return { locator };
}

/**
 * Map a raw-file URL onto a locator:
 *  - `https://raw.githubusercontent.com/{o}/{r}/{ref}/{path}` → Git (github)
 *  - `https://github.com/{o}/{r}/(blob|raw)/{ref}/{path}` → Git (github)
 *  - `https://{gitlab}/{group..}/{repo}/-/(raw|blob)/{ref}/{path}` → Git (gitlab)
 *  - `https://{gitea}/{o}/{r}/(raw|src)/(branch|tag|commit)/{ref}/{path}` → Git (gitea)
 *  - any other https URL → Url
 * A ref taken from a URL is one path segment; a branch name containing `/`
 * cannot be recovered from a raw URL and yields the first segment.
 * @param {string} src
 */
export function locatorFromLegacySrc(src) {
	let u;
	try {
		u = new URL(src);
	} catch {
		return null;
	}
	if (u.protocol !== 'https:') return null;
	const segs = u.pathname.split('/').filter(Boolean).map(decodeURIComponent);
	const git = (remote, ref, pathSegs, host) => {
		const path = pathSegs.join('/');
		const parsedRef = parseRef(ref);
		if (!parsedRef || !isValidRepoPath(path)) return null;
		return { type: 'Git', remote: normalizeRemote(remote), path, ref: parsedRef, host };
	};
	if (u.hostname === 'raw.githubusercontent.com' && segs.length >= 4) {
		return git(`https://github.com/${segs[0]}/${segs[1]}`, segs[2], segs.slice(3), 'github');
	}
	if (u.hostname === 'github.com' && segs.length >= 5 && (segs[2] === 'blob' || segs[2] === 'raw')) {
		return git(`https://github.com/${segs[0]}/${segs[1]}`, segs[3], segs.slice(4), 'github');
	}
	const dash = segs.indexOf('-');
	if (dash >= 2 && segs.length >= dash + 4 && (segs[dash + 1] === 'raw' || segs[dash + 1] === 'blob')) {
		const remote = `${u.origin}/${segs.slice(0, dash).join('/')}`;
		return git(remote, segs[dash + 2], segs.slice(dash + 3), 'gitlab');
	}
	if (
		segs.length >= 6 &&
		(segs[2] === 'raw' || segs[2] === 'src') &&
		['branch', 'tag', 'commit'].includes(segs[3]) &&
		inferHost(`${u.origin}/${segs[0]}/${segs[1]}`) === 'gitea'
	) {
		const ref = segs[3] === 'tag' ? `refs/tags/${segs[4]}` : segs[4];
		return git(`${u.origin}/${segs[0]}/${segs[1]}`, ref, segs.slice(5), 'gitea');
	}
	return { type: 'Url', url: u.toString() };
}

/**
 * POSIX-normalize `dirname(basePath)/rel`; null if it escapes the repo root.
 * @param {string} basePath - the document's own repo path
 * @param {string} rel - a `Relative` locator path (may contain `..`)
 */
export function joinRepoPath(basePath, rel) {
	const dir = basePath.split('/').slice(0, -1);
	const out = [...dir];
	for (const seg of rel.split('/')) {
		if (seg === '' || seg === '.') continue;
		if (seg === '..') {
			if (out.length === 0) return null;
			out.pop();
		} else {
			out.push(seg);
		}
	}
	return out.length ? out.join('/') : null;
}

/**
 * Resolve a `Relative` locator against the document's location (§2.4): the
 * same remote and the SAME ref, so a clone, a fork, or a branch keeps its
 * intra-repo links. Returns null when the document has no git location.
 * @param {{type:'Git', remote:string, path:string, ref:any, host?:string}|null|undefined} docLocation
 * @param {string} relPath
 */
export function resolveRelative(docLocation, relPath) {
	if (!docLocation || docLocation.type !== 'Git') return null;
	const path = joinRepoPath(docLocation.path, relPath);
	if (!path) return null;
	const out = { type: 'Git', remote: docLocation.remote, path, ref: docLocation.ref };
	if (docLocation.host) out.host = docLocation.host;
	return out;
}

/** Short human label for a locator (banners, source lists). @param {any} loc */
export function describeLocator(loc) {
	if (!loc) return 'unknown';
	switch (loc.type) {
		case 'Git': {
			const r = loc.ref;
			const refLabel = r?.type === 'Commit' ? r.sha.slice(0, 7) : r?.type === 'Tag' ? `tag ${r.name}` : r?.name ?? '?';
			return `${loc.remote.replace(/^https:\/\//, '')}/${loc.path} @ ${refLabel}`;
		}
		case 'Url':
			return loc.url;
		case 'Relative':
			return `./${loc.path}`;
		case 'Local':
			return `${loc.provider}:${loc.doc_id}`;
		case 'Embedded':
			return 'embedded';
		default:
			return `${loc.type ?? '?'} (unsupported)`;
	}
}
