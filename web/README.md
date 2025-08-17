# Framework Control Web

Svelte + Vite minimal UI that talks to the local service at `http://127.0.0.1:8090`.

## Develop

```
npm i
npm run dev
```

Configure the service base URL via `VITE_API_BASE` env var if needed.

## Deploy to GitHub Pages

This project can be deployed as a static site to GitHub Pages. The app uses hash-based routing, so it works well behind Pages.

1) Add a repository variable for the installer URL (optional but recommended):

- Create a public download for your Windows installer (e.g., upload an MSI to a GitHub Release).
- Go to your repository → Settings → Variables → New repository variable.
- Name: `INSTALLER_URL`
- Value: the direct download URL for the MSI (e.g. `https://github.com/<owner>/<repo>/releases/latest/download/FrameworkControlService.msi`).

2) Enable GitHub Pages:

- Go to your repository → Settings → Pages and set Source to "GitHub Actions".

3) Push to `main`.

This repo includes a workflow at `.github/workflows/deploy-pages.yml` that:

- Builds the site in `web/` with Vite.
- Sets the correct base path for Pages.
- Publishes `web/dist` to GitHub Pages.
- Injects the installer URL into the build as `VITE_INSTALLER_URL` from the `INSTALLER_URL` repository variable.

At runtime, if the app cannot reach the local service (via `/api/health`), a banner will appear with a link to download the MSI.


