/**
 * v4 Phase 2 P2-5b — a GitLab repository as a document storage provider
 * (specs/waffle_v4_document_model.md §7.1–7.4), against an in-memory mock of
 * the GitLab files API: connect with a token through the dialog, create a
 * document (file + index written, keyed by document.id), open it from its
 * card through the provider-aware /doc route, save, share, survive a reload,
 * delete, disconnect.
 */
import { test as rawTest, expect } from '@playwright/test';

const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const PROJECT = '/api/v4/projects/acme%2Fparts';

/** In-memory GitLab: project exists; files API over `files` map. */
async function mockGitLab(page, { files = new Map(), calls = [] } = {}) {
	await page.route('https://gitlab.com/api/v4/**', async (route) => {
		const req = route.request();
		const url = new URL(req.url());
		const method = req.method();
		calls.push(`${method} ${url.pathname}${url.search}`);
		const auth = req.headers()['authorization'];
		if (!auth) return route.fulfill({ status: 401, contentType: 'application/json', body: '{"message":"401 Unauthorized"}' });
		if (url.pathname === PROJECT && method === 'GET') {
			return route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ id: 1, path_with_namespace: 'acme/parts' }) });
		}
		const m = new RegExp(`^${PROJECT}/repository/files/([^/]+)$`).exec(url.pathname);
		if (!m) return route.fulfill({ status: 404, contentType: 'application/json', body: '{"message":"404"}' });
		const path = decodeURIComponent(m[1]);
		if (method === 'GET') {
			if (!files.has(path)) return route.fulfill({ status: 404, contentType: 'application/json', body: '{"message":"404 File Not Found"}' });
			return route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ content: Buffer.from(files.get(path), 'utf8').toString('base64'), blob_id: 'b' + files.get(path).length, encoding: 'base64' }) });
		}
		const body = req.postDataJSON?.() ?? {};
		if (method === 'POST' || method === 'PUT') {
			if (method === 'POST' && files.has(path)) return route.fulfill({ status: 400, body: '{"message":"exists"}' });
			if (method === 'PUT' && !files.has(path)) return route.fulfill({ status: 400, body: '{"message":"missing"}' });
			files.set(path, body.content);
			return route.fulfill({ status: method === 'POST' ? 201 : 200, contentType: 'application/json', body: JSON.stringify({ file_path: path, branch: body.branch }) });
		}
		if (method === 'DELETE') {
			files.delete(path);
			return route.fulfill({ status: 204, body: '' });
		}
		return route.fulfill({ status: 405, body: '' });
	});
	return { files, calls };
}

async function connectGitLab(page) {
	await page.goto('/home');
	await page.waitForTimeout(300);
	await page.locator('[data-testid="provider-dropdown"] button').first().click();
	await page.locator('[data-testid="provider-connect-git"]').click();
	await expect(page.locator('[data-testid="git-connect-dialog"]')).toBeVisible();
	await page.locator('[data-testid="git-connect-remote"]').fill('https://gitlab.com/acme/parts.git');
	await expect(page.locator('[data-testid="git-connect-kind"]')).toContainText('gitlab · acme/parts');
	await page.locator('[data-testid="git-connect-token"]').fill('glpat-secret');
	await page.locator('[data-testid="git-connect-btn"]').click();
	await expect(page.locator('[data-testid="git-connect-dialog"]')).toBeHidden({ timeout: 10000 });
	await expect(page.locator('[data-testid="provider-dropdown"] button').first()).toContainText('GitLab · acme/parts');
}

rawTest.describe('Git storage provider (GitLab)', () => {
	rawTest('connects with a token, stores the config per host, and refuses a missing repository', async ({ page }) => {
		const { calls } = await mockGitLab(page);
		await connectGitLab(page);
		const saved = await page.evaluate(() => ({
			providers: JSON.parse(localStorage.getItem('waffle-git-providers') || '[]'),
			tokens: JSON.parse(localStorage.getItem('waffle-host-tokens') || '[]'),
			active: localStorage.getItem('waffle-active-provider')
		}));
		expect(saved.providers).toEqual([{ id: 'git:gitlab.com/acme/parts', kind: 'gitlab', remote: 'https://gitlab.com/acme/parts', branch: 'main', folder: '', label: 'GitLab · acme/parts' }]);
		expect(saved.tokens).toEqual([{ host_url: 'https://gitlab.com', token: 'glpat-secret', kind: 'gitlab' }]);
		expect(saved.active).toBe('git:gitlab.com/acme/parts');
		expect(calls.some((c) => c === `GET ${PROJECT}`)).toBe(true);

		// A repository that does not exist is reported, not connected.
		await page.route('https://gitlab.com/api/v4/projects/acme%2Fnope**', (route) =>
			route.fulfill({ status: 404, contentType: 'application/json', body: '{"message":"404 Project Not Found"}' })
		);
		await page.locator('[data-testid="provider-dropdown"] button').first().click();
		await page.locator('[data-testid="provider-connect-git"]').click();
		await page.locator('[data-testid="git-connect-remote"]').fill('https://gitlab.com/acme/nope');
		await page.locator('[data-testid="git-connect-btn"]').click();
		await expect(page.locator('[data-testid="git-connect-error"]')).toContainText('repository not found');
	});

	rawTest('creates, opens, saves, shares, lists after reload, deletes, disconnects', async ({ page }) => {
		const { files, calls } = await mockGitLab(page);
		await connectGitLab(page);

		// New document → file + index written to the repo, keyed by document.id.
		await page.locator('[data-testid="new-document-btn"]').click();
		await page.waitForURL((url) => url.pathname.startsWith('/doc/'), { timeout: 15000 });
		const id = new URL(page.url()).pathname.split('/').pop();
		expect(id).toMatch(UUID_RE);
		expect([...files.keys()].sort()).toEqual(['.waffle-index.json', 'untitled.waffle']);
		const index = JSON.parse(files.get('.waffle-index.json'));
		expect(index).toHaveLength(1);
		expect(index[0].id).toBe(id);
		expect(index[0].filename).toBe('untitled.waffle');
		expect(JSON.parse(files.get('untitled.waffle')).document.id).toBe(id);

		// The /doc route reads the ACTIVE provider (not the local store).
		await page.waitForURL('/', { timeout: 15000 });
		await page.waitForFunction(() => window.__waffle?.getState()?.engineReady === true, { timeout: 30000 });
		await page.waitForFunction((d) => window.__waffle.getDocumentState().activeDocId === d, id, { timeout: 15000 });
		expect(calls.some((c) => c.startsWith(`GET ${PROJECT}/repository/files/untitled.waffle?ref=main`))).toBe(true);

		// Ctrl+S saves back through the files API (PUT on an existing file).
		const putsBefore = calls.filter((c) => c.startsWith('PUT')).length;
		await page.keyboard.press('Control+s');
		await page.waitForFunction(() => (window.__waffle.getToasts() || []).some((t) => t.message === 'Saved'), { timeout: 10000 });
		await expect.poll(() => calls.filter((c) => c.startsWith(`PUT ${PROJECT}/repository/files/untitled.waffle`)).length).toBeGreaterThan(putsBefore - 1);
		expect(JSON.parse(files.get('untitled.waffle')).format).toBe('waffle-iron');

		// Its share link is the /open locator for this repo, and its location resolves Relative links.
		// (A dynamic `/src/…` import is a second module instance, not the app's
		// registry — so build the provider from its saved config instead.)
		const share = await page.evaluate(async (d) => {
			const { GitProvider } = await import('/src/lib/storage/git-provider.js');
			const cfg = JSON.parse(localStorage.getItem('waffle-git-providers'))[0];
			const p = new GitProvider(cfg);
			return { url: await p.getShareUrl(d), locator: await p.getLocator(d), id: p.id };
		}, id);
		expect(share.id).toBe('git:gitlab.com/acme/parts');
		expect(share.url).toBe('http://localhost:5173/open?remote=https%3A%2F%2Fgitlab.com%2Facme%2Fparts&path=untitled.waffle&ref=main');
		expect(share.locator).toEqual({ type: 'Git', remote: 'https://gitlab.com/acme/parts', path: 'untitled.waffle', ref: { type: 'Branch', name: 'main' }, host: 'gitlab' });

		// After a reload the provider is re-registered from its saved config and lists the document.
		await page.goto('/home');
		await expect(page.locator('[data-testid="document-card"]')).toBeVisible({ timeout: 10000 });
		await expect(page.locator('[data-testid="document-card"]')).toContainText('Untitled');
		await expect(page.locator('[data-testid="provider-dropdown"] button').first()).toContainText('GitLab · acme/parts');

		// Copy share link from the card.
		await page.locator('[data-testid="document-card"]').first().click({ button: 'right' });
		await page.locator('[data-testid="doc-ctx-share"]').click();
		await expect(page.locator('[data-testid="share-notice"]')).toContainText('/open?remote=https%3A%2F%2Fgitlab.com%2Facme%2Fparts');

		// Delete removes the file and the index entry.
		page.once('dialog', (d) => d.accept());
		await page.locator('[data-testid="document-card"]').first().click({ button: 'right' });
		await page.locator('[data-testid="doc-ctx-delete"]').click();
		await expect(page.locator('[data-testid="empty-state"]')).toBeVisible({ timeout: 10000 });
		expect(files.has('untitled.waffle')).toBe(false);
		expect(JSON.parse(files.get('.waffle-index.json'))).toEqual([]);

		// Disconnect forgets the repository (the host token stays).
		await page.locator('[data-testid="provider-dropdown"] button').first().click();
		await page.locator('[data-testid="provider-disconnect-git:gitlab.com/acme/parts"]').click();
		await expect(page.locator('[data-testid="provider-dropdown"] button').first()).toContainText('This Browser');
		const after = await page.evaluate(() => ({
			providers: JSON.parse(localStorage.getItem('waffle-git-providers') || '[]'),
			tokens: JSON.parse(localStorage.getItem('waffle-host-tokens') || '[]').map((t) => t.host_url)
		}));
		expect(after.providers).toEqual([]);
		expect(after.tokens).toEqual(['https://gitlab.com']);
	});
});
