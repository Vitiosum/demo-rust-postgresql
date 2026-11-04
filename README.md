# Incident Tracker — Rust + PostgreSQL on Clever Cloud

> An on-call incident board: the first screen counts the incidents in progress, draws one bar per incident by severity, and flags the critical and high ones still open past their SLA. Declare an incident, move it from open to investigating to resolved: every change is a row in the **PostgreSQL add-on**. The app is a single Axum + SQLx binary that the **native Rust runtime** compiles with `cargo build --release --locked` on a dedicated build instance, with no Dockerfile; the schema is migrated at startup and the deployment is validated by a health check that queries PostgreSQL.

![Incident Tracker run locally: five incidents in progress, the severity bars and the deployment console with the demo variables](docs/screenshot-desktop.png)

![The incident list: severity, status, service, age and the "SLA dépassé" flag on the critical and high incidents still open](docs/screenshot-incidents.png)

<img src="docs/screenshot-phone.png" alt="Incident Tracker on a phone: the incident count and the severity bars" width="300">

**Run it yourself.** The hosted demo was taken down after the presales review (November 2026). The steps in [Deploy on Clever Cloud](#deploy-on-clever-cloud) recreate it from scratch in about 5 minutes.

---

## What the Rust runtime does here

| Where | Clever Cloud feature | What you see |
|---|---|---|
| Build | Native Rust runtime: `cargo build --release --locked` against the committed `Cargo.lock`, on a dedicated M build instance | Measured deploys: 128 s with no cache, about 80 s once the dependency cache (`CC_CACHE_DEPENDENCIES`) is reused, 35 s for a variable change and `clever restart` (no build) |
| Start-up | The binary listens on `0.0.0.0:$PORT` and runs `sqlx::migrate!()` before serving | No migration step in the deployment: the `incidents` table is created by the app, which starts in under 100 ms |
| Deployment check | `CC_HEALTH_CHECK_PATH=/health`: `SELECT 1` on PostgreSQL, 200 or 503 | A deployment that cannot reach the database never takes the traffic |
| PostgreSQL add-on | `POSTGRESQL_ADDON_URI`, injected by the linked add-on and read directly; pool of `DB_POOL_MAX` connections | Board, list sorted by status then severity, filters (`?status=`), `/stats` |
| Environment variables | `APP_ENV`, `TEAM_NAME`, `SLA_CRITICAL_MINUTES`, `ANNOUNCEMENT`, set in the Console | Environment pill, on-call team, "SLA dépassé" flags, banner on every page. Change one, `clever restart`, reload |
| Platform variables | `CC_APP_NAME`, `APP_ID`, `INSTANCE_NUMBER`, `INSTANCE_TYPE`, `CC_PRETTY_INSTANCE_NAME`, `CC_COMMIT_ID`, `CC_DEPLOYMENT_ID` | Shown in the deployment console of the first screen; "Local · hors Clever Cloud" outside the platform |
| Redeploy | SIGTERM handled: in-flight requests drained, pool closed | In-flight requests finish before the old instance stops |

The steps listed at the top of the deployment console (`git push`, `cargo build`, `sqlx migrate`, `GET /health`) are fixed text; the incident count under them, the variables and the platform values are read live.

---

## Deploy on Clever Cloud

You need [clever-tools](https://www.clever.cloud/developers/doc/cli/) (`clever login` done) and the ID of the organisation that will own the resources (`orga_…`, shown in the Console). Run the commands from the repo root: `clever create` links the folder to the new app.

```bash
# 1. The app and its size (fixed nano first, then the range; the dedicated build is off by default)
clever create --type rust demo-rust-postgresql --org <org_id> --region par
clever scale --flavor nano
clever scale --min-flavor nano --max-flavor XS --min-instances 1 --max-instances 2
clever scale --build-flavor M

# 2. PostgreSQL (dedicated xxs_sml, encrypted at rest), linked to the app
clever addon create postgresql-addon demo-rust-db --plan xxs_sml --option encryption=true --region par --org <org_id> --yes
clever service link-addon demo-rust-db

# 3. Variables
clever env set DB_POOL_MAX 2
clever env set CC_HEALTH_CHECK_PATH /health
clever env set CC_CACHE_DEPENDENCIES true
clever env set CC_IGNORE_FROM_BUILDCACHE target/release/deps:target/release/build:target/release/.fingerprint:target/release/incremental

# 4. Deploy
clever deploy
clever open

# 5. Optional: the nine demo incidents (needs psql; reads the add-on URI with clever env, never prints it)
./scripts/reset-demo.sh
```

Turn the dedicated build on **before** the first `clever deploy` and check that `clever status` shows `Dedicated build: M`: without it, `cargo build` runs on the nano/XS app instance, pins its CPU at 100 % and the deployment stalls. The build instance is only billed while a build runs. Set the fixed `nano` size before the range: `--min-flavor` alone does not shrink an instance that already runs at XS. After the deploy, `clever status` shows `running (1*nano)`, `Scalers: 1 to 2`, `Sizes: nano to XS`.

The add-on must be linked before the first deploy: without a PostgreSQL URI the app exits at startup. Encryption at rest can only be chosen when the add-on is created, and the DEV plan offers neither encryption nor more than 5 connections. `xxs_sml` keeps daily backups (7 retained).

`scripts/reset-demo.sh` truncates the table and loads `scripts/demo-seed.sql`, with dates relative to now: 3 open, 2 investigating, 4 resolved, 3 flagged "SLA dépassé" with the default 30 minutes. Run it again just before a demo so the ages start from the present.

An app created this way deploys from its Clever Cloud git remote: redeploy with `clever deploy`, a push to GitHub does not trigger it.

## Variables

| Variable | Set by | Role |
|---|---|---|
| `POSTGRESQL_ADDON_URI` | PostgreSQL add-on (linked) | Connection string, read directly when `DATABASE_URL` is absent. Do not copy it into `DATABASE_URL` |
| `PORT` | Clever Cloud | Listening port (8080, also the local default) |
| `DB_POOL_MAX` | you | Pool size, default 2. Keep `DB_POOL_MAX × instances` below the add-on limit (`xxs_sml` = 45): a redeploy briefly doubles the instances, 4 × 2 = 8 |
| `CC_HEALTH_CHECK_PATH` | you | `/health`: the platform validates the deployment against PostgreSQL |
| `CC_CACHE_DEPENDENCIES` | you | `true`: reuses `target/` and the cargo registry between builds |
| `CC_IGNORE_FROM_BUILDCACHE` | you | Leaves the intermediate build files out of the build cache archive: the binary is enough to run |
| `CC_RUSTUP_CHANNEL` | optional | Pins the toolchain (default `stable`, e.g. `1.94.0`). `CC_RUST_VERSION` does not exist |
| `RUST_LOG` | optional | Log filter, default `incident_tracker=info,tower_http=info` |
| `APP_ENV` | optional, demo | Environment pill in the deployment console, Clever gradient when `production`. Default `local` |
| `TEAM_NAME` | optional, demo | "Astreinte : …" under the title. Default `Équipe SRE` |
| `SLA_CRITICAL_MINUTES` | optional, demo | Critical and high incidents still unresolved after this many minutes are flagged "SLA dépassé". Default `30` |
| `ANNOUNCEMENT` | optional, demo | Banner on every page when set |
| `DATABASE_URL` | you, local only | PostgreSQL connection string for a local run, takes precedence when set |

The deployment console shows `CC_APP_NAME`, `APP_ID`, `INSTANCE_NUMBER`, `INSTANCE_TYPE`, `CC_PRETTY_INSTANCE_NAME`, `CC_COMMIT_ID` (7 characters) and `CC_DEPLOYMENT_ID` (16 characters), and nothing else.

## Routes

| Method | Path | What it does |
|---|---|---|
| GET | `/` | Board and incident list (`?status=open`, `investigating` or `resolved`) |
| GET | `/incidents/new` | Form to declare an incident |
| POST | `/incidents` | Creates an incident (title ≤ 255, service ≤ 100, description ≤ 10 000 characters: a form error, never a 500) |
| GET | `/incidents/{id}` | Incident detail |
| POST | `/incidents/{id}/status` | Moves the incident to `open`, `investigating` or `resolved` |
| GET | `/stats` | Counts by status and severity |
| GET | `/health` | 200 when PostgreSQL answers `SELECT 1`, 503 otherwise |
| GET | `/cc-brand.css` | Clever Brand Kit stylesheet, embedded in the binary |

There is no delete. Every response carries `X-Content-Type-Options`, `Referrer-Policy`, `X-Frame-Options` and a `Content-Security-Policy`; TLS and HSTS stay on the Clever Cloud proxy.

## Code tour

```
src/main.rs              config, pool, migrations, router, security headers, graceful shutdown
src/handlers.rs          HTTP handlers, form validation, Askama templates
src/db.rs                SQLx queries: list (sorted by status then severity), create, update status, stats
src/models.rs            Incident (age, SLA check), Stats, Platform (injected and demo variables)
migrations/              0001_create_incidents.sql: the incidents table, run by sqlx::migrate!() at startup
templates/               base (topbar, banner, footer), index, new, detail, stats; partials/ holds the
                         Clever logo, the certification block and the platform panel
static/cc-brand.css      Clever Brand Kit, copied as is and embedded with include_str!
scripts/                 demo-seed.sql (9 incidents) and reset-demo.sh (reloads the linked add-on)
clevercloud/rust.json    detected at build time ("Configuration file detected"); the Rust runtime itself
                         is configured through environment variables
```

Axum 0.8, Tokio, SQLx 0.8 (PostgreSQL, rustls), Askama 0.12; Rust edition 2021, `rust-version` 1.85. Axum 0.8 path parameters use `{id}`: the former `:id` syntax panics at startup. The design follows `docs/superpowers/specs/2026-09-06-clever-brand-design.md`.

## Run locally

Needs Rust 1.85+ and a local PostgreSQL.

```bash
createdb demo_rust
DATABASE_URL=postgres://localhost/demo_rust PORT=8080 cargo run   # migrations run at startup
psql postgres://localhost/demo_rust -f scripts/demo-seed.sql      # optional: the nine demo incidents
# → http://localhost:8080
```

`cp .env.example .env` also works: the app reads a `.env` file when there is one (local only).
