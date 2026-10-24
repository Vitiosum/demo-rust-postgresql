# 🧠 Claude.md — demo-rust-postgresql

## 🏛️ Posture et méthode d'exécution

Tu es un expert cloud senior, rigoureux, structuré et orienté exécution. Toute recommandation doit être pensée pour être durable, propre techniquement, et directement applicable dans le cloud sans blocage ni dépendance cachée.

---

## 🎯 Contexte du projet

Incident tracker CRUD en Rust avec Axum et SQLx.
L'utilisateur peut créer, lire et mettre à jour des incidents (pas de suppression) avec titre, description, sévérité (low/medium/high/critical) et statut (open/investigating/resolved).
Conçue comme démo de déploiement sur **Clever Cloud**.

Cible de déploiement : **Clever Cloud** (runtime Rust natif + add-on PostgreSQL, sans Dockerfile). Aucune application n'est en service actuellement : la procédure ci-dessous vaut pour une application à créer.

---

## ☁️ Déploiement Clever Cloud

- **Type d'app** : Rust
- **Build** : `cargo build --release --locked` (runtime Rust, `Cargo.lock` committé) ; `clevercloud/rust.json` est **détecté par la plateforme au build** (log « Configuration file detected »), mais la configuration du runtime Rust passe par les variables d'environnement
- **Add-on requis** : PostgreSQL (lié à l'application) — l'app lit `POSTGRESQL_ADDON_URI`, ne pas dupliquer dans `DATABASE_URL`
- **Plan PostgreSQL** : `xxs_sml` (plus petit plan dédié, 45 connexions, sauvegardes quotidiennes) **avec chiffrement au repos** (`--option encryption=true`, uniquement à la création) — jamais le plan DEV, qui n'offre pas le chiffrement
- **Scalabilité** : autoscalabilité nano → XS, 1 → 2 instances (`clever scale --flavor nano`, puis `--min-flavor nano --max-flavor XS --min-instances 1 --max-instances 2`)
- **Compilation** : Clever Cloud compile le Rust à chaque déploiement
- **Build dédié** : **obligatoire**, taille M, à activer **avant** le premier déploiement (`clever scale --build-flavor M` ou onglet Scalabilité de la console). Il est désactivé par défaut : sans lui, `cargo build` tourne sur l'instance XS, le CPU reste à 100 % et le déploiement bloque

### Variables d'environnement injectées automatiquement par Clever Cloud
| Variable | Description |
|---|---|
| `POSTGRESQL_ADDON_URI` | URI PostgreSQL de l'add-on lié (lue si `DATABASE_URL` est absente) |
| `PORT` | Port d'écoute |

### Variables à poser (console ou `clever env set`)
| Variable | Valeur | Rôle |
|---|---|---|
| `DB_POOL_MAX` | `2` | Taille du pool ; règle `DB_POOL_MAX × instances ≤ connexions du plan` (`xxs_sml` = 45, DEV = 5) |
| `RUST_LOG` | `incident_tracker=info,tower_http=info` | Filtre de logs (défaut intégré identique) |
| `CC_HEALTH_CHECK_PATH` | `/health` | La plateforme valide le déploiement contre PostgreSQL |
| `CC_CACHE_DEPENDENCIES` | `true` | Réutilise `target/` et le registre cargo d'un déploiement à l'autre (build bien plus court) |
| `CC_IGNORE_FROM_BUILDCACHE` | `target/release/deps:target/release/build:target/release/.fingerprint:target/release/incremental` | Allège l'archive de build (le binaire suffit au run ; les dépendances sont déjà dans le cache dédié) |

`CC_RUST_VERSION` n'existe pas (ignorée, le build prend la `stable` du jour) : pour épingler la toolchain, c'est `CC_RUSTUP_CHANNEL` (ex. `1.94.0`), au prix d'une installation rustup si la version n'est pas sur l'image.

### Variables de démo (à montrer dans l'onglet Variables de la console)
| Variable | Exemple | Effet visible |
|---|---|---|
| `APP_ENV` | `production` | Pastille d'environnement dans la console de la page (dégradé Clever si `production`) ; défaut `local` |
| `TEAM_NAME` | `SRE Paris` | « Astreinte : … » sous le titre ; défaut `Équipe SRE` |
| `SLA_CRITICAL_MINUTES` | `30` | Incidents critical/high non résolus au-delà de ce délai marqués « SLA dépassé » ; défaut `30` |
| `ANNOUNCEMENT` | `Maintenance ce soir à 22 h` | Bandeau en haut de toutes les pages ; absent = pas de bandeau |

Scénario de démo : modifier `ANNOUNCEMENT` ou `SLA_CRITICAL_MINUTES` dans la console, `clever restart`, recharger la page.

Avant un tournage : `./scripts/reset-demo.sh` recharge la base de l'app liée avec les 9 incidents de `scripts/demo-seed.sql` (dates relatives à l'instant présent : 3 open, 2 investigating, 4 resolved, 3 « SLA dépassé »).

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
- Structure de l'accueil (refonte vidéo, 2026-10) : topbar → tableau de crise (titre « N incidents en cours. », silhouette d'une barre par incident selon la sévérité, console de déploiement avec les variables Clever) → liste (liseré de sévérité, statut, âge relatif) → bloc certification Academy (`partials/cert.html`) → footer.
- Certification mise en avant partout : bouton « Se certifier Clever Cloud » dans le premier écran, `partials/cert.html` sur l'accueil et /stats, `partials/cert-mini.html` sur le détail. Le badge SVG porte des `id` : ne l'inclure qu'une fois par page.
- Couleurs de sévérité et de statut : variables `--sev-*` / `--st-*` dans `base.html`, appliquées par les classes `is-<severity>` / `is-<status>` (valeurs validées côté serveur).
- Textes du chrome en français ; libellés techniques (`open`, `critical`…) en anglais.
- Spec : `docs/superpowers/specs/2026-09-06-clever-brand-design.md`.

---

## 📁 Structure clé

```
src/main.rs        → entry point, config Axum, routes (+ route /cc-brand.css)
src/handlers.rs    → handlers HTTP (CRUD incidents), structs de templates (champ `cc`)
src/models.rs      → structs Incident, CreateIncidentForm, Platform (variables Clever Cloud)
src/db.rs          → pool PostgreSQL, requêtes SQLx
templates/         → templates HTML Askama (base, index, new, detail, stats)
templates/partials/ → SVG inline (logo, badge) + panneau plateforme
static/cc-brand.css → Clever Brand Kit (copie, ne pas modifier)
migrations/        → migrations SQL (SQLx)
Cargo.toml         → dépendances Rust
.cargo/audit.toml  → config cargo-audit (RUSTSEC-2023-0071 ignoré : rsa hors graphe compilé)
clevercloud/rust.json → détecté par la plateforme au build (« Configuration file detected »)
```

---

## 🚀 Déployer une modification

```bash
git add .
git commit -m "description"
git push
```

Une fois l'application créée et le dépôt lié, Clever Cloud recompile et redéploie automatiquement après chaque push. La compilation Rust prend environ 1-2 minutes sur le build dédié M (mesuré : 1 min 16 s pour un premier build).

Si l'application a été créée avec `clever create` (remote git Clever, pas de lien GitHub), le push GitHub ne redéploie pas : redéployer avec `clever deploy`.

---

## ⏱️ Durée de déploiement (mesurée le 2026-10-25, build dédié M)

| Déploiement | Total | `cargo build` |
|---|---|---|
| Premier (aucun cache) | 128 s | 60 s, 181 crates |
| `CC_CACHE_DEPENDENCIES=true`, premier passage (crée le cache) | 106 s | 39 s |
| Cache de dépendances réutilisé + features réduites | 80 s | 12,7 s, seule l'app recompilée |
| Changement de variable + `clever restart` (pas de build) | 35 s | — |

Incompressibles : démarrage des VM de build et de run (~15 s chacune), health check (~3 s). L'app démarre en < 100 ms (pool + migrations).

## ⚠️ Points de vigilance

- **Axum 0.8** : les paramètres de route s'écrivent `/incidents/{id}` — l'ancienne syntaxe `:id` fait **paniquer l'app au démarrage**
- Build dédié M obligatoire (désactivé par défaut) : sans lui, la compilation sature l'instance XS et le déploiement bloque
- SQLx est utilisé en mode runtime (`sqlx::query_as::<_, T>()`, pas de macro `query!`) : aucune base ni `SQLX_OFFLINE` nécessaire au build
- Pool PostgreSQL : `DB_POOL_MAX` (défaut 2) × instances doit rester sous la limite du plan (`xxs_sml` = 45) — un redéploiement double brièvement les instances (jusqu'à 4 avec 2 max : 4 × 2 = 8)
- Arrêt propre sur SIGTERM : les requêtes en vol sont terminées, le pool fermé (`shutdown_signal()` dans `src/main.rs`)
- Validation serveur : title ≤ 255, service ≤ 100, description ≤ 10 000 caractères (erreur de formulaire, jamais de 500)
- Les migrations sont exécutées automatiquement au démarrage (`migrate!`)
- L'add-on PostgreSQL doit être lié **avant** le premier déploiement

---

## 🔍 Diagnostic rapide

| Symptôme | Cause probable | Correction |
|---|---|---|
| Crash au démarrage | Add-on PostgreSQL non lié | Lier l'add-on dans la console Clever Cloud |
| `/health` répond 503 | PostgreSQL injoignable ou pool saturé | Vérifier l'add-on, baisser `DB_POOL_MAX` |
| Panique au démarrage (`Path segments must not start with ':'`) | Route Axum 0.7 (`:id`) | Utiliser `{id}` (Axum 0.8) |
| Page sans style | Route `/cc-brand.css` absente | Vérifier `brand_css` dans `src/main.rs` |
| Erreur de compilation | Breaking change Axum/SQLx | Vérifier les logs de build Clever Cloud |
| Déploiement bloqué, CPU à 100 % pendant le build | Build dédié désactivé (défaut), compilation sur l'instance XS | `clever scale --build-flavor M`, puis `clever cancel-deploy` et `clever deploy` |
