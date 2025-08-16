import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig(() => {
  const isGitHubPages = process.env.GITHUB_PAGES === 'true';
  const base = isGitHubPages ? '/framework-control/' : '/';
  return {
    base,
    plugins: [svelte()],
    server: {
      port: 5174,
    },
  };
});


