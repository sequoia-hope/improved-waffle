/**
 * GitHub-backed document storage: the `GitProvider` (`git-provider.js`) on
 * `https://github.com/<login>/<repo>` at `main`, kept under the historical
 * provider id `github` for the device-flow sign-in and existing configs.
 * Adds repository auto-creation, which the generic provider does not do.
 *
 * @implements {import('./types.js').DocumentStore}
 */

import { GitHostError } from './git/hosts.js';
import { GitProvider, buildSlug } from './git-provider.js';

export { buildSlug };
export { GitHostError as GitHubStorageError };

export class GitHubStore extends GitProvider {
	#owner;
	#repo;
	#ghToken;

	/**
	 * @param {string} token - GitHub access token
	 * @param {string} owner - GitHub username
	 * @param {string} [repo='waffle-iron-documents'] - Repository name
	 */
	constructor(token, owner, repo = 'waffle-iron-documents') {
		super(
			{
				id: 'github',
				kind: 'github',
				remote: `https://github.com/${owner}/${repo}`,
				branch: 'main',
				folder: '',
				label: 'GitHub'
			},
			token
		);
		this.#owner = owner;
		this.#repo = repo;
		this.#ghToken = token;
	}

	/**
	 * Ensure the target repository exists, creating it if needed (the
	 * sign-in flow's default documents repo).
	 */
	async ensureRepo() {
		const headers = {
			Authorization: `Bearer ${this.#ghToken}`,
			Accept: 'application/vnd.github+json',
			'Content-Type': 'application/json'
		};
		let res;
		try {
			res = await fetch(`https://api.github.com/repos/${this.#owner}/${this.#repo}`, { headers });
		} catch (err) {
			throw new GitHostError(`Network error: ${err?.message || err}`, 'network');
		}
		if (res.ok) return;
		if (res.status !== 404 && res.status !== 403) {
			throw new GitHostError(`GitHub API error: ${res.status}`, 'api_error');
		}
		const create = await fetch('https://api.github.com/user/repos', {
			method: 'POST',
			headers,
			body: JSON.stringify({
				name: this.#repo,
				description: 'Waffle Iron CAD documents',
				private: false,
				auto_init: true
			})
		});
		if (!create.ok) throw new GitHostError(`Could not create repository: ${create.status}`, 'api_error');
		this.invalidate();
	}
}
