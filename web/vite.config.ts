import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { VitePWA } from 'vite-plugin-pwa';

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
    plugins: [
      svelte(),
      VitePWA({
        registerType: 'autoUpdate',
        includeAssets: ['assets/**/*'],
        manifest: {
          name: 'Framework Control',
          short_name: 'FW Control',
          start_url: '/',
          scope: '/',
          display: 'standalone',
          background_color: '#0b0d10',
          theme_color: '#16a34a',
          icons: [
            { src: '/assets/generated/icon-192.png', sizes: '192x192', type: 'image/png' },
            { src: '/assets/generated/icon-512.png', sizes: '512x512', type: 'image/png' },
            { src: '/assets/generated/maskable-icon-192.png', sizes: '192x192', type: 'image/png', purpose: 'maskable' },
            { src: '/assets/generated/maskable-icon-512.png', sizes: '512x512', type: 'image/png', purpose: 'maskable' }
          ]
        },
        workbox: {
          skipWaiting: true,
          clientsClaim: true,
          navigateFallback: '/index.html',
          globPatterns: ['**/*.{js,css,html,svg,png,woff2}'],
          runtimeCaching: [
            {
              urlPattern: ({ url }) => url.pathname.startsWith('/api/'),
              handler: 'NetworkOnly' as const,
            }
          ]
        }
      })
    ],
    server: {
      port: 5174,
    },
  };
});


