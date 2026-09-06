# Incident Tracker — Rust + PostgreSQL on Clever Cloud

> A Rust (Axum) incident management app connected to PostgreSQL, deployed on Clever Cloud's Rust runtime. Demonstrates async Rust web development with automatic database migrations — dressed with the **Clever Brand Kit** and putting the **Clever Cloud certification** front and center.

---

## Deploy on Clever Cloud

1. Fork this repository
2. In the Clever Cloud console, create a new **Rust** application — connect your forked repo
3. Add a **PostgreSQL** add-on and link it to your app
4. Set the `DATABASE_URL` environment variable to the value of `POSTGRESQL_ADDON_URI` (or `POSTGRESQL_ADDON_DIRECT_URI`) — the app also falls back to `POSTGRESQL_ADDON_URI` automatically
5. Optionally set `RUST_LOG=incident_tracker=info,tower_http=info`
6. Push → Clever Cloud builds with `cargo build --release` and deploys automatically

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
- `/health` endpoint (200 OK)
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

- Rust (stable, 1.75+)
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
| `DATABASE_URL` | ✅       | PostgreSQL connection string (falls back to `POSTGRESQL_ADDON_URI`) |
| `PORT`         | auto     | Injected by Clever Cloud (default: 8080)                |
| `RUST_LOG`     | —        | Log level, e.g. `incident_tracker=info,tower_http=info` |
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
| GET    | `/health`                 | Health check (200 OK)                |
| GET    | `/cc-brand.css`           | Clever Brand Kit stylesheet (embedded) |

> **Axum 0.8 note:** path parameters use the `{id}` syntax. The former `:id` syntax makes Axum 0.8 **panic at startup** (`Path segments must not start with ':'`) — this was fixed in this repository.

---

## Deployment Notes

- `DATABASE_URL` (or the linked add-on's `POSTGRESQL_ADDON_URI`) must be available before the first deploy — the app will crash on startup without it
- Migrations are applied automatically at startup via `sqlx::migrate!()` — no manual migration step needed
- The binary listens on `0.0.0.0:$PORT` as required by Clever Cloud
- First build is slow (~3–5 min) — Clever Cloud caches compiled artifacts for subsequent deploys
