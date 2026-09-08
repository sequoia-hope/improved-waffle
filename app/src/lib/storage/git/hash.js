/**
 * Content hashing for `SourceEntry.content_hash` — the JS mirror of
 * `file_format::git_blob_sha1`: SHA-1 of `"blob " + byte_len + "\0" + bytes`,
 * i.e. the git blob id, which GitHub/GitLab/Gitea report for free.
 */

const PREFIX = 'git-blob-sha1:';

/**
 * `git-blob-sha1:<40 hex>` of a UTF-8 text.
 * @param {string} text
 * @returns {Promise<string>}
 */
export async function gitBlobSha1(text) {
	const body = new TextEncoder().encode(text);
	const header = new TextEncoder().encode(`blob ${body.byteLength}\0`);
	const buf = new Uint8Array(header.byteLength + body.byteLength);
	buf.set(header, 0);
	buf.set(body, header.byteLength);
	const digest = await crypto.subtle.digest('SHA-1', buf);
	const hex = [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, '0')).join('');
	return PREFIX + hex;
}

/**
 * Compare a recorded `content_hash` with the hash of `text`. An unknown
 * algorithm prefix (or no hash) is "no hash", never a mismatch (§4 inv. 9).
 * @param {string|null|undefined} recorded
 * @param {string} text
 * @returns {Promise<'match'|'mismatch'|'none'>}
 */
export async function checkContentHash(recorded, text) {
	if (!recorded || !recorded.startsWith(PREFIX)) return 'none';
	const actual = await gitBlobSha1(text);
	return actual === recorded ? 'match' : 'mismatch';
}

/**
 * The bare hex of a `git-blob-sha1:` hash (what hosts report as a blob sha),
 * or null for any other algorithm.
 * @param {string|null|undefined} contentHash
 */
export function blobShaOf(contentHash) {
	if (!contentHash || !contentHash.startsWith(PREFIX)) return null;
	return contentHash.slice(PREFIX.length);
}

/** Wrap a host-reported blob sha as a `content_hash`. @param {string} sha */
export function contentHashFromBlobSha(sha) {
	return PREFIX + sha.toLowerCase();
}
