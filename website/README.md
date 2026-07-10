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
| `STORAGE_PATH` | `./data` | Where clips/thumbnails are stored |
| `MAX_UPLOAD_SIZE_MB` | `500` | Per-file upload limit |
| `DEFAULT_MAX_STORAGE_GB` | `10` | Default per-user storage quota |
| `RATE_LIMIT_REQUESTS_PER_MIN` | `100` | Global rate limit |
| `SITE_URL` | `http://localhost:3000` | Public URL for OG meta tags |
| `GOOGLE_CLIENT_ID` | — | Google OAuth client ID |
| `GOOGLE_CLIENT_SECRET` | — | Google OAuth client secret |
| `GOOGLE_REDIRECT_URI` | `http://localhost:8080/api/auth/google/callback` | Google OAuth callback |
| `DESKTOP_SCHEME_URL` | `prism://auth/callback` | Desktop app callback scheme |

## Production Deployment (Portainer)

```bash
# Portainer should use the prebuilt Docker Hub images from docker-compose.prod.yml.
# Set Docker Hub secrets in GitHub Actions:
#   DOCKER_USERNAME, DOCKER_PASSWORD
# Set runtime env vars in Portainer:
#   JWT_SECRET, POSTGRES_DB, POSTGRES_USER, POSTGRES_PASSWORD, DATABASE_URL,
#   SERVER_HOST, SERVER_PORT, STORAGE_PATH, SITE_URL, API_ORIGIN,
#   GOOGLE_CLIENT_ID, GOOGLE_CLIENT_SECRET, GOOGLE_REDIRECT_URI,
#   DESKTOP_SCHEME_URL, MAX_UPLOAD_SIZE_MB, DEFAULT_MAX_STORAGE_GB,
#   RATE_LIMIT_REQUESTS_PER_MIN, RUST_LOG, API_PORT, WEB_PORT
```

Nginx config:

```nginx
server {
    listen 443 ssl;
    server_name goprism.studio;

    location / {
        proxy_pass http://server:8080;
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
Browser ──HTTPS──▶ Nginx ──▶ Axum Server ──▶ PostgreSQL
                             │
                             └── /data/clips/ (local storage)
```

- Single binary serves the React SPA as static files
- JWT tokens (15m access + 30d refresh) for web auth
- API keys (SHA-256 hashed) for desktop app auth
- Rate limiting via token bucket middleware
- Server config stored in DB, overridable at runtime via admin API
