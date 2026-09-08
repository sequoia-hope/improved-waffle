import { generateDocId } from './types.js';

const DB_NAME = 'waffle-iron';
const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const DB_VERSION = 1;
const STORE_NAME = 'documents';

/**
 * Open (or create) the IndexedDB database.
 * @returns {Promise<IDBDatabase>}
 */
function openDB() {
	return new Promise((resolve, reject) => {
		const request = indexedDB.open(DB_NAME, DB_VERSION);
		request.onupgradeneeded = (event) => {
			const db = event.target.result;
			if (!db.objectStoreNames.contains(STORE_NAME)) {
				const store = db.createObjectStore(STORE_NAME, { keyPath: 'id' });
				store.createIndex('modified', 'modified', { unique: false });
			}
		};
		request.onsuccess = () => resolve(request.result);
		request.onerror = () => reject(request.error);
	});
}

/**
 * @implements {import('./types.js').DocumentStore}
 */
export class IndexedDBStore {
	id = 'local';
	label = 'This Browser';
	canShare = false;

	/** @type {Promise<IDBDatabase>|null} */
	#dbPromise = null;

	/** @returns {Promise<IDBDatabase>} */
	#getDB() {
		if (!this.#dbPromise) {
			this.#dbPromise = openDB();
		}
		return this.#dbPromise;
	}

	/**
	 * List all documents, sorted by modified desc.
	 * @returns {Promise<import('./types.js').DocumentSummary[]>}
	 */
	async list() {
		const db = await this.#getDB();
		return new Promise((resolve, reject) => {
			const tx = db.transaction(STORE_NAME, 'readonly');
			const store = tx.objectStore(STORE_NAME);
			const request = store.getAll();
			request.onsuccess = () => {
				const docs = request.result.map((doc) => {
					let name = 'Untitled';
					let tabCount = 1;
					let displayUnit = null;
					let previewMesh = null;
					try {
						const parsed = JSON.parse(doc.json);
						if (parsed.document) {
							name = parsed.document.name || name;
							displayUnit = parsed.document.display_unit || null;
						} else if (parsed.project) {
							name = parsed.project.name || name;
							displayUnit = parsed.project.display_unit || null;
						}
						if (parsed.tabs) {
							tabCount = parsed.tabs.length;
							// Extract preview mesh from first tab with one
							for (const tab of parsed.tabs) {
								if (tab.kind?.preview_mesh) {
									previewMesh = tab.kind.preview_mesh;
									break;
								}
							}
						}
					} catch {
						// ignore parse errors for listing
					}
					return {
						id: doc.id,
						name,
						created: doc.created,
						modified: doc.modified,
						displayUnit,
						tabCount,
						previewMesh,
						provider: 'local',
						// Share-link provenance (open-link.js); null for the user's own docs.
						link: doc.link ?? null
					};
				});
				// Sort by modified descending
				docs.sort((a, b) => b.modified - a.modified);
				resolve(docs);
			};
			request.onerror = () => reject(request.error);
		});
	}

	/**
	 * Get a document by its record key, or — since v4 Phase 2 P2-5 — by its
	 * `document.id`: records created since then are keyed by the document's
	 * own identity, and older 8-char-keyed records are still found by the
	 * identity inside their JSON (no re-keying migration; a linked read-only
	 * copy of someone else's document is only returned when no record of the
	 * user's own carries that identity).
	 * @param {string} id
	 * @returns {Promise<import('./types.js').StoredDocument|null>}
	 */
	async get(id) {
		const db = await this.#getDB();
		const byKey = await new Promise((resolve, reject) => {
			const tx = db.transaction(STORE_NAME, 'readonly');
			const request = tx.objectStore(STORE_NAME).get(id);
			request.onsuccess = () => resolve(request.result || null);
			request.onerror = () => reject(request.error);
		});
		if (byKey || !UUID_RE.test(id)) return byKey;
		const all = await new Promise((resolve, reject) => {
			const tx = db.transaction(STORE_NAME, 'readonly');
			const request = tx.objectStore(STORE_NAME).getAll();
			request.onsuccess = () => resolve(request.result || []);
			request.onerror = () => reject(request.error);
		});
		let linkedMatch = null;
		for (const doc of all) {
			let docId = null;
			try {
				docId = JSON.parse(doc.json)?.document?.id ?? null;
			} catch {
				continue;
			}
			if (docId !== id) continue;
			if (!doc.link) return doc;
			linkedMatch = linkedMatch ?? doc;
		}
		return linkedMatch;
	}

	/**
	 * Create or update a document.
	 * @param {import('./types.js').StoredDocument} doc
	 * @returns {Promise<void>}
	 */
	async put(doc) {
		const db = await this.#getDB();
		return new Promise((resolve, reject) => {
			const tx = db.transaction(STORE_NAME, 'readwrite');
			const store = tx.objectStore(STORE_NAME);
			const request = store.put(doc);
			request.onsuccess = () => resolve();
			request.onerror = () => reject(request.error);
		});
	}

	/**
	 * Delete a document by ID.
	 * @param {string} id
	 * @returns {Promise<void>}
	 */
	async delete(id) {
		const db = await this.#getDB();
		return new Promise((resolve, reject) => {
			const tx = db.transaction(STORE_NAME, 'readwrite');
			const store = tx.objectStore(STORE_NAME);
			const request = store.delete(id);
			request.onsuccess = () => resolve();
			request.onerror = () => reject(request.error);
		});
	}

	/**
	 * Get a shareable URL for a document (not supported for local storage).
	 * @param {string} _id
	 * @returns {Promise<string|null>}
	 */
	async getShareUrl(_id) {
		return null;
	}
}
