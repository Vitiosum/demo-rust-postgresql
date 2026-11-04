#!/usr/bin/env bash
# Recharge la base PostgreSQL de l'app Clever liée (.clever.json) avec le jeu de démo.
# À lancer juste avant un tournage : les âges et les « SLA dépassé » repartent de maintenant.
# L'app est visée par son alias (défaut : demo-rust-postgresql, le nom créé par le README),
# sinon clever-tools refuse dès que plusieurs apps sont liées au dossier :
#   CLEVER_ALIAS=mon-alias ./scripts/reset-demo.sh
set -euo pipefail
cd "$(dirname "$0")/.."

alias_name="${CLEVER_ALIAS:-demo-rust-postgresql}"

uri="$(clever env -F json --alias "$alias_name" | python3 -c '
import json, sys
d = json.load(sys.stdin)
for addon in d.get("fromAddons", []):
    for v in addon.get("env", []):
        if v.get("name") == "POSTGRESQL_ADDON_URI":
            print(v["value"])
')"
[ -n "$uri" ] || { echo "POSTGRESQL_ADDON_URI introuvable pour l'alias $alias_name : app liée ? (clever link, CLEVER_ALIAS)" >&2; exit 1; }

psql -q -v ON_ERROR_STOP=1 "$uri" -f scripts/demo-seed.sql
psql -tA "$uri" -c "SELECT status || ' ' || count(*) FROM incidents GROUP BY status ORDER BY status"
