# Design Spec — « Clever Brand Kit » pour les six démos clients

**Date :** 2026-09-06
**Périmètre :** demo-php-frankenphp, demo-rust-postgresql, demo-nodejs-postgresql, demo-dotnet-blazor, demo-react-vite, demo-go
**Statut :** proposé, exécuté sous hypothèses explicites (mode autonome), branches locales non poussées

---

## 1. Besoin reformulé

Rafraîchir six démos d'avant-vente pour qu'elles :

1. mettent en avant Clever Cloud au maximum (marque, plateforme, ce qu'elle fait pour l'app) ;
2. mettent en avant **la certification Clever Cloud** (academy.clever.cloud) comme élément central, pas comme lien de bas de page ;
3. soient visuellement plus abouties que l'existant ;
4. restent simples à builder et à déployer sur Clever Cloud (aucune dépendance nouvelle côté runtime).

## 2. État constaté (2026-09-06)

| Démo | Runtime CC | Design actuel | Constat |
|---|---|---|---|
| demo-php-frankenphp | Docker (FrankenPHP + Symfony 7) | « Stellar.ai » fond blanc, Inter | Landing générique, certification = petit badge |
| demo-rust-postgresql | Rust (Axum 0.8 + SQLx) + PG | « Track Night » orange | **Bug bloquant** : routes `:id` → panique au démarrage avec Axum 0.8 (corrigé en `{id}`) |
| demo-nodejs-postgresql | Node (Express 5 + Pug) + PG | « Track Night » orange | Fonctionnel, certification en bas de page |
| demo-dotnet-blazor | .NET 8 Blazor Server | « Track Night » + sidebar Bootstrap violette | Sidebar Bootstrap par défaut, incohérent |
| demo-react-vite | Static (React 18 + Vite + Tailwind 4) | « Nexus AI » bleu | RunRank, aucune identité Clever à part le footer |
| demo-go | Go stdlib | « Aura » orbes bleu/violet, marquee | Dashboard runtime, esthétique « startup IA » |

Quatre chartes différentes, aucune n'utilise le logo ni les couleurs Clever Cloud. Toutes les apps Clever correspondantes sont **arrêtées** (état vérifié par `clever status`).

## 3. Options écartées

- **Option A — retoucher chaque charte existante** : garde quatre identités différentes, ne répond pas à « un maximum Clever Cloud ». Écartée.
- **Option B — dépendre d'un package CSS partagé (npm/CDN)** : ajoute une dépendance réseau ou un build à des runtimes (Go, Rust, PHP) qui n'en ont pas. Écartée : chaque dépôt doit rester autonome et déployable tel quel.
- **Option C (retenue) — un « Clever Brand Kit » copié dans chaque dépôt** : un fichier CSS unique (tokens + composants), le logo SVG inline, trois blocs communs (barre de marque, bloc certification, pied de page plateforme). Zéro dépendance ajoutée, un seul système visuel, chaque dépôt reste standalone.

## 4. Système visuel « Clever Brand Kit »

Source : tokens réels de clever.cloud (feuille de style du site, logo officiel `logo_on_dark.svg`).

**Typographie** : Plus Jakarta Sans (police du site clever.cloud, Google Fonts, 300–800, italic) pour tout le texte ; JetBrains Mono pour les valeurs techniques (IDs, métriques, code). Repli système : `system-ui, sans-serif` / `ui-monospace`.

**Couleurs** (tokens CSS `--cc-*`) :

| Token | Valeur | Usage |
|---|---|---|
| `--cc-bg` | `#13172e` (neutral-black) | fond de page |
| `--cc-surface` | `#1c2045` (tint-black) | cartes, panneaux |
| `--cc-surface-2` | `#242a55` | cartes surélevées, hover |
| `--cc-border` | `rgba(222,221,238,0.14)` | bordures |
| `--cc-text` | `#f9f9fb` (greylight) | texte principal |
| `--cc-text-2` | `#deddee` (purplegrey) | texte secondaire |
| `--cc-muted` | `#9a9ab8` (grey éclairci pour contraste AA sur navy) | libellés |
| `--cc-primary` | `#a51050` (cherry) | accent principal |
| `--cc-red` | `#cb1c42` | accent |
| `--cc-orange` | `#f57461` | accent chaud, points « live » |
| `--cc-gradient` | `linear-gradient(90deg,#f57461,#cb1c42 50.48%,#a51050)` | CTA, texte dégradé, badge |
| `--cc-purple` | `#5754aa` | liens secondaires, focus |
| `--cc-green` | `#11bea9` (product-green) | états « ok » |

Thème unique sombre (navy). Justification : c'est le rendu des sections héro du site clever.cloud et de ses supports, il projette bien en démo, et il reste cohérent avec l'existant (cinq démos sur six déjà sombres).

**Formes** : rayons 12 px (cartes) / 999 px (pills), bordure 1 px, ombre douce `0 12px 40px rgba(0,0,0,.35)`. Pas d'orbes flous, pas de marquee, pas de bordure conique animée : un seul mouvement discret (point « live » qui pulse, fade-in 400 ms à l'apparition).

## 5. Blocs communs (dans chaque démo)

1. **Barre de marque** (sticky, fond navy 85 % + blur) : logo Clever Cloud officiel (SVG inline, wordmark blanc + icône dégradée) · nom de la démo · pill runtime (ex. « Node.js 20 · PostgreSQL ») · pill « Live on Clever Cloud » (point orange pulsant) · lien « Se certifier » (dégradé) · lien Console.
2. **Héro** : titre en 2 lignes avec le mot-clé en texte dégradé (ex. « Node.js + PostgreSQL, *déployé en un push* ») · sous-titre nommant runtime + add-on · rangée de 3–4 pills « ce que Clever fait » (build automatique, add-on lié, scaling, HTTPS + domaine).
3. **Bloc Certification** (juste sous le héro, largeur pleine) : à gauche un **badge** SVG (anneau dégradé + logo) ; à droite « Certifié Clever Cloud » + une phrase + deux cartes de parcours (« Cloud Computing Fundamentals », « Advanced Deployment ») + CTA dégradé « Obtenir ma certification → academy.clever.cloud ». Reprend la mention réelle de l'Academy : badges numériques délivrés automatiquement.
4. **Panneau plateforme** (« Vu depuis Clever Cloud ») : lit les variables injectées `CC_APP_NAME`, `APP_ID`, `INSTANCE_NUMBER`, `INSTANCE_TYPE`, `CC_PRETTY_INSTANCE_NAME`, `CC_COMMIT_ID`, `CC_DEPLOYMENT_ID` et affiche « Local (hors Clever Cloud) » quand elles sont absentes. Pour React (statique, pas d'env serveur) : panneau explicatif du runtime Static à la place.
5. **Pied de page** : liens doc runtime, doc add-on, Console, Academy, dépôt GitHub · rappel « Hébergé en France · ISO 27001 · HDS » (mentions présentes sur clever.cloud).

## 6. Contenu propre à chaque démo (conservé, restylé)

| Démo | Fonction conservée | Ajustements |
|---|---|---|
| Node + PG | liste clé/valeur (ajout/suppression) | version PostgreSQL et nombre de lignes affichés ; suppression du bouton `/prime` de l'UI (reste en route) |
| Rust + PG | incident tracker (liste, filtre, création, détail, stats) | correction des routes Axum 0.8 ; 4 templates restylés ; stats en cartes |
| React Vite | RunRank (allure + rang) | chrome Clever autour du calculateur ; rebuild committé (racine) |
| Blazor | Home + Counter | suppression de la sidebar Bootstrap ; layout à barre de marque ; indicateur de connexion SignalR |
| Go | dashboard runtime (polling 2 s) | mêmes 6 métriques, cartes brand kit ; panneau plateforme |
| PHP FrankenPHP | landing + onglets + Swagger UI | contenu conservé, restylé ; certification remontée sous le héro |

## 7. Contraintes techniques respectées

- Aucune dépendance ajoutée aux runtimes ; polices via Google Fonts avec repli système.
- Le CSS partagé est **copié** dans chaque dépôt (`/public`, `/wwwroot`, `/static`, constante Go, template Askama), pas référencé entre dépôts.
- Le logo est embarqué en SVG inline (pas de requête vers cdn.clever-cloud.com).
- Ports / variables / fichiers `clevercloud/*.json` inchangés.
- Vérification locale avant commit : chaque démo lancée, capturée, console sans erreur.
- Commits sur une branche `redesign/clever-brand` par dépôt ; **aucun push** sans validation.

## 8. Tests / vérification

- Chaque démo démarre en local et répond `/` (et `/health` quand il existe).
- Capture d'écran desktop 1280 px et mobile 375 px sans débordement horizontal.
- Console navigateur vide d'erreurs.
- Rust : `cargo build` sans avertissement bloquant ; Node : `npm start` ; Blazor : `dotnet build` ; Go : `go vet` ; PHP : `php -l` sur les templates rendus + page 200 ; React : `npm run build` et rendu du build.

## 9. Résultat attendu

Six démos avec une seule identité Clever Cloud reconnaissable, la certification visible sans scroller, un panneau qui montre en direct ce que la plateforme injecte, et un code plus propre (bug Rust corrigé, sidebar Bootstrap retirée).
