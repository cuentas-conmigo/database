#!/usr/bin/env bash
# Resets the demo data: drops and recreates the `cuentas` Postgres database on the instance (on start, the API runs
# the migrations and the seed files again) and deletes the Pey charge items in DynamoDB, whose keys reuse payment ids.
# Webhook items are kept. Run it yourself before a demo; it asks before deleting and cannot be undone.
set -euo pipefail

PROFILE="${AWS_PROFILE:-personal}"
REGION="${AWS_REGION:-mx-central-1}"
HERE="$(cd "$(dirname "$0")" && pwd)"

cd "$HERE"
INSTANCE_ID="$(tofu output -raw instance_id)"
TABLE="$(tofu output -raw pey_events_table)"

read -r -p "Drop the demo database on $INSTANCE_ID and delete the charge items in $TABLE? Type yes: " answer
[ "$answer" = "yes" ] || { echo "Nothing deleted."; exit 0; }

COMMAND_ID="$(aws ssm send-command --profile "$PROFILE" --region "$REGION" \
  --instance-ids "$INSTANCE_ID" --document-name AWS-RunShellScript \
  --comment "pagacontufoto reset demo" \
  --parameters 'commands=["set -euo pipefail","cd /opt/pagacontufoto","docker compose stop api","docker compose exec -T postgres psql -U cuentas -d postgres -v ON_ERROR_STOP=1 -c \"DROP DATABASE IF EXISTS cuentas WITH (FORCE)\" -c \"CREATE DATABASE cuentas OWNER cuentas\"","docker compose up -d api"]' \
  --query Command.CommandId --output text)"
aws ssm wait command-executed --profile "$PROFILE" --region "$REGION" \
  --command-id "$COMMAND_ID" --instance-id "$INSTANCE_ID" || true
aws ssm get-command-invocation --profile "$PROFILE" --region "$REGION" \
  --command-id "$COMMAND_ID" --instance-id "$INSTANCE_ID" --query Status --output text

ids="$(aws dynamodb scan --profile "$PROFILE" --region "$REGION" --table-name "$TABLE" \
  --projection-expression eventId --filter-expression 'begins_with(eventId, :p)' \
  --expression-attribute-values '{":p":{"S":"charge#"}}' --query 'Items[].eventId.S' --output text)"
count=0
[ "$ids" = None ] && ids=""
for id in $ids; do
  aws dynamodb delete-item --profile "$PROFILE" --region "$REGION" --table-name "$TABLE" \
    --key "{\"eventId\":{\"S\":\"$id\"}}"
  count=$((count + 1))
done
echo "Database reset; deleted $count charge items from $TABLE."
