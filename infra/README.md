# Infra (OpenTofu, AWS mx-central-1)

The payments API runs on one small EC2 instance, with HTTPS in front from CloudFront:

```text
app ──HTTPS──▶ CloudFront ──HTTP:80──▶ EC2 t4g.small (Docker Compose)
                                          ├── postgres (postgres:17, data in /var/lib/pagacontufoto/pg on the root EBS volume)
                                          └── api  (Actix, from ECR; JSON under /v1, Amanda's HTML pages at /)
                                                ├── DynamoDB  pagacontufoto-pey-events (Pey charges and webhooks)
                                                └── Secrets Manager  pagacontufoto/pey (Pey API key)
```

- The instance has no SSH key and no port 22; only CloudFront's origin-facing servers can reach port 80. Shell access goes through SSM Session Manager.
- The API container gets AWS access from the instance role (IMDSv2, hop limit 2). There are no access keys on the machine.
- Amanda's schema lives in Postgres on the instance. The API runs its SQL migrations and seed files on start (embedded at compile time, see `crates/cuentas-api/migrations` and `seeds`).
- The Postgres password is generated on the instance at the first Postgres deploy (`/etc/pagacontufoto/db.env`, root-only) and never leaves it; the API gets `DATABASE_URL` from `/etc/pagacontufoto/api-db.env`.
- The table `pagacontufoto-payments` is no longer used by the API (payments are rows in Amanda's `pagos` table). It is kept until someone decides to remove it, and the instance role no longer has access to it.

## Layout

| Path | What |
| --- | --- |
| `bootstrap/` | S3 bucket for OpenTofu state (local state; apply once) |
| `*.tf` | Main stack (state in S3, with S3-native locking) |
| `user-data.sh.tftpl` | First boot: Docker, Compose, `/opt/pagacontufoto/docker-compose.yml`, systemd unit |
| `deploy.sh` | Build the image (arm64), push it to ECR, write the instance's compose file (Postgres + API) and logins via SSM, restart. `--rollback` goes back to the saved SQLite tag |
| `reset-demo.sh` | Asks first, then drops and recreates the `cuentas` database (migrations and seeds run again) and deletes the Pey charge items in DynamoDB |
| `.demo.env` | `DEMO_USUARIO`, `DEMO_CLAVE` (demo tallerista) and `ADMIN_CLAVE` (Amanda's admin login), generated locally, git-ignored, never printed |
| `.last-good-tag` | The last image tag that ran with SQLite, for `deploy.sh --rollback`; git-ignored |

## Apply

Requires OpenTofu 1.10+ and the AWS CLI profile `personal`.

```bash
cd infra/bootstrap && tofu init && tofu apply   # once
cd .. && tofu init && tofu plan && tofu apply
tofu output
```

## Deploy the API

From the repo root, with Docker running:

```bash
infra/deploy.sh            # tag = UTC timestamp, also pushes :latest
curl "$(cd infra && tofu output -raw api_url)/v1/salud"
```

The systemd unit `pagacontufoto.service` keeps retrying every 60 s until the image exists, so the first deploy starts the app on its own.

`deploy.sh` creates `infra/.demo.env` the first time (user `mariagonzalez` with a random 20-character password, plus a random `ADMIN_CLAVE`; file mode 600) and installs it on the instance as `/etc/pagacontufoto/demo.env`. With `SEED=1` the API uses it to create the demo tallerista and Amanda's admin login, whose passwords need an argon2 hash and so cannot live in the SQL seed files. The app reads the same file to configure its demo login. These passwords travel inside the SSM command, so they appear in the account's SSM command history; they only protect invented demo data.

API environment on the instance: `DATABASE_URL` (from `api-db.env`), `SEED=1`, `DEMO_USUARIO`, `DEMO_CLAVE`, `ADMIN_CLAVE`, `PEY_MODE` and `PEY_AMOUNT_OVERRIDE` (from `mode.env`), `PEY_SECRET_ID`, `PEY_EVENTS_TABLE`, `AWS_REGION`, `CURRENT_PERIOD=2026-12`, `PORT=8080`.

## Roll back

Before each deploy, `deploy.sh` saves the running tag in `.last-good-tag` if that deploy still used SQLite. To go back to it (SQLite compose, the old `cuentas.db` untouched in `/var/lib/pagacontufoto/data`):

```bash
infra/deploy.sh --rollback
```

Postgres data stays in `/var/lib/pagacontufoto/pg`; a later normal deploy picks it up again.

## Back up Postgres

```bash
aws ssm send-command --profile personal --region mx-central-1 \
  --instance-ids "$(cd infra && tofu output -raw instance_id)" --document-name AWS-RunShellScript \
  --parameters 'commands=["cd /opt/pagacontufoto && docker compose exec -T postgres pg_dump -U cuentas -d cuentas -Fc > /var/lib/pagacontufoto/cuentas-$(date -u +%Y%m%d%H%M%S).dump"]'
```

The dump stays on the instance's disk. For a full copy, take an EBS snapshot of the root volume (Console or `aws ec2 create-snapshot`).

## Reset the demo data

```bash
infra/reset-demo.sh   # asks for "yes"; drops and recreates the cuentas database and deletes the charge# items in DynamoDB
```

On start, the API runs the migrations and the seed files again. Webhook items stay in DynamoDB so a late duplicate from Pey is still ignored.

## Turn on real Pey payments

1. Put the Pey API key in Secrets Manager. Run this yourself and type the key only in your own terminal:

   ```bash
   aws secretsmanager put-secret-value --profile personal --region mx-central-1 \
     --secret-id pagacontufoto/pey --secret-string '<PEY_API_KEY>'
   ```

2. Switch the API to live mode and restart it (`/etc/pagacontufoto/mode.env` survives deploys):

   ```bash
   aws ssm send-command --profile personal --region mx-central-1 \
     --instance-ids "$(cd infra && tofu output -raw instance_id)" \
     --document-name AWS-RunShellScript \
     --parameters 'commands=["printf \"PEY_MODE=live\\nPEY_AMOUNT_OVERRIDE=1\\n\" > /etc/pagacontufoto/mode.env","cd /opt/pagacontufoto && cp /etc/pagacontufoto/mode.env .env && docker compose up -d --force-recreate api"]'
   ```

3. Register the webhook in Pey's dashboard: `tofu output -raw webhook_url` (that is `https://<cloudfront>/v1/webhooks/pey`).
4. `PEY_AMOUNT_OVERRIDE=1` makes every charge MXN 1 for testing; drop that line from `mode.env` to charge real amounts. Set `PEY_MODE=simulate` again the same way to go back.

## Look inside the instance

```bash
aws ssm start-session --profile personal --region mx-central-1 --target "$(cd infra && tofu output -raw instance_id)"
sudo docker compose -f /opt/pagacontufoto/docker-compose.yml logs -f api
sudo docker compose -f /opt/pagacontufoto/docker-compose.yml exec postgres psql -U cuentas -d cuentas
```

## Cost (on-demand, roughly, per month)

| Item | USD |
| --- | --- |
| EC2 t4g.small (24/7) | ~13–15 |
| EBS gp3 30 GB | ~2.5–3 |
| Public IPv4 address | ~3.6 |
| DynamoDB on-demand, Secrets Manager (1 secret), ECR (<1 GB) | ~1 |
| CloudFront (first 1 TB and 10 M requests are free tier) | ~0 |
| **Total** | **~20–23** |

A budget `pagacontufoto-monthly` (US$30) emails when actual spend passes 80% or the forecast passes 100%.

## Known limits

- The CloudFront origin is the instance's public DNS name. If the instance is stopped and started, the name changes and `tofu apply` must run again to update CloudFront. Rebooting keeps it.
- One instance, so no high availability. Postgres lives on the root volume; back up with `pg_dump` or an EBS snapshot.
- On the instance, `/var/lib/pagacontufoto/data` (Amanda's SQLite file, used by `--rollback`) and `/var/lib/pagacontufoto/postgres` (the first night's rewrite, a different schema) are kept but not used.
- `ami` and `user_data` changes are ignored, to avoid replacing the instance by accident. `user-data.sh.tftpl` describes the SQLite layout; on a rebuilt instance, run `deploy.sh` once to switch it to Postgres. Replace it on purpose with `tofu apply -replace=aws_instance.api`, which loses the SQLite data unless you snapshot the volume first.

## Next steps

- GitHub Actions deploys: needs an IAM OIDC provider and a deploy role, which were not created yet.
- Use a deploy IAM role instead of root credentials for the `personal` profile.

## Teardown

```bash
cd infra && tofu destroy   # the secret has a 7-day recovery window; the ECR repo must be empty first
```

The state bucket in `bootstrap/` is kept on purpose.
