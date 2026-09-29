# Site Deployment (Vercel)

`site/` is a dependency-free static bundle served straight from the repository
root: `index.html`, `styles.css`, `app.js`, and the assets. There is no site
backend, no API route, and no server-side rendering — the only dynamic input is
a generated Supabase config file. This document is the reproducible deploy path
and the honest inventory of what is static, mocked, or live.

## Reproduce a fresh deploy

From the repository root:

```bash
cd site

# No npm dependencies today; this keeps an added lockfile honest.
npm install

# Generate site/auth-config.js from the environment (see "Environment").
SETU_SUPABASE_URL="https://<project-ref>.supabase.co" \
SETU_SUPABASE_ANON_KEY="<anon-or-publishable-key>" \
npm run build

# Static + smoke checks (required by .github/workflows/site-ci.yml).
npm run smoke

# Deploy.
npx vercel --prod
```

`site/package.json` defines every command used above:

| Script   | Command                    | What it does                                             |
| -------- | -------------------------- | -------------------------------------------------------- |
| `build`  | `node build-config.cjs`    | writes `site/auth-config.js` from the environment        |
| `dev`    | `node server.cjs`          | static server on `127.0.0.1:4174` (override with `PORT`) |
| `smoke`  | `node smoke-check.cjs`     | file/syntax/reference checks + a server round trip       |
| `start`  | `npx serve .`              | serve the directory as-is                                |
| `deploy` | `npx vercel --prod`        | production deploy                                        |

`site/vercel.json` pins the platform side: `buildCommand: npm run build`,
`outputDirectory: "."`, `cleanUrls: true`, and the security headers
(`X-Content-Type-Options`, `Referrer-Policy`, `X-Frame-Options: DENY`,
`Permissions-Policy`). Because the output directory is the site directory
itself, whatever is present at build time is what gets published —
`site/auth-config.js` and `site/.vercel` are git-ignored on purpose.

## Environment

`build-config.cjs` reads, in order of preference:

| Purpose       | Primary                        | Fallbacks                                                        |
| ------------- | ------------------------------ | ---------------------------------------------------------------- |
| Supabase URL  | `SETU_SUPABASE_URL`            | `SUPABASE_URL`                                                    |
| Supabase key  | `SETU_SUPABASE_ANON_KEY`       | `SUPABASE_ANON_KEY`, `SUPABASE_PUBLISHABLE_KEY`                   |

Only the anon/publishable key is ever read; a service-role key must never be
placed in `site/`. When both values are present the build writes
`window.SETU_SUPABASE_CONFIG = { enabled: true, url, anonKey }`; when either is
missing it writes `enabled: false` and logs that auth is disabled. The site then
shows a configuration warning instead of opening an unauthenticated workspace.

`server.cjs` (used by `npm run dev`) generates the same config on the fly from
the environment when `auth-config.js` is absent, so local runs do not need a
build step.

Set these variables in the Vercel project for **Production** (and Preview if you
want sign-in to work there).

## What is static, mocked, or live

Be explicit when demoing this prototype:

- **Static** — the entire site bundle (`index.html`, `styles.css`, `app.js`,
  assets) and the generated `auth-config.js`. There is no site API, so a
  deployment cannot "go down" independently of the CDN.
- **Live** — the Stellar **testnet** contract and the Groth16 verification it
  performs (see the contract id in the root `README.md`), and Supabase Auth once
  the environment above is configured. Supabase holds real user accounts; the
  site never sees the service-role key.
- **Mocked / stubbed** — fiat on-ramp and INR off-ramp flows, the anchor
  integration, gas/relayer privacy, and the local trusted setup. These are
  product story and test scaffolding, not live integrations. See
  [Privacy & Compliance Limitations](privacy-compliance-limitations.md).

The site does not claim a live backend beyond the testnet contract and the
Supabase project you configure. If you deploy without Supabase variables, the
deployment is a static, unauthenticated marketing/demo page.

## Rollback

The site is static and holds no server-side state, so a rollback only re-points
traffic at a previous immutable deployment:

1. In the Vercel dashboard open the project → **Deployments**, find the last
   known-good deployment, and **Promote to Production** (or run
   `npx vercel rollback` from `site/` to go back to the previous production
   deployment).
2. Because `auth-config.js` is regenerated on every build from the environment,
   there is nothing to migrate: rolling back also rolls back the Supabase URL/key
   that was baked in, so a rollback to a deployment built before a Supabase
   project change would point at the old project.
3. If the rollback is caused by a bad environment value, fix the variable in the
   Vercel project and redeploy — do not rely on the previous build.

The Soroban contract is deployed independently on testnet and is **not** affected
by a site rollback; contract changes follow the deployment and upgrade path in
the root [README](../README.md).

## CI

`.github/workflows/site-ci.yml` runs `npm run smoke` in `site/` on every push to
`main` and on pull requests. It checks that the required files exist, syntax
checks the site scripts, generates the auth config when it is absent (and cleans
up a config it generated), resolves the local references in `index.html`, and
boots `server.cjs` for a request round trip. It is a static and smoke-level
check, not a browser test suite.
