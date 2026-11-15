#!/usr/bin/env bash
# Deploys the API to the instance through SSM.
#
#   ./deploy.sh [TAG]        build the image (arm64) from the repo root, push it to ECR and run it with Postgres
#   ./deploy.sh --rollback   run the tag saved in .last-good-tag again, with the SQLite compose it used before
#
# Postgres keeps its data in /var/lib/pagacontufoto/pg. Its password is generated on the instance the first time
# (/etc/pagacontufoto/db.env, root-only) and never leaves it. The old SQLite file in /var/lib/pagacontufoto/data is
# never deleted, so a rollback finds it as it was.
set -euo pipefail

PROFILE="${AWS_PROFILE:-personal}"
REGION="${AWS_REGION:-mx-central-1}"
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
DEMO_ENV="$HERE/.demo.env"
LAST_GOOD="$HERE/.last-good-tag"

MODE=deploy
if [ "${1:-}" = "--rollback" ]; then
  MODE=rollback
  shift
fi
TAG="${1:-$(date -u +%Y%m%d%H%M%S)}"

cd "$HERE"
REPO_URL="$(tofu output -raw ecr_repository_url)"
INSTANCE_ID="$(tofu output -raw instance_id)"
PEY_EVENTS_TABLE="$(tofu output -raw pey_events_table)"
PEY_SECRET_ID="$(tofu output -raw pey_secret_id)"
API_URL="$(tofu output -raw api_url)"
REGISTRY="${REPO_URL%%/*}"

random_secret() {
  LC_ALL=C tr -dc 'A-Za-z0-9' </dev/urandom | head -c "$1"
}

# Runs shell commands on the instance and prints their output. The parameters go through a private temp file so
# secrets never show in the local process list.
run_on_instance() {
  local comment="$1" params command_id
  shift
  params="$(mktemp)"
  chmod 600 "$params"
  python3 -I -c 'import json, sys; print(json.dumps({"commands": sys.argv[1:]}))' "$@" >"$params"
  command_id="$(aws ssm send-command --profile "$PROFILE" --region "$REGION" \
    --instance-ids "$INSTANCE_ID" \
    --document-name AWS-RunShellScript \
    --comment "$comment" \
    --parameters "file://$params" \
    --query Command.CommandId --output text)"
  rm -f "$params"
  aws ssm wait command-executed --profile "$PROFILE" --region "$REGION" \
    --command-id "$command_id" --instance-id "$INSTANCE_ID" || true
  aws ssm get-command-invocation --profile "$PROFILE" --region "$REGION" \
    --command-id "$command_id" --instance-id "$INSTANCE_ID" \
    --query '{status:Status,out:StandardOutputContent,err:StandardErrorContent}' --output json
}

# The compose file that ran before Postgres: one service, SQLite in /data.
compose_sqlite() {
  cat <<YAML
services:
  api:
    image: $REPO_URL:$1
    restart: unless-stopped
    ports:
      - "80:8080"
    volumes:
      - /var/lib/pagacontufoto/data:/data
    env_file: /etc/pagacontufoto/demo.env
    environment:
      DATABASE_URL: "sqlite:/data/cuentas.db?mode=rwc"
      PEY_MODE: "\${PEY_MODE:-simulate}"
      PEY_AMOUNT_OVERRIDE: "\${PEY_AMOUNT_OVERRIDE:-}"
      PEY_SECRET_ID: "$PEY_SECRET_ID"
      PEY_EVENTS_TABLE: "$PEY_EVENTS_TABLE"
      AWS_REGION: "$REGION"
      CURRENT_PERIOD: "2026-12"
      PORT: "8080"
      RUST_LOG: "info"
YAML
}

# Postgres next to the API. DATABASE_URL comes from /etc/pagacontufoto/api-db.env, written on the instance.
compose_postgres() {
  cat <<YAML
services:
  postgres:
    image: postgres:17
    restart: unless-stopped
    env_file: /etc/pagacontufoto/db.env
    volumes:
      - /var/lib/pagacontufoto/pg:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U cuentas -d cuentas"]
      interval: 5s
      timeout: 3s
      retries: 30
  api:
    image: $REPO_URL:$1
    restart: unless-stopped
    depends_on:
      postgres:
        condition: service_healthy
    ports:
      - "80:8080"
    env_file:
      - /etc/pagacontufoto/demo.env
      - /etc/pagacontufoto/api-db.env
    environment:
      SEED: "1"
      PEY_MODE: "\${PEY_MODE:-simulate}"
      PEY_AMOUNT_OVERRIDE: "\${PEY_AMOUNT_OVERRIDE:-}"
      PEY_SECRET_ID: "$PEY_SECRET_ID"
      PEY_EVENTS_TABLE: "$PEY_EVENTS_TABLE"
      AWS_REGION: "$REGION"
      CURRENT_PERIOD: "2026-12"
      PORT: "8080"
      RUST_LOG: "info"
YAML
}

# Prints the image tag the instance runs now and whether that compose uses SQLite.
running_compose() {
  run_on_instance "pagacontufoto read compose" \
    "grep -m1 'image: .*pagacontufoto-api' /opt/pagacontufoto/docker-compose.yml | sed 's/.*://'" \
    "grep -q 'sqlite:' /opt/pagacontufoto/docker-compose.yml && echo sqlite || echo postgres" |
    python3 -I -c 'import json, sys; print(" ".join(json.load(sys.stdin)["out"].split()))'
}

if [ "$MODE" = rollback ]; then
  [ -s "$LAST_GOOD" ] || { echo "No $LAST_GOOD to roll back to." >&2; exit 1; }
  TAG="$(tr -d '[:space:]' <"$LAST_GOOD")"
  COMPOSE="$(compose_sqlite "$TAG")"
  COMPOSE_B64="$(printf '%s\n' "$COMPOSE" | base64 | tr -d '\n')"
  run_on_instance "pagacontufoto rollback $TAG" \
    "set -euo pipefail" \
    "echo $COMPOSE_B64 | base64 -d > /opt/pagacontufoto/docker-compose.yml" \
    "aws ecr get-login-password --region $REGION | docker login --username AWS --password-stdin $REGISTRY >/dev/null" \
    "docker pull --quiet $REPO_URL:$TAG" \
    "/opt/pagacontufoto/start.sh" \
    "docker compose -f /opt/pagacontufoto/docker-compose.yml ps --format '{{.Name}} {{.Status}}'"
  echo "Rolled back to $TAG (SQLite). API: $API_URL/api/v1/health"
  exit 0
fi

if [ ! -f "$DEMO_ENV" ]; then
  umask 077
  printf 'DEMO_USUARIO=mariagonzalez\nDEMO_CLAVE=%s\n' "$(random_secret 20)" >"$DEMO_ENV"
  echo "Created $DEMO_ENV (demo login; not printed)."
fi
if ! grep -q '^ADMIN_CLAVE=' "$DEMO_ENV"; then
  printf 'ADMIN_CLAVE=%s\n' "$(random_secret 24)" >>"$DEMO_ENV"
  echo "Added ADMIN_CLAVE to $DEMO_ENV (not printed)."
fi
chmod 600 "$DEMO_ENV"

# Remember the last SQLite tag that worked, so --rollback has somewhere to go.
read -r RUNNING_TAG RUNNING_KIND <<<"$(running_compose)"
if [ "$RUNNING_KIND" = sqlite ] && [ -n "$RUNNING_TAG" ]; then
  printf '%s\n' "$RUNNING_TAG" >"$LAST_GOOD"
  echo "Saved $RUNNING_TAG as the rollback tag."
fi

aws ecr get-login-password --profile "$PROFILE" --region "$REGION" |
  docker login --username AWS --password-stdin "$REGISTRY"

docker build --platform linux/arm64 -t "$REPO_URL:$TAG" -t "$REPO_URL:latest" "$ROOT"
docker push "$REPO_URL:$TAG"
docker push "$REPO_URL:latest"

COMPOSE="$(compose_postgres "$TAG")"
COMPOSE_B64="$(printf '%s\n' "$COMPOSE" | base64 | tr -d '\n')"
DEMO_B64="$(base64 <"$DEMO_ENV" | tr -d '\n')"

run_on_instance "pagacontufoto deploy $TAG" \
  "set -euo pipefail" \
  "install -d -m 700 /etc/pagacontufoto" \
  "install -d -m 700 /var/lib/pagacontufoto/pg" \
  "umask 077 && echo $DEMO_B64 | base64 -d > /etc/pagacontufoto/demo.env" \
  "if [ ! -s /etc/pagacontufoto/db.env ]; then umask 077; pw=\$(openssl rand -hex 24); printf 'POSTGRES_USER=cuentas\nPOSTGRES_DB=cuentas\nPOSTGRES_PASSWORD=%s\n' \"\$pw\" > /etc/pagacontufoto/db.env; printf 'DATABASE_URL=postgres://cuentas:%s@postgres:5432/cuentas\n' \"\$pw\" > /etc/pagacontufoto/api-db.env; fi" \
  "chmod 600 /etc/pagacontufoto/db.env /etc/pagacontufoto/api-db.env" \
  "touch /etc/pagacontufoto/mode.env" \
  "echo $COMPOSE_B64 | base64 -d > /opt/pagacontufoto/docker-compose.yml" \
  "aws ecr get-login-password --region $REGION | docker login --username AWS --password-stdin $REGISTRY >/dev/null" \
  "docker pull --quiet $REPO_URL:$TAG" \
  "docker pull --quiet postgres:17" \
  "/opt/pagacontufoto/start.sh" \
  "docker compose -f /opt/pagacontufoto/docker-compose.yml ps --format '{{.Name}} {{.Status}}'"

echo "Deployed $TAG (Postgres). API: $API_URL/v1/salud"
