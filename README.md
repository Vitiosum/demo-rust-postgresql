# Incident Tracker — Rust + PostgreSQL on Clever Cloud

> A Rust (Axum) incident management app connected to PostgreSQL, deployed on Clever Cloud's Rust runtime. Demonstrates async Rust web development with automatic database migrations.

---

## Deploy on Clever Cloud

1. Fork this repository
2. In the Clever Cloud console, create a new **Rust** application — connect your forked repo
3. Add a **PostgreSQL** add-on and link it to your app
4. Set the `DATABASE_URL` environment variable to the value of `POSTGRESQL_ADDON_URI` (or `POSTGRESQL_ADDON_DIRECT_URI`)
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
| Templates     | Askama              |
| Design        | Track Night (Bebas Neue, orange #FF5A1F, dark background) |

---

## Features

- List, create, and update incidents
- Filter by status: open / investigating / resolved
- Severity levels: low / medium / high / critical
- Automatic database migrations at startup (`sqlx::migrate!()`)
- `/health` endpoint (200 OK)
- `/stats` page with incident counts

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

---

## Environment Variables

| Variable       | Required | Description                                             |
|----------------|----------|---------------------------------------------------------|
| `DATABASE_URL` | ✅       | PostgreSQL connection string                            |
| `PORT`         | auto     | Injected by Clever Cloud (default: 8080)                |
| `RUST_LOG`     | —        | Log level, e.g. `incident_tracker=info,tower_http=info` |

---

## Endpoints

| Method | Path                    | Description            |
|--------|-------------------------|------------------------|
| GET    | `/`                     | List incidents          |
| GET    | `/incidents/new`        | Create form             |
| POST   | `/incidents`            | Create incident         |
| GET    | `/incidents/:id`        | Incident detail         |
| POST   | `/incidents/:id/status` | Update status           |
| GET    | `/stats`                | Statistics              |
| GET    | `/health`               | Health check (200 OK)   |

---

## Deployment Notes

- `DATABASE_URL` must be set before the first deploy — the app will crash on startup without it
- Migrations are applied automatically at startup via `sqlx::migrate!()` — no manual migration step needed
- The binary listens on `0.0.0.0:$PORT` as required by Clever Cloud
- First build is slow (~3–5 min) — Clever Cloud caches compiled artifacts for subsequent deploys
