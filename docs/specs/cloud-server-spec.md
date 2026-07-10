# Spec: cloud-server-spec

Scope: feature

# Prism Cloud Server — Feature Specification

## 1. Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                     Desktop App                          │
│  ┌──────────┐  ┌───────────┐  ┌────────────────────┐   │
│  │Settings  │  │Library UI │  │Upload API Client   │   │
│  │(server   │  │(upload    │  │(reqwest, retry,    │   │
│  │ URL+key) │  │ progress) │  │ queue, auth)       │   │
│  └──────────┘  └───────────┘  └────────┬───────────┘   │
└─────────────────────────────────────────┼───────────────┘
                                          │
                          HTTPS (API key or JWT)
                                          │
┌─────────────────────────────────────────┼────────────────────────────┐
│  Docker Container                       ▼          Prism Server      │
│  ┌────────────────────────────────────────────────────────┐          │
│  │  ┌──────────────────┐  ┌─────────────────────────┐    │          │
│  │  │  Axum HTTP Server │  │  Tower Middleware        │    │          │
│  │  │  ┌──────────────┐ │  │  - Rate limiting         │    │          │
│  │  │  │ /api/auth/*  │ │  │  - CORS                  │    │          │
│  │  │  │ /api/clips/* │ │  │  - JWT/API key auth      │    │          │
│  │  │  │ /s/:shareId  │ │  │  - Request validation    │    │          │
│  │  │  │ /admin/*     │ │  │  - File size enforcement │    │          │
│  │  │  │ /api/* (static│ │  └─────────────────────────┘    │          │
│  │  │  │   frontend)  │ │                                  │          │
│  │  │  └──────────────┘ │                                  │          │
│  │  └───────────────────────────────────────────────────────┘          │
│  │                                                                    │
│  │  ┌──────────────┐  ┌──────────────────┐  ┌────────────────────┐   │
│  │  │  PostgreSQL   │  │  Local Filesystem │  │  React SPA         │   │
│  │  │  (sqlx)       │  │  /data/clips/     │  │  (static build)    │   │
│  │  │  users, clips │  │  /data/thumbs/    │  │  Dashboard + Admin │   │
│  │  │  api_keys     │  │  /data/avatars/   │  │  + Player page     │   │
│  │  └──────────────┘  └──────────────────┘  └────────────────────┘   │
│  └────────────────────────────────────────────────────────────────────┘
└─────────────────────────────────────────────────────────────────────────┘
```

## 2. Tech Stack

| Component | Technology | Justification |
|-----------|-----------|---------------|
| Server | Rust + Axum 0.8+ | Matches desktop stack, performance, shared types |
| Database | PostgreSQL 16+ via sqlx | Multi-user safety, concurrent writes, migrations |
| Auth | JWT (access + refresh) + API keys | Stateless API auth, long-lived keys for desktop |
| Password hashing | argon2 | Industry standard |
| Frontend | React 19 + Vite + Tailwind v4 | Same stack as desktop, code sharing potential |
| Media handling | image crate (thumbnails), mp4 crate (metadata) | Already in desktop dependency tree |
| Deployment | Docker + docker-compose | Single container + Postgres service |
| Static serving | Axum tower-http | Serve built SPA from the same binary |

## 3. Database Schema

### users
```sql
CREATE TYPE user_role AS ENUM ('user', 'admin');

CREATE TABLE users (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email           TEXT NOT NULL UNIQUE,
    password_hash   TEXT NOT NULL,
    display_name    TEXT NOT NULL DEFAULT '',
    role            user_role NOT NULL DEFAULT 'user',
    storage_used_bytes BIGINT NOT NULL DEFAULT 0,
    max_storage_bytes  BIGINT NOT NULL DEFAULT 10737418240, -- 10 GB
    is_banned       BOOLEAN NOT NULL DEFAULT false,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

### clips
```sql
CREATE TYPE clip_visibility AS ENUM ('public', 'private', 'unlisted');

CREATE TABLE clips (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id           UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    original_filename TEXT NOT NULL,
    storage_path      TEXT NOT NULL,
    thumbnail_path    TEXT,
    share_id          TEXT NOT NULL UNIQUE DEFAULT encode(gen_random_bytes(6), 'hex'),
    title             TEXT NOT NULL DEFAULT '',
    game              TEXT NOT NULL DEFAULT '',
    duration_secs     REAL NOT NULL DEFAULT 0,
    size_bytes        BIGINT NOT NULL DEFAULT 0,
    width             INT NOT NULL DEFAULT 0,
    height            INT NOT NULL DEFAULT 0,
    codec             TEXT NOT NULL DEFAULT 'h264',
    visibility        clip_visibility NOT NULL DEFAULT 'unlisted',
    download_count    INT NOT NULL DEFAULT 0,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_clips_user_id ON clips(user_id);
CREATE INDEX idx_clips_share_id ON clips(share_id);
CREATE INDEX idx_clips_visibility ON clips(visibility) WHERE visibility != 'private';
CREATE INDEX idx_clips_game ON clips(game);
CREATE INDEX idx_clips_created_at ON clips(created_at DESC);
```

### clip_tags
```sql
CREATE TABLE clip_tags (
    clip_id UUID NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
    tag     TEXT NOT NULL,
    PRIMARY KEY (clip_id, tag)
);
```

### api_keys
```sql
CREATE TABLE api_keys (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name        TEXT NOT NULL DEFAULT '',
    key_hash    TEXT NOT NULL,
    prefix      TEXT NOT NULL, -- first 8 chars for identification
    last_used_at TIMESTAMPTZ,
    expires_at  TIMESTAMPTZ,
    is_revoked  BOOLEAN NOT NULL DEFAULT false,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_api_keys_user_id ON api_keys(user_id);
CREATE INDEX idx_api_keys_prefix ON api_keys(prefix);
```

## 4. API Endpoints

### Auth (prefix: /api/auth)

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | /api/auth/register | None | Create account. Body: `{email, password, display_name?}`. Returns: `{user, access_token, refresh_token}` |
| POST | /api/auth/login | None | Login. Body: `{email, password}`. Returns: `{user, access_token, refresh_token}` |
| POST | /api/auth/refresh | None | Refresh token. Body: `{refresh_token}`. Returns: `{access_token, refresh_token}` |
| POST | /api/auth/change-password | JWT | Change password. Body: `{current_password, new_password}` |
| GET | /api/auth/me | JWT | Current user info |
| DELETE | /api/auth/me | JWT | Delete own account |

### API Keys (prefix: /api/auth/api-keys)

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | /api/auth/api-keys | JWT | List own API keys |
| POST | /api/auth/api-keys | JWT | Generate new key. Body: `{name?}`. Returns: `{key, key_id}` (full key only shown once) |
| DELETE | /api/auth/api-keys/:id | JWT | Revoke key |

### Clips (prefix: /api/clips)

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | /api/clips/upload | JWT or API Key | Upload clip. Multipart: `file` (video), `title?`, `game?`, `duration_secs?`, `width?`, `height?`, `codec?`, `visibility?`. Returns: `{id, share_url}` |
| GET | /api/clips | JWT | List user's clips. Query: `page`, `per_page`, `search`, `game`, `visibility`, `sort_by`, `sort_dir`, `date_from`, `date_to` |
| GET | /api/clips/:id | JWT | Single clip detail (user's own or admin) |
| PATCH | /api/clips/:id | JWT | Update clip. Body: `{title?, game?, visibility?}` |
| DELETE | /api/clips/:id | JWT | Delete clip (user's own or admin) |
| POST | /api/clips/:id/regenerate-share | JWT | Generate new share ID |

### Public (no auth)

| Method | Path | Description |
|--------|------|-------------|
| GET | /s/:shareId | Public player page (HTML) |
| GET | /api/s/:shareId/meta | Clip metadata for player (JSON). Returns only public/unlisted clips. |

### Admin (prefix: /api/admin, role: admin required)

| Method | Path | Description |
|--------|------|-------------|
| GET | /api/admin/users | List all users (paginated, searchable) |
| GET | /api/admin/users/:id | User detail with stats |
| PATCH | /api/admin/users/:id | Update user: `{role?, is_banned?, max_storage_bytes?}` |
| DELETE | /api/admin/users/:id | Delete user + their clips |
| GET | /api/admin/stats | Server stats: total users, clips, storage, uploads today/week/all time |
| GET | /api/admin/clips | List ALL clips (paginated, filterable) |
| DELETE | /api/admin/clips/:id | Delete any clip |
| GET | /api/admin/logs | Recent activity log (paginated) |

### Health

| Method | Path | Description |
|--------|------|-------------|
| GET | /api/health | Health check (DB connectivity). Returns `{status, version, uptime}` |

## 5. JWT Token Design

```rust
struct AccessClaims {
    sub: Uuid,         // user_id
    role: UserRole,    // "user" | "admin"
    exp: usize,        // expiry (15 min)
    iat: usize,
}

struct RefreshClaims {
    sub: Uuid,
    exp: usize,        // expiry (30 days)
    iat: usize,
    typ: String,       // "refresh"
}
```

- Access token: 15 min validity, sent as `Authorization: Bearer <token>`
- Refresh token: 30 day validity, sent in body or secure cookie
- API Key: SHA-256 hash stored in DB, prefix for lookup, never expires (manual revocation)

## 6. Frontend Routes (React SPA)

| Route | Component | Auth | Description |
|-------|-----------|------|-------------|
| / | HomePage | JWT | Dashboard home — recent clips, storage meter, quick actions |
| /login | LoginPage | None | Login form |
| /register | RegisterPage | None | Registration form |
| /library | LibraryPage | JWT | Clip grid with search/filter/sort/bulk actions |
| /clip/:id | ClipDetailPage | JWT | Clip detail, player, edit meta, delete |
| /settings | SettingsPage | JWT | Account settings (password, API keys) |
| /admin | AdminDashboard | Admin | Server stats overview |
| /admin/users | AdminUsersPage | Admin | User management |
| /admin/users/:id | AdminUserDetailPage | Admin | Single user detail + clips |
| /admin/clips | AdminClipsPage | Admin | All clips management |
| /admin/settings | AdminSettingsPage | Admin | Server config (limits, storage) |
| /admin/logs | AdminLogsPage | Admin | Activity log viewer |

## 7. Project Structure

```
/website/
├── Cargo.toml              # Axum server crate
├── .env.example             # Environment variable template
├── Dockerfile               # Multi-stage build
├── docker-compose.yml       # App + Postgres
├── src/
│   ├── main.rs              # Entry point, server startup
│   ├── config.rs            # Env-based configuration
│   ├── db/
│   │   ├── mod.rs           # Database pool setup
│   │   ├── migrations/      # sqlx migrations
│   │   ├── users.rs         # User queries
│   │   ├── clips.rs         # Clip queries
│   │   └── api_keys.rs      # API key queries
│   ├── auth/
│   │   ├── mod.rs           # Auth middleware (JWT + API key)
│   │   ├── jwt.rs           # JWT encode/decode
│   │   └── api_key.rs       # API key verify
│   ├── api/
│   │   ├── mod.rs           # Router composition
│   │   ├── auth.rs          # Auth endpoints
│   │   ├── clips.rs         # Clip CRUD + upload
│   │   ├── admin.rs         # Admin endpoints
│   │   └── public.rs        # Public share pages
│   ├── storage/
│   │   ├── mod.rs           # StorageBackend trait
│   │   └── local.rs         # Local filesystem implementation
│   ├── thumbnail.rs         # Thumbnail generation
│   ├── errors.rs            # Unified error type
│   ├── middleware/
│   │   ├── rate_limit.rs    # Rate limiter
│   │   └── cors.rs          # CORS config
│   └── frontend.rs          # Static file serving for SPA
├── frontend/                # React + Vite SPA
│   ├── package.json
│   ├── vite.config.ts
│   ├── tsconfig.json
│   ├── tailwind.config.ts
│   ├── index.html
│   └── src/
│       ├── main.tsx
│       ├── App.tsx
│       ├── lib/             # API client, utils
│       ├── stores/          # Zustand stores
│       ├── components/      # Shared UI components
│       └── pages/           # Route pages
└── sqlx-data.json          # Offline compile-time query check
```

## 8. Configuration (Environment Variables)

```
DATABASE_URL=postgres://prism:prism@localhost:5432/prism
JWT_SECRET=change-me-to-a-random-64-char-string
SERVER_HOST=0.0.0.0
SERVER_PORT=8080
STORAGE_PATH=/data
MAX_UPLOAD_SIZE_MB=500
DEFAULT_MAX_STORAGE_GB=10
RATE_LIMIT_REQUESTS_PER_MIN=100
RUST_LOG=info
FRONTEND_URL=http://localhost:5173  # for CORS in dev
```

## 9. Docker Setup

```yaml
# docker-compose.yml (conceptual)
services:
  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_DB: prism
      POSTGRES_USER: prism
      POSTGRES_PASSWORD: ${DB_PASSWORD}
    volumes:
      - pgdata:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U prism"]
      interval: 5s

  app:
    build: .
    ports:
      - "8080:8080"
    environment:
      DATABASE_URL: postgres://prism:${DB_PASSWORD}@postgres:5432/prism
      JWT_SECRET: ${JWT_SECRET}
      STORAGE_PATH: /data
    volumes:
      - clipdata:/data
    depends_on:
      postgres:
        condition: service_healthy

volumes:
  pgdata:
  clipdata:
```

## 10. Key Design Decisions

1. **Single binary serving everything**: Axum serves the built SPA as static files via tower-http. No separate frontend server needed.

2. **API key auth for desktop**: Desktop app uses long-lived API keys (not JWT) for upload. JWTs are for the web dashboard. API keys can be revoked individually.

3. **File isolation**: Clips stored at `/data/clips/{user_id}/{uuid}.mp4`. Storage path is never exposed to users — the DB maps UUID → path.

4. **Share IDs**: 12-char hex string from random bytes (6 bytes → 12 hex chars). Collision probability is negligible. Can be regenerated by the clip owner.

5. **Storage used tracking**: `storage_used_bytes` on users table updated on upload/delete. Enforced at API layer before upload. Admin can adjust per-user limits.

6. **Thumbnails**: Generated server-side on upload from the video file (first frame extraction). Stored alongside clip at `/data/thumbs/{clip_id}.jpg`. The image crate handles JPEG encoding.

## 11. Acceptance Criteria

- [ ] `docker-compose up` starts Postgres + Axum server with zero manual setup
- [ ] `POST /api/auth/register` creates user and returns JWT tokens
- [ ] `POST /api/auth/login` authenticates and returns tokens
- [ ] JWT auth middleware correctly guards protected routes
- [ ] `POST /api/clips/upload` accepts multipart upload with metadata
- [ ] Uploaded file lands in `/data/clips/{user_id}/{uuid}.mp4`
- [ ] `GET /s/:shareId` serves a functional video player page
- [ ] Dashboard login → see clips → search → filter → delete works
- [ ] Admin panel: list users, ban, change roles, view stats
- [ ] API key auth works for desktop upload flow
- [ ] Rate limiting prevents abuse (>100 req/min returns 429)
- [ ] Health endpoint returns DB connectivity status
- [ ] Multi-stage Docker build produces small final image (<50MB)