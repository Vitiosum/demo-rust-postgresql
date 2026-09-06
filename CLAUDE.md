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
| Design | Clever Brand Kit (Plus Jakarta Sans, navy #13172e, dégradé Clever) |

---

## 🎨 Design — Clever Brand Kit

- `static/cc-brand.css` : copie **telle quelle** du kit partagé (tokens `--cc-*`, composants `.cc-*`). **Ne jamais le modifier** ; les styles propres à la démo vont dans le `<style>` court de `templates/base.html`.
- Le CSS est embarqué dans le binaire (`include_str!`) et servi sur `/cc-brand.css` — aucune dépendance ajoutée (pas de `tower-http/fs`).
- Logo et badge certification : SVG inline via `templates/partials/cc-logo.html` et `cc-badge.html`. Ces partials ne doivent contenir **ni `{{` ni `{%`** (Askama les parse).
- Chaque struct `#[derive(Template)]` reçoit `cc: Platform` (`models::Platform::from_env()`) ; dans les templates : `{% if cc.live %}`, `{{ cc.app_name }}`.
- Structure de page (identique à la référence du kit) : topbar → héro → bloc certification (accueil uniquement) → contenu + panneau « Vu depuis Clever Cloud » → « Ce que Clever Cloud fait » → footer.
- Textes du chrome en français ; libellés techniques (`open`, `critical`…) en anglais.
- Spec : `docs/superpowers/specs/2026-09-06-clever-brand-design.md`.

---

## 📁 Structure clé

```
src/main.rs        → entry point, config Axum, routes (+ route /cc-brand.css)
src/handlers.rs    → handlers HTTP (CRUD incidents), structs de templates (champ `cc`)
src/models.rs      → structs Incident, CreateIncident, Platform (variables Clever Cloud)
src/db.rs          → pool PostgreSQL, requêtes SQLx
templates/         → templates HTML Askama (base, index, new, detail, stats)
templates/partials/ → SVG inline (logo, badge) + panneau plateforme
static/cc-brand.css → Clever Brand Kit (copie, ne pas modifier)
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

- **Axum 0.8** : les paramètres de route s'écrivent `/incidents/{id}` — l'ancienne syntaxe `:id` fait **paniquer l'app au démarrage**
- La compilation Rust sur Clever Cloud est longue (~3-5 min) — normal
- SQLx utilise des requêtes vérifiées à la compilation (`query_as!`) — si la DB est inaccessible localement, utiliser `SQLX_OFFLINE=true` ou `cargo build --no-default-features`
- Les migrations sont exécutées automatiquement au démarrage (`migrate!`)
- L'add-on PostgreSQL doit être lié **avant** le premier déploiement

---

## 🔍 Diagnostic rapide

| Symptôme | Cause probable | Correction |
|---|---|---|
| Crash au démarrage | Add-on PostgreSQL non lié | Lier l'add-on dans la console Clever Cloud |
| Panique au démarrage (`Path segments must not start with ':'`) | Route Axum 0.7 (`:id`) | Utiliser `{id}` (Axum 0.8) |
| Page sans style | Route `/cc-brand.css` absente | Vérifier `brand_css` dans `src/main.rs` |
| Erreur de compilation | Breaking change Axum/SQLx | Vérifier les logs de build Clever Cloud |
| Timeout au démarrage | Compilation trop longue | Normal — attendre la fin du build |
