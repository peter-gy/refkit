# Documentation Site

The user-facing documentation is a standalone [VitePress](https://vitepress.dev/) project under `docs/`. VitePress compiles Markdown and the TypeScript site configuration into a static website. [pnpm](https://pnpm.io/) installs the exact JavaScript dependency graph recorded in `docs/pnpm-lock.yaml`. [Portless](https://portless.sh/) gives the development server a stable local HTTPS URL.

## Run The Site

Install the locked dependencies:

```bash
pnpm --dir docs install --frozen-lockfile
```

Start the development server:

```bash
pnpm --dir docs dev
```

Portless serves the main checkout at `https://docs.refkit.localhost`. It assigns the child process a free port through `PORT`, and the VitePress configuration binds that exact port on IPv4 loopback. A linked Git worktree receives its branch name as a hostname prefix.

The first Portless run creates and trusts a local certificate authority and can request administrator approval to bind HTTPS on port 443. Run the VitePress server directly when the proxy or certificate trust is not needed:

```bash
pnpm --dir docs dev:direct
```

The direct server binds `127.0.0.1` and uses VitePress's default development port. `make docs-dev` uses the named Portless route.

Build and verify the static output:

```bash
pnpm --dir docs build
```

The build runs a strict TypeScript check, VitePress, the llms.txt generator, and `docs/scripts/verify-build.mjs`. The verifier checks routes, public assets, social metadata, theme-specific brand references, internal links, heading fragments, raw Markdown twins, `llms.txt`, and `llms-full.txt`.

The published site is `https://peter-gy.github.io/refkit/`. GitHub Pages serves it under `/refkit/`, while local VitePress and Portless development use `/`.

`BASE_PATH` carries the Pages deployment path into VitePress. The configuration applies it to site navigation, scripts, styles, and static head assets. Canonical URLs, Open Graph URLs, `sitemap.xml`, `robots.txt`, and agent-readable Markdown URLs always use the published site URL.

`make docs-check` builds and verifies both `/refkit/` and `/`. `make docs-build` builds one artifact with the current `BASE_PATH`.

`make docs-check` composes the Markdown audience-boundary check with the locked VitePress install and build.

## Information Architecture

`docs/.vitepress/navigation.json` owns top-level navigation and the complete sidebar. The static verifier compares it with every public Markdown page. `docs/.vitepress/config.mts` owns search, source links, previous and next navigation, and theme assets.

Public pages follow this reader path:

1. The landing page establishes product fit and routes to one first result.
2. Get Started installs one package and renders a visible citation.
3. Concepts define normalized and raw bibliography models, ordered rendering, and recovery.
4. Guides complete parsing, rendering, raw editing, formatting, Polars, and Pyodide tasks.
5. Reference pages define exact Python, Polars, data-shape, option, selector, error, Rust adapter, and agent-readable documentation contracts.
6. Migration and troubleshooting pages support change and recovery.

Every public page must appear in navigation or be an intentional compatibility route. Use extensionless internal links because VitePress owns the output suffix.

## Agent-Readable Output

`vitepress-plugin-llms` derives three text surfaces from the public Markdown source:

- `llms.txt` lists every page with its title, description, and Markdown URL.
- `llms-full.txt` concatenates the complete public documentation.
- Each HTML route receives a raw Markdown twin ending in `.md`.

The plugin uses the configured navigation order. The verifier compares both llms inventories with the complete route set and rejects missing, unexpected, or duplicate URLs. It also requires every raw Markdown twin to be present and non-empty.

## GitHub Pages

`.github/workflows/pages.yml` builds documentation for pull requests and `main`. Pull requests prove the root-based build. A `main` build reads `base_path` from `actions/configure-pages`, builds the `/refkit/` artifact, uploads `docs/.vitepress/dist`, and deploys that exact artifact.

The workflow includes `.nojekyll` so GitHub Pages serves VitePress asset directories and generated Markdown files directly.

## Public Assets

VitePress copies `docs/public` to the static output root. The site uses:

- Light and dark horizontal lockups in the navigation.
- Light and dark vertical lockups in the home hero.
- Light and dark marks as media-qualified browser icons.
- `favicon.ico` as the fallback browser icon requested before the themed document head loads.
- `og.png` for Open Graph and Twitter cards.
- `og-dark.png` and `og-light.png` as the themed source variants.
- Light and dark Lucide feature icons through VitePress's native `icon` frontmatter field.

The feature icons are decorative because each card has a visible title. Keep their alternative text empty and preserve the Lucide license beside the public assets.

Keep image dimensions, alternative text, social metadata, and public filenames stable together. The static build verifier checks publication and references. Use browser inspection to confirm which themed asset is active.

## Validate Delivery

Inspect the built preview at desktop and narrow widths. Check:

- Home, concept, guide, reference, migration, troubleshooting, and unknown routes.
- Top navigation, sidebar, previous and next links, source links, and local search.
- Search results for `Library`, `BibDocument`, `Document`, `Citation Style Language`, `TidyOptions`, and Polars.
- Light and dark logos, hero assets, colors, and persisted theme preference.
- Code blocks, wide tables, focus states, landmarks, alt text, and mobile menus.
- Browser console errors and failed asset requests.
- Root and `/refkit/` asset, navigation, search, Markdown, llms, sitemap, and canonical URLs.

Run `make docs-check test` after changing documentation source, configuration, validation, or published assets.
