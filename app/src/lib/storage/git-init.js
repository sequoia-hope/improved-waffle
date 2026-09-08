/**
 * Register every saved git storage provider at startup: the legacy GitHub
 * sign-in (device flow, `github-auth.js`) and each host/repo the user
 * connected through the git host dialog (`providers.js`).
 */
import { loadAuth } from './github-auth.js';
import { GitHubStore } from './github.js';
import { GitProvider } from './git-provider.js';
import { registerProvider } from './index.js';
import { listGitProviderConfigs } from './providers.js';

const auth = loadAuth();
if (auth) {
	registerProvider(new GitHubStore(auth.token, auth.login, auth.repo));
}
for (const cfg of listGitProviderConfigs()) {
	if (cfg?.id && cfg.remote) registerProvider(new GitProvider(cfg));
}
