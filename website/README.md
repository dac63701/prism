# Prism Cloud Server

Cloud sharing backend for the Prism game clipping app. Serves a Next.js dashboard + public player page with an Axum API.

## Quick Start

```bash
# Edit .env with your JWT_SECRET
cp .env.example .env

# Start Postgres + API + web with a freshly built frontend bundle
docker compose up --build -d

# Open dashboard
open http://localhost:3000
```

For a bundled local run without Docker, use:

```bash
make serve
```

The first registered user is **not** automatically admin. To promote a user:

```bash
docker compose exec postgres psql -U prism -d prism -c "UPDATE users SET role = 'admin' WHERE email = 'admin@example.com';"
```

## Configuration

| Variable | Default | Description |
|---|---|---|
| `DATABASE_URL` | — | Postgres connection string (required) |
| `JWT_SECRET` | — | 32+ char random string (required) |
| `SERVER_HOST` | `0.0.0.0` | Bind address |
| `SERVER_PORT` | `8080` | Bind port |
| `STORAGE_PATH` | `/data` | Where clips/thumbnails are stored |
| `MAX_UPLOAD_SIZE_MB` | `500` | Per-file upload limit |
| `DEFAULT_MAX_STORAGE_GB` | `10` | Default per-user storage quota |
| `RATE_LIMIT_REQUESTS_PER_MIN` | `100` | Global rate limit |
| `SITE_URL` | `http://localhost:3000` | Public URL for OG meta tags |
| `GOOGLE_CLIENT_ID` | — | Google OAuth client ID |
| `GOOGLE_CLIENT_SECRET` | — | Google OAuth client secret |
| `GOOGLE_REDIRECT_URI` | `http://localhost:8080/api/auth/google/callback` | Google OAuth callback |
| `DESKTOP_SCHEME_URL` | `prism://auth/callback` | Desktop app callback scheme |
| `API_IMAGE` | Pinned production API digest | Production API image reference |
| `WEB_IMAGE` | Pinned production web digest | Production web image reference |

## Production Deployment (Portainer)

```bash
# Portainer should use the pinned Docker Hub image digests in docker-compose.prod.yml.
# Set Docker Hub secrets in GitHub Actions:
#   DOCKER_USERNAME, DOCKER_PASSWORD
# GitHub Actions now has separate workflows for PR validation, main-branch publish,
# cache warming, and cache cleanup.
# Set runtime env vars in Portainer:
#   JWT_SECRET, POSTGRES_DB, POSTGRES_USER, POSTGRES_PASSWORD, DATABASE_URL,
#   SERVER_HOST, SERVER_PORT, STORAGE_PATH, SITE_URL, API_ORIGIN,
#   GOOGLE_CLIENT_ID, GOOGLE_CLIENT_SECRET, GOOGLE_REDIRECT_URI,
#   DESKTOP_SCHEME_URL, MAX_UPLOAD_SIZE_MB, DEFAULT_MAX_STORAGE_GB,
#   RATE_LIMIT_REQUESTS_PER_MIN, RUST_LOG, API_IMAGE, WEB_IMAGE
```

Set `DATABASE_URL` to use the in-stack hostname, not `localhost`:

```text
postgres://<user>:<url-encoded-password>@postgres:5432/<database>
```

`API_IMAGE` and `WEB_IMAGE` must be immutable `@sha256:` references for a
specific release. Do not deploy production with a mutable `latest` tag.

### Recovering a pre-migration-checksum database

Early Prism images changed the already-applied SQLx migration `001`. Existing
Postgres volumes created before that release cannot start newer API images until
their known migration checksum is reconciled.

1. Stop `api` and `web`, then take and verify a Postgres backup and a copy of the `prism-data` volume.
2. Inspect migration `001` before changing anything:

```sql
SELECT version, success, encode(checksum, 'hex') AS checksum
FROM _sqlx_migrations
WHERE version = 1;
```

3. Only when it reports the known legacy checksum, run `scripts/reconcile-sqlx-001-checksum.sql` against the database from a trusted shell.
4. Redeploy a pinned API image built from this release. Migration `006` then normalizes the legacy `share_id` default.

The recovery script is intentionally guarded: it changes exactly one successful
`001` row with the known legacy checksum, or rolls back. Never delete
`_sqlx_migrations` and never recreate a production database volume to bypass a
migration error.

Nginx must also join `nginx-network`. Use Docker's resolver so Nginx can survive
an API or web container restart instead of resolving the upstream only at boot:

```nginx
server {
    listen 443 ssl;
    server_name goprism.studio;

    resolver 127.0.0.11 valid=10s ipv6=off;

    location /api/ {
        set $api_upstream api:8080;
        proxy_pass http://$api_upstream;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        client_max_body_size 1000M;
    }

    location / {
        set $web_upstream web:3000;
        proxy_pass http://$web_upstream;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_read_timeout 300s;
        proxy_send_timeout 300s;
        client_max_body_size 1000M;
    }
}
```

## Development

```bash
# Bundled local server (starts Postgres, builds frontend, runs backend)
make serve

# Hot-reload backend only
cargo run

# Hot-reload frontend only
cd frontend && npm run dev
```

## API Overview

All endpoints under `/api/`.

### Auth
- `POST /api/auth/register` — create account
- `POST /api/auth/login` — get JWT tokens
- `POST /api/auth/refresh` — rotate tokens
- `GET /api/auth/me` — current user
- `POST /api/auth/change-password`
- `POST /api/auth/update-profile`

### API Keys
- `GET /api/auth/api-keys` — list keys
- `POST /api/auth/api-keys` — generate key
- `DELETE /api/auth/api-keys/{id}` — revoke

### Clips
- `POST /api/clips/upload` — multipart upload (JWT or API key)
- `GET /api/clips` — list (paginated, searchable)
- `GET /api/clips/{id}` — detail (includes tags)
- `PATCH /api/clips/{id}` — update title/game/visibility
- `DELETE /api/clips/{id}` — delete
- `POST /api/clips/{id}/regenerate-share` — new share ID
- `GET /api/clips/{id}/tags` — list tags
- `PUT /api/clips/{id}/tags` — set tags

### Public
- `GET /s/{shareId}` — player page (HTML+OG tags)
- `GET /api/s/{shareId}/meta` — clip metadata
- `GET /api/media/{*path}` — serve stored files

### Admin (requires admin role)
- `GET /api/admin/users`
- `GET /api/admin/users/{id}`
- `PATCH /api/admin/users/{id}` — role/ban/storage
- `DELETE /api/admin/users/{id}`
- `GET /api/admin/stats`
- `GET /api/admin/clips`
- `DELETE /api/admin/clips/{id}`
- `GET /api/admin/logs`
- `GET /api/admin/config` — current effective config
- `PUT /api/admin/config` — update runtime config

### Health
- `GET /api/health`

## Architecture

```
Browser ──HTTPS──▶ Nginx ──▶ web (Next.js, port 3000)
                        │
                        └── /api/* ──▶ api (Axum, port 8080) ──▶ PostgreSQL
                                                                   │
                                                                   └── /data/clips/ (local storage)
```

- Nginx routes `/api/*` directly to the Axum API server, bypassing the Next.js proxy for reliability after container/network resets.
- Next.js serves the dashboard and public pages; the Next.js rewrite fallback is only used when running without nginx (dev mode).
- JWT tokens (15m access + 30d refresh) for web auth
- API keys (SHA-256 hashed) for desktop app auth
- Rate limiting via token bucket middleware
- Server config stored in DB, overridable at runtime via admin API
