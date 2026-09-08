/**
 * Git-backed document storage on any supported host
 * (`specs/waffle_v4_document_model.md` §7.1–7.4): one `.waffle` file per
 * document in a repository folder, plus an index file `.waffle-index.json`
 * mapping document id → path. Reads and writes go through the host adapters'
 * write-back API (`git/hosts.js`); the token comes from the per-host token
 * store. A document stored here has a git LOCATION
 * (`{remote, path, ref: Branch(branch)}`), which is what makes it linkable
 * (`getShareUrl` → the `/open` locator link) and what its `Relative` links
 * resolve against.
 *
 * @implements {import('./types.js').DocumentStore}
 */

import { base } from '$app/paths';
import { GitHostError, adapterFor } from './git/hosts.js';
import { buildOpenLink } from './git/locator.js';
import { parseRemote } from './git/remote.js';
import { getHostToken } from './git/tokens.js';

const INDEX_FILE = '.waffle-index.json';

export class GitProvider {
	/** @type {string} */
	id;
	/** @type {string} */
	label;
	canShare = true;

	#cfg;
	#token;
	#adapter;
	/** @type {Array|null} */
	#indexCache = null;

	/**
	 * @param {import('./providers.js').GitProviderConfig} cfg
	 * @param {string|null} [token] - explicit token; else the host token store
	 */
	constructor(cfg, token = null) {
		this.#cfg = { ...cfg, folder: cfg.folder ?? '' };
		this.id = cfg.id;
		this.label = cfg.label;
		this.#token = token;
		this.#adapter = adapterFor({ remote: cfg.remote, host: cfg.kind });
	}

	/** @returns {import('./providers.js').GitProviderConfig} */
	get config() {
		return { ...this.#cfg };
	}

	get kind() {
		return this.#cfg.kind;
	}

	get remote() {
		return this.#cfg.remote;
	}

	get branch() {
		return this.#cfg.branch;
	}

	get token() {
		if (this.#token) return this.#token;
		const origin = parseRemote(this.#cfg.remote)?.origin;
		return origin ? getHostToken(origin) : null;
	}

	/** Throw unless the repository exists and the token can see it. */
	async ensureRepo() {
		const exists = await this.#adapter.repoExists(this.#cfg.remote, this.token);
		if (!exists) {
			throw new GitHostError(
				`${this.#cfg.remote}: repository not found — create it on the host first (or check the token)`,
				'not_found'
			);
		}
	}

	#path(name) {
		return `${this.#cfg.folder}${name}`;
	}

	/**
	 * @returns {Promise<import('./types.js').DocumentSummary[]>}
	 */
	async list() {
		const index = await this.#loadIndex();
		return index.map((entry) => ({
			id: entry.id,
			name: entry.name,
			created: entry.created,
			modified: entry.modified,
			displayUnit: entry.displayUnit || null,
			tabCount: entry.tabCount || 1,
			provider: this.id,
			previewMesh: entry.previewMesh || null,
			link: null
		}));
	}

	/**
	 * @param {string} docId
	 * @returns {Promise<import('./types.js').StoredDocument|null>}
	 */
	async get(docId) {
		const index = await this.#loadIndex();
		const entry = index.find((e) => e.id === docId);
		if (!entry) return null;
		const file = await this.#adapter.getFile(this.#cfg.remote, this.#cfg.branch, this.#path(entry.filename), this.token);
		if (!file) return null;
		return { id: entry.id, json: file.text, created: entry.created, modified: entry.modified };
	}

	/**
	 * @param {import('./types.js').StoredDocument} doc
	 */
	async put(doc) {
		const parsed = JSON.parse(doc.json);
		const name = parsed.document?.name || parsed.project?.name || parsed.name || 'Untitled';
		const filename = buildSlug(name) + '.waffle';

		const index = await this.#loadIndex();
		const existing = index.find((e) => e.id === doc.id);

		let existingSha = null;
		if (existing && existing.filename !== filename) {
			// Renamed: remove the old file, then write the new name.
			const old = await this.#adapter.getFile(this.#cfg.remote, this.#cfg.branch, this.#path(existing.filename), this.token);
			if (old) {
				await this.#adapter.deleteFile(this.#cfg.remote, this.#cfg.branch, this.#path(existing.filename), `Rename ${existing.filename} to ${filename}`, this.token, old.sha);
			}
		}
		const current = await this.#adapter.getFile(this.#cfg.remote, this.#cfg.branch, this.#path(filename), this.token);
		if (current) existingSha = current.sha;
		await this.#adapter.putFile(this.#cfg.remote, this.#cfg.branch, this.#path(filename), doc.json, `Save ${name}`, this.token, existingSha);

		const entry = {
			id: doc.id,
			name,
			filename,
			created: existing ? existing.created : doc.created,
			modified: doc.modified,
			displayUnit: parsed.document?.display_unit || parsed.displayUnit || null,
			tabCount: parsed.tabs ? parsed.tabs.length : 1,
			previewMesh: null
		};
		if (existing) index[index.indexOf(existing)] = entry;
		else index.push(entry);
		await this.#saveIndex(index);
	}

	/** @param {string} docId */
	async delete(docId) {
		const index = await this.#loadIndex();
		const entry = index.find((e) => e.id === docId);
		if (!entry) return;
		const file = await this.#adapter.getFile(this.#cfg.remote, this.#cfg.branch, this.#path(entry.filename), this.token);
		if (file) {
			await this.#adapter.deleteFile(this.#cfg.remote, this.#cfg.branch, this.#path(entry.filename), `Delete ${entry.name}`, this.token, file.sha);
		}
		await this.#saveIndex(index.filter((e) => e.id !== docId));
	}

	/** The document's git locator (its location, §7.1), or null. @param {string} docId */
	async getLocator(docId) {
		const index = await this.#loadIndex();
		const entry = index.find((e) => e.id === docId);
		if (!entry) return null;
		return {
			type: 'Git',
			remote: this.#cfg.remote,
			path: this.#path(entry.filename),
			ref: { type: 'Branch', name: this.#cfg.branch },
			host: this.#cfg.kind
		};
	}

	/**
	 * A share link is a locator (§7.4): this repo, this file, the branch tip.
	 * @param {string} docId
	 */
	async getShareUrl(docId) {
		const loc = await this.getLocator(docId);
		if (!loc) return null;
		return buildOpenLink(loc, `${window.location.origin}${base}`);
	}

	async #loadIndex() {
		if (this.#indexCache) return this.#indexCache;
		const file = await this.#adapter.getFile(this.#cfg.remote, this.#cfg.branch, this.#path(INDEX_FILE), this.token);
		if (!file) {
			this.#indexCache = [];
			return this.#indexCache;
		}
		try {
			const parsed = JSON.parse(file.text);
			this.#indexCache = Array.isArray(parsed) ? parsed : Array.isArray(parsed?.documents) ? parsed.documents : [];
		} catch {
			this.#indexCache = [];
		}
		return this.#indexCache;
	}

	async #saveIndex(index) {
		this.#indexCache = index;
		const current = await this.#adapter.getFile(this.#cfg.remote, this.#cfg.branch, this.#path(INDEX_FILE), this.token);
		await this.#adapter.putFile(
			this.#cfg.remote,
			this.#cfg.branch,
			this.#path(INDEX_FILE),
			JSON.stringify(index, null, 2),
			'Update document index',
			this.token,
			current?.sha ?? null
		);
	}

	/** Drop the cached index (e.g. after an external change). */
	invalidate() {
		this.#indexCache = null;
	}
}

/**
 * Build a URL-safe slug from a document name.
 * @param {string} name
 */
export function buildSlug(name) {
	return (
		String(name)
			.toLowerCase()
			.replace(/[^a-z0-9]+/g, '-')
			.replace(/^-+|-+$/g, '') || 'untitled'
	);
}
