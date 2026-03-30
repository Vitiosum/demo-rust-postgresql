# Incident Tracker — Rust + PostgreSQL on Clever Cloud

Demo application showing a Rust (Axum) web app connected to PostgreSQL, deployed on Clever Cloud.

## Features

- List, create, and update incidents
- Filter by status (open / investigating / resolved)
- Severity levels: low / medium / high / critical
- `/health` endpoint
- `/stats` page with incident counts

## Stack

| Layer     | Technology        |
|-----------|-------------------|
| Language  | Rust (edition 2021) |
| Web framework | Axum 0.7     |
| Database  | PostgreSQL         |
| DB access | SQLx 0.7           |
| Templates | Askama 0.12        |
| Runtime   | Tokio              |

---

## Local development

### Prerequisites

- Rust (stable, 1.75+)
- PostgreSQL running locally

### Setup

```bash
# Clone the repo
git clone <repo-url>
cd demo-rust-postgresql

# Copy and edit environment file
cp .env.example .env
# Edit DATABASE_URL with your local credentials

# Run (migrations are applied automatically at startup)
cargo run
```

App is available at `http://localhost:8080`.

---

## Environment variables

| Variable     | Description                                  | Default  |
|--------------|----------------------------------------------|----------|
| `DATABASE_URL` | PostgreSQL connection string               | required |
| `PORT`        | Listening port                              | `8080`   |
| `RUST_LOG`    | Log level (e.g. `incident_tracker=info`)   | `info`   |

On Clever Cloud, `PORT` and `POSTGRESQL_ADDON_URI` are injected automatically.

---

## Migrations

Migrations are applied automatically at startup via `sqlx::migrate!()`.
The SQL file is at `migrations/0001_create_incidents.sql`.

To run manually:
```bash
psql $DATABASE_URL -f migrations/0001_create_incidents.sql
```

---

## Deployment on Clever Cloud

### 1. Create a Rust application

In the Clever Cloud console:
- **New application** → **Rust**
- Connect your Git repository

### 2. Add a PostgreSQL add-on

- **Add-on** → **PostgreSQL** → link it to your app
- Enable **"Direct hostname and port"** on the add-on for `POSTGRESQL_ADDON_DIRECT_URI`

### 3. Set environment variables

In the app's **Environment variables** section:

```
DATABASE_URL = <value of POSTGRESQL_ADDON_URI or POSTGRESQL_ADDON_DIRECT_URI>
RUST_LOG     = incident_tracker=info,tower_http=info
```

`PORT` is injected automatically by Clever Cloud — do not set it manually.

### 4. Deploy

```bash
git push
```

Clever Cloud builds with `cargo build --release` and runs the `incident-tracker` binary.
Migrations are applied automatically at startup.

### Points of attention

- `DATABASE_URL` must be set before the first deploy (the app will crash on startup without it)
- Build time is ~3–5 minutes on first deploy (Rust compilation); subsequent deploys reuse the cache
- The binary listens on `0.0.0.0:$PORT` as required by Clever Cloud

---

## Endpoints

| Method | Path                       | Description            |
|--------|----------------------------|------------------------|
| GET    | `/`                        | List incidents          |
| GET    | `/incidents/new`           | Create form            |
| POST   | `/incidents`               | Create incident         |
| GET    | `/incidents/:id`           | Incident detail         |
| POST   | `/incidents/:id/status`    | Update status           |
| GET    | `/stats`                   | Statistics              |
| GET    | `/health`                  | Health check (200 OK)   |
