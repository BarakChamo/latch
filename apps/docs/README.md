# Latch Docs

The app uses Fumadocs with Next.js static export. Content lives in
`content/docs/` and follows the [documentation outline](../../docs/outline.md).
The docs app maintains its own lockfile and build.

The [public site](https://barakchamo.github.io/latch/) is published by GitHub
Actions when the app changes on `main`. To reproduce the Pages build
locally, run `NEXT_PUBLIC_DOCS_BASE_PATH=/latch npm run build` from this
directory. The default build uses the site root for local preview.

| Command | Use |
| --- | --- |
| `npm ci` | Install the docs dependencies |
| `npm run dev` | Preview and edit the docs |
| `npm run types:check` | Validate the docs app's TypeScript |
| `npm run build` | Export the static site into `out/` |
| `npm start` | Serve the exported site locally |
