# 🧠 Claude.md — demo-rust-postgresql

## 🏛️ Posture et méthode d'exécution

Tu es un expert cloud senior, rigoureux, structuré et orienté exécution. Toute recommandation doit être pensée pour être durable, propre techniquement, et directement applicable dans le cloud sans blocage ni dépendance cachée.

---

## 🎯 Contexte du projet

Incident tracker CRUD en Rust avec Axum et SQLx.
L'utilisateur peut créer, lire, mettre à jour et supprimer des incidents avec titre, description, sévérité (low/medium/high/critical) et statut (open/investigating/resolved).
Conçue comme démo de déploiement sur **Clever Cloud**.

Déployée sur **Clever Cloud** (runtime Rust + add-on PostgreSQL).

---

## ☁️ Déploiement Clever Cloud

- **Type d'app** : Rust
- **Config** : `clevercloud/rust.json` → `appIsToBeBuilt: true`
- **Add-on requis** : PostgreSQL (lié à l'application)
- **Compilation** : Clever Cloud compile le Rust à chaque déploiement

### Variables d'environnement injectées automatiquement par Clever Cloud
| Variable | Description |
|---|---|
| `POSTGRESQL_ADDON_DIRECT_URI` | URI PostgreSQL avec accès direct |
| `POSTGRESQL_ADDON_URI` | URI PostgreSQL standard (fallback) |
| `PORT` | Port d'écoute |

---

## 🛠️ Stack

| Élément | Valeur |
|---|---|
| Rust | édition 2021 |
| Framework | Axum 0.8.x |
| Base de données | PostgreSQL via SQLx 0.8.x |
| Async runtime | Tokio 1.x |
| Templates | Askama 0.12.x |
| UUID | uuid 1.x |
| Dates | chrono 0.4.x |

---

## 📁 Structure clé

```
src/main.rs        → entry point, config Axum, routes
src/handlers.rs    → handlers HTTP (CRUD incidents)
src/models.rs      → structs Incident, CreateIncident, etc.
src/db.rs          → pool PostgreSQL, requêtes SQLx
templates/         → templates HTML Askama
migrations/        → migrations SQL (SQLx)
Cargo.toml         → dépendances Rust
clevercloud/rust.json → config déploiement Clever Cloud
```

---

## 🚀 Déployer une modification

```bash
git add .
git commit -m "description"
git push
```

Clever Cloud recompile et redéploie automatiquement après chaque push. La compilation Rust prend environ 3-5 minutes.

---

## ⚠️ Points de vigilance

- La compilation Rust sur Clever Cloud est longue (~3-5 min) — normal
- SQLx utilise des requêtes vérifiées à la compilation (`query_as!`) — si la DB est inaccessible localement, utiliser `SQLX_OFFLINE=true` ou `cargo build --no-default-features`
- Les migrations sont exécutées automatiquement au démarrage (`migrate!`)
- L'add-on PostgreSQL doit être lié **avant** le premier déploiement

---

## 🔍 Diagnostic rapide

| Symptôme | Cause probable | Correction |
|---|---|---|
| Crash au démarrage | Add-on PostgreSQL non lié | Lier l'add-on dans la console Clever Cloud |
| Erreur de compilation | Breaking change Axum/SQLx | Vérifier les logs de build Clever Cloud |
| Timeout au démarrage | Compilation trop longue | Normal — attendre la fin du build |
