# MarkdownFS docs site

Source for [docs.markdownfs.com](https://docs.markdownfs.com/), built with MkDocs Material.

## Local development

```bash
uv sync
uv run mkdocs serve
```

Open http://localhost:8000.

## Build

```bash
uv run mkdocs build --strict
```

Output in `site/`. The CI workflow at `.github/workflows/docs.yml` builds and deploys on every push to `master`.

## Custom domain setup (one-time)

1. Push to `master`. The workflow builds and deploys to GitHub Pages.
2. In **GitHub repo → Settings → Pages**:
   - Source: GitHub Actions
   - Custom domain: `docs.markdownfs.com`
   - Tick "Enforce HTTPS" once the cert is issued (~10 minutes after DNS resolves).
3. At your DNS registrar, add a CNAME record:

   ```
   docs.markdownfs.com.  CNAME  <github-username>.github.io.
   ```

   (Replace `<github-username>` with the account that owns the repo.) Apex domains use four `A` records pointing at GitHub's Pages IPs instead — see [GitHub's docs](https://docs.github.com/en/pages/configuring-a-custom-domain-for-your-github-pages-site/managing-a-custom-domain-for-your-github-pages-site).

The `docs-site/docs/CNAME` file in this repo guarantees the custom domain survives every redeploy.
