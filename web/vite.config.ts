import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig(() => {
  const explicitBase = process.env.VITE_BASE;
  const isGitHubPages = process.env.GITHUB_PAGES === 'true';
  const repoName = process.env.GITHUB_REPOSITORY?.split('/')?.[1];
  const isCI = process.env.CI === 'true' || !!process.env.GITHUB_ACTIONS;
  const inferredPagesBase = repoName ? `/${repoName}/` : '/';
  const base = explicitBase
    || (isGitHubPages ? '/framework-control/' : (isCI ? inferredPagesBase : '/'));
  return {
    base,
    plugins: [svelte()],
    server: {
      port: 5174,
    },
  };
});


