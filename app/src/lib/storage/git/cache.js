/**
 * Content cache keyed by `content_hash` (§7.1): bytes fetched through a
 * locator are kept in a separate IndexedDB database so re-opening a linked
 * document needs no network and a pinned source never re-downloads.
 * Separate from the `waffle-iron` documents DB so its schema (and the
 * test helpers that open it at version 1) stay untouched.
 */

const DB_NAME = 'waffle-iron-cache';
const DB_VERSION = 1;
const STORE = 'content';

/** @type {Promise<IDBDatabase>|null} */
let dbPromise = null;

function openDB() {
	if (!dbPromise) {
		dbPromise = new Promise((resolve, reject) => {
			if (typeof indexedDB === 'undefined') {
				reject(new Error('IndexedDB unavailable'));
				return;
			}
			const req = indexedDB.open(DB_NAME, DB_VERSION);
			req.onupgradeneeded = (e) => {
				const db = e.target.result;
				if (!db.objectStoreNames.contains(STORE)) {
					const store = db.createObjectStore(STORE, { keyPath: 'hash' });
					store.createIndex('fetched', 'fetched', { unique: false });
				}
			};
			req.onsuccess = () => resolve(req.result);
			req.onerror = () => reject(req.error);
		});
	}
	return dbPromise;
}

/**
 * @param {string} hash - a `content_hash` (algorithm-prefixed)
 * @returns {Promise<string|null>} the cached text
 */
export async function cacheGet(hash) {
	if (!hash) return null;
	try {
		const db = await openDB();
		return await new Promise((resolve, reject) => {
			const req = db.transaction(STORE, 'readonly').objectStore(STORE).get(hash);
			req.onsuccess = () => resolve(req.result?.text ?? null);
			req.onerror = () => reject(req.error);
		});
	} catch {
		return null;
	}
}

/**
 * @param {string} hash
 * @param {string} text
 */
export async function cachePut(hash, text) {
	if (!hash) return;
	try {
		const db = await openDB();
		await new Promise((resolve, reject) => {
			const req = db
				.transaction(STORE, 'readwrite')
				.objectStore(STORE)
				.put({ hash, text, fetched: Date.now() });
			req.onsuccess = () => resolve();
			req.onerror = () => reject(req.error);
		});
	} catch {
		// A cache miss later is the only consequence.
	}
}

/** Drop everything (settings → "clear cached sources"). */
export async function cacheClear() {
	try {
		const db = await openDB();
		await new Promise((resolve, reject) => {
			const req = db.transaction(STORE, 'readwrite').objectStore(STORE).clear();
			req.onsuccess = () => resolve();
			req.onerror = () => reject(req.error);
		});
	} catch {
		// nothing to clear
	}
}
