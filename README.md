# Incident Tracker — Rust + PostgreSQL on Clever Cloud

> A Rust (Axum) incident management app connected to PostgreSQL, built for Clever Cloud's **native Rust runtime** (no Docker image, no Dockerfile). Demonstrates async Rust web development with automatic database migrations — dressed with the **Clever Brand Kit** and putting the **Clever Cloud certification** front and center.

---

## Deploy on Clever Cloud

1. Fork this repository
2. In the Clever Cloud console, create a new **Rust** application — connect your forked repo
3. Add a **PostgreSQL** add-on and link it to your app — the app reads `POSTGRESQL_ADDON_URI` directly, **do not** copy it into `DATABASE_URL`
4. Recommended environment variables (console or `clever env set`):
   - `DB_POOL_MAX=2` — pool size; keep `DB_POOL_MAX × instances` below the add-on's connection limit (DEV plan = 5)
   - `RUST_LOG=incident_tracker=info,tower_http=info` (already the built-in default)
   - `CC_HEALTH_CHECK_PATH=/health` — the platform validates the deployment against PostgreSQL
   - `CC_RUST_VERSION=1.94` — pins the toolchain used by the build (minimum declared: 1.85)
5. Push → Clever Cloud builds with `cargo build --release --locked` and deploys automatically

> **Build time:** First deploy takes ~3–5 minutes (Rust compilation). Subsequent deploys reuse the cache and are faster.

---

## Stack

| Layer         | Technology          |
|---------------|---------------------|
| Language      | Rust (edition 2021) |
| Web framework | Axum 0.8            |
| Async runtime | Tokio               |
| Database      | PostgreSQL          |
| DB access     | SQLx 0.8            |
| Templates     | Askama 0.12         |
| Design        | Clever Brand Kit (Plus Jakarta Sans, navy #13172e, dégradé Clever) |

---

## Features

- List, create, and update incidents
- Filter by status: open / investigating / resolved (`?status=`)
- Severity levels: low / medium / high / critical
- Automatic database migrations at startup (`sqlx::migrate!()`)
- `/health` endpoint — 200 when PostgreSQL answers `SELECT 1`, 503 otherwise
- Server-side validation: title ≤ 255, service ≤ 100, description ≤ 10 000 characters (form error, never a 500)
- Security headers on every response (`X-Content-Type-Options`, `Referrer-Policy`, `X-Frame-Options`, `Content-Security-Policy`) — TLS/HSTS stay on the Clever Cloud proxy
- Graceful shutdown on SIGTERM/Ctrl+C (in-flight requests drained, pool closed)
- `/stats` page with incident counts
- **« Vu depuis Clever Cloud » panel**: shows the variables the platform injects (`CC_APP_NAME`, `APP_ID`, `INSTANCE_NUMBER`, `INSTANCE_TYPE`, `CC_PRETTY_INSTANCE_NAME`, `CC_COMMIT_ID`, `CC_DEPLOYMENT_ID`) — displays « Local · hors Clever Cloud » when running outside the platform

---

## Design — Clever Brand Kit

The UI uses the shared **Clever Brand Kit**: a single stylesheet (`static/cc-brand.css`, tokens `--cc-*`, Plus Jakarta Sans + JetBrains Mono via Google Fonts with system fallbacks), the official Clever Cloud logo and the certification badge inlined as Askama partials (`templates/partials/`). No dependency was added: the CSS is embedded in the binary with `include_str!` and served on `/cc-brand.css`.

Page structure (identical to the kit's reference page): sticky topbar → hero → **certification block** → demo content + platform panel → « Ce que Clever Cloud fait » → footer. App-specific styles live in a short `<style>` block in `templates/base.html`; `cc-brand.css` is never edited.

Spec: `docs/superpowers/specs/2026-09-06-clever-brand-design.md`.

---

## Certification Clever Cloud

The home page puts the **Clever Cloud Academy** certification right under the hero: the two official tracks (*Cloud Computing Fundamentals*, *Advanced Deployment*) and a call to action. Every page carries a « Se certifier ↗ » link in the topbar.

→ **[academy.clever.cloud](https://academy.clever.cloud/)** — digital badges are issued automatically once a track is validated.

---

## Local Development

### Prerequisites

- Rust (stable, 1.85+ — `rust-version` in `Cargo.toml`)
- PostgreSQL running locally

### Run

```bash
git clone https://github.com/Vitiosum/demo-rust-postgresql
cd demo-rust-postgresql
cp .env.example .env
# Edit .env: set DATABASE_URL to your local PostgreSQL connection string
cargo run
# → http://localhost:8080
```

One-liner without `.env`:

```bash
DATABASE_URL=postgres://localhost/demo_rust PORT=8082 cargo run
```

---

## Environment Variables

| Variable       | Required | Description                                             |
|----------------|----------|---------------------------------------------------------|
| `POSTGRESQL_ADDON_URI` | ✅ (Clever) | Injected by the linked PostgreSQL add-on; read when `DATABASE_URL` is absent |
| `DATABASE_URL` | ✅ (local) | PostgreSQL connection string for local runs (takes precedence if set) |
| `PORT`         | auto     | Injected by Clever Cloud (default: 8080)                |
| `DB_POOL_MAX`  | —        | Max pool connections (default: 2). Rule: `DB_POOL_MAX × instances ≤ add-on limit` (DEV plan = 5) |
| `RUST_LOG`     | —        | Log filter (default: `incident_tracker=info,tower_http=info`) |
| `CC_HEALTH_CHECK_PATH` | — (Clever) | Set to `/health` so the platform checks PostgreSQL before routing traffic |
| `CC_RUST_VERSION` | — (Clever) | Pins the Rust toolchain used by the build (e.g. `1.94`) |
| `CC_APP_NAME`, `APP_ID`, `INSTANCE_NUMBER`, `INSTANCE_TYPE`, `CC_PRETTY_INSTANCE_NAME`, `CC_COMMIT_ID`, `CC_DEPLOYMENT_ID` | auto | Injected by Clever Cloud, read-only, displayed in the platform panel |

---

## Endpoints

| Method | Path                      | Description                          |
|--------|---------------------------|--------------------------------------|
| GET    | `/`                       | List incidents (`?status=` filter)   |
| GET    | `/incidents/new`          | Create form                          |
| POST   | `/incidents`              | Create incident                      |
| GET    | `/incidents/{id}`         | Incident detail                      |
| POST   | `/incidents/{id}/status`  | Update status                        |
| GET    | `/stats`                  | Statistics                           |
| GET    | `/health`                 | Health check (200 if PostgreSQL answers, 503 otherwise) |
| GET    | `/cc-brand.css`           | Clever Brand Kit stylesheet (embedded) |

> **Axum 0.8 note:** path parameters use the `{id}` syntax. The former `:id` syntax makes Axum 0.8 **panic at startup** (`Path segments must not start with ':'`) — this was fixed in this repository.

---

## Deployment Notes

- The PostgreSQL add-on must be linked before the first deploy (`POSTGRESQL_ADDON_URI`) — the app exits at startup without it
- Pool sizing: `DB_POOL_MAX` (default 2) × number of instances must stay below the add-on's connection limit — a redeploy briefly runs two instances
- `clevercloud/rust.json` **is** picked up by the platform at build time (build log: `Configuration file detected: …/clevercloud/rust.json`); the Rust runtime itself is configured through environment variables — nothing indicates that its `appIsToBeBuilt` key changes the runtime's behaviour
- Migrations are applied automatically at startup via `sqlx::migrate!()` — no manual migration step needed
- The binary listens on `0.0.0.0:$PORT` as required by Clever Cloud
- First build is slow (~3–5 min) — Clever Cloud caches compiled artifacts for subsequent deploys
