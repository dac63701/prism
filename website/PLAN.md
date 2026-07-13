# Prism Cloud Server — Full Implementation Plan

> **Repository location:** `/website/` (new top-level directory)
> **Stack:** Rust (Axum) + React (Vite + Tailwind) + PostgreSQL + Docker
> **Status:** Planned

---

## Architecture Overview

```
┌────────────────────────────────────┐     ┌───────────────────────────────────────────────┐
│         Desktop App                │     │              Docker Container                  │
│  ┌──────────────────────────┐      │     │  ┌─────────────────────────────────────────┐  │
│  │ Upload API Client        │──────┼─────┼─▶│  Axum HTTP Server                       │  │
│  │ (reqwest, API key auth,  │      │     │  │  ┌──────────┐ ┌───────────┐ ┌────────┐ │  │
│  │  queue, retry, progress) │      │     │  │  │ /api/*   │ │ /admin/*  │ │ /s/    │ │  │
│  └──────────────────────────┘      │     │  │  │ Clip CRUD │ │ Users,    │ │ Player │ │  │
└────────────────────────────────────┘     │  │  │ Auth, key │ │ Stats,    │ │ Page   │ │  │
                                           │  │  │ Upload    │ │ Config    │ │ (HTML) │ │  │
                                           │  │  └────┬─────┘ └─────┬─────┘ └────┬───┘ │  │
                                           │  └───────┼─────────────┼────────────┼──────┘  │
                                           │          │             │            │         │
                                           │  ┌───────▼─────────────▼────────────▼──────┐  │
                                           │  │        PostgreSQL (via sqlx)             │  │
                                           │  │   users · clips · api_keys · clip_tags  │  │
                                           │  └─────────────────────────────────────────┘  │
                                           │                                              │
                                           │  ┌─────────────────────────────────────────┐  │
                                           │  │  React SPA (static build, served by     │  │
                                           │  │  Axum via tower-http)                   │  │
                                           │  │  Dashboard · Admin · Player page        │  │
                                           │  └─────────────────────────────────────────┘  │
                                           │                                              │
                                           │  ┌─────────────────────────────────────────┐  │
                                           │  │  Local Filesystem (/data/)              │  │
                                           │  │  clips/{user_id}/{uuid}.mp4             │  │
                                           │  │  thumbs/{clip_id}.jpg                   │  │
                                           │  └─────────────────────────────────────────┘  │
                                           └──────────────────────────────────────────────┘
```

---

## Step 1 — Scaffold /website/ project

**Goal:** Create the project skeleton with Axum server + React frontend + Docker setup.

### Tasks

1.1 Create `/website/Cargo.toml` with dependencies:
```toml
[package]
name = "prism-server"
version = "0.2.3"
edition = "2021"

[dependencies]
axum = "0.8"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres", "uuid", "chrono"] }
jsonwebtoken = "9"
argon2 = "0.5"
tower-http = { version = "0.6", features = ["cors", "limit", "fs"] }
tower = "0.5"
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-appender = "0.2"
reqwest = { version = "0.12", features = ["multipart"] }
image = "0.25"
mp4 = "0.15"
base64 = "0.22"
rand = "0.8"
sha2 = "0.10"
hex = "0.4"
thiserror = "2"
dotenvy = "0.15"
mime_guess = "2"
```

1.2 Create `/website/src/main.rs` — entry point that:
   - Loads env vars from `.env` or environment
   - Initializes tracing subscriber
   - Creates DB connection pool via `sqlx::PgPool::connect()`
   - Runs sqlx migrations
   - Builds the Axum router
   - Serves the built React frontend as static files (`/` and all non-API routes)
   - Starts listening on `SERVER_HOST:SERVER_PORT`

1.3 Create `/website/src/config.rs` — configuration struct loaded from env vars:
   - `DATABASE_URL` (required)
   - `JWT_SECRET` (required, 64+ char warning in logs if short)
   - `SERVER_HOST` (default `0.0.0.0`)
   - `SERVER_PORT` (default `8080`)
   - `STORAGE_PATH` (default `/data`)
   - `MAX_UPLOAD_SIZE_MB` (default `500`)
   - `DEFAULT_MAX_STORAGE_GB` (default `10`)
   - `RATE_LIMIT_REQUESTS_PER_MIN` (default `100`)
   - `RUST_LOG` (default `info`)
   - `FRONTEND_URL` (default empty, for CORS in dev)

1.4 Create `/website/.env.example` with all vars + comments

1.5 Scaffold `/website/frontend/` with `npm create vite@latest`:
   - React + TypeScript + Vite
   - Install: `tailwindcss v4`, `react-router-dom`, `zustand`, `lucide-react`, `clsx`, `tailwind-merge`
   - Configure path alias `@/` → `./src/`
   - Create basic folder structure: `lib/`, `stores/`, `components/`, `pages/`

1.6 Create `Dockerfile` (multi-stage — placeholder, refined in Step 11):
   - Stage 1: Node image builds the frontend (npm ci → npm run build)
   - Stage 2: Rust image builds the server (cargo build --release)
   - Stage 3: Distroless or alpine image with binary + `frontend/dist/`

1.7 Create `docker-compose.yml`:
   - `postgres:16-alpine` with healthcheck
   - `app` service building from Dockerfile, depends on postgres

1.8 Verify `docker-compose up` starts both services without errors

---

## Step 2 — Database Schema & Migrations

**Goal:** Define all tables and create runnable sqlx migrations.

### Tasks

2.1 Create `/website/src/db/mod.rs` — `init_pool()` function that creates PgPool, runs migrations

2.2 Create migration `001_initial_schema.sql`:

```sql
-- 001_initial_schema.sql

CREATE TYPE user_role AS ENUM ('user', 'admin');
CREATE TYPE clip_visibility AS ENUM ('public', 'private', 'unlisted');

-- Users table
CREATE TABLE users (
    id                 UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email              TEXT NOT NULL UNIQUE,
    password_hash      TEXT NOT NULL,
    display_name       TEXT NOT NULL DEFAULT '',
    role               user_role NOT NULL DEFAULT 'user',
    storage_used_bytes BIGINT NOT NULL DEFAULT 0,
    max_storage_bytes  BIGINT NOT NULL DEFAULT 10737418240,
    is_banned          BOOLEAN NOT NULL DEFAULT false,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Clips table
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

-- Tags
CREATE TABLE clip_tags (
    clip_id UUID NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
    tag     TEXT NOT NULL,
    PRIMARY KEY (clip_id, tag)
);

-- Indexes
CREATE INDEX idx_clips_user_id ON clips(user_id);
CREATE INDEX idx_clips_share_id ON clips(share_id);
CREATE INDEX idx_clips_visibility ON clips(visibility) WHERE visibility != 'private';
CREATE INDEX idx_clips_game ON clips(game);
CREATE INDEX idx_clips_created_at ON clips(created_at DESC);
```

2.3 Create migration `002_api_keys.sql`:

```sql
CREATE TABLE api_keys (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name         TEXT NOT NULL DEFAULT '',
    key_hash     TEXT NOT NULL,
    key_prefix   TEXT NOT NULL,
    last_used_at TIMESTAMPTZ,
    expires_at   TIMESTAMPTZ,
    is_revoked   BOOLEAN NOT NULL DEFAULT false,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_api_keys_user_id ON api_keys(user_id);
CREATE INDEX idx_api_keys_key_prefix ON api_keys(key_prefix);
```

2.4 Create migration `003_activity_logs.sql`:

```sql
CREATE TYPE log_level AS ENUM ('info', 'warn', 'error');
CREATE TYPE log_action AS ENUM (
    'user_registered', 'user_logged_in', 'user_deleted',
    'clip_uploaded', 'clip_deleted', 'clip_viewed',
    'api_key_created', 'api_key_revoked',
    'admin_user_banned', 'admin_user_unbanned', 'admin_role_changed'
);

CREATE TABLE activity_logs (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID REFERENCES users(id) ON DELETE SET NULL,
    action      log_action NOT NULL,
    level       log_level NOT NULL DEFAULT 'info',
    ip_address  TEXT,
    details     JSONB,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_logs_created_at ON activity_logs(created_at DESC);
CREATE INDEX idx_logs_user_id ON activity_logs(user_id);
CREATE INDEX idx_logs_action ON activity_logs(action);
```

2.5 Query modules:
   - `/website/src/db/users.rs` — `create_user()`, `get_user_by_email()`, `get_user_by_id()`, `update_user()`, `list_users()`, `delete_user()`, `add_storage()`, `release_storage()`
   - `/website/src/db/clips.rs` — `insert_clip()`, `get_clip()` / `get_clip_by_share_id()`, `list_clips()`, `update_clip()`, `delete_clip()`, `count_user_clips()`
   - `/website/src/db/api_keys.rs` — `insert_api_key()`, `get_api_key_by_prefix()`, `list_api_keys()`, `revoke_api_key()`

2.6 Run `cargo sqlx prepare` for offline mode (compile-time query checking)

---

## Step 3 — Auth System

**Goal:** Full JWT-based auth with register, login, refresh, middleware.

### Tasks

3.1 `/website/src/auth/jwt.rs`:
   - Define `AccessClaims { sub, role, exp, iat }` and `RefreshClaims { sub, exp, iat, typ }`
   - `create_access_token(user_id, role) → String`
   - `create_refresh_token(user_id) → String`
   - `verify_access_token(token) → AccessClaims`
   - `verify_refresh_token(token) → RefreshClaims`
   - Use `jsonwebtoken` crate with HS256, secret from `JWT_SECRET` env var
   - Access token TTL: 15 min, Refresh token TTL: 30 days

3.2 `/website/src/auth/mod.rs`:
   - Axum middleware via `FromRequestParts` impl for `AuthUser` extractor:
     - Reads `Authorization: Bearer <token>` header
     - Verifies JWT, extracts `AuthUser { user_id, role }`
     - Returns 401 on invalid/missing token
   - `AdminAuth` extractor (extends `AuthUser`, returns 403 if not admin)
   - `ApiKeyOrJwtAuth` extractor (tries API key first, falls back to JWT) — used on upload endpoint

3.3 `/website/src/api/auth.rs` — endpoints:

**POST /api/auth/register**
- Body: `{ email, password, display_name? }`
- Validate: email format, password 8+ chars, email not taken
- Hash password with argon2
- Create user in DB
- Generate JWT tokens
- Log activity: `user_registered`
- Return: `{ user: { id, email, display_name, role }, access_token, refresh_token }`

**POST /api/auth/login**
- Body: `{ email, password }`
- Lookup user, verify argon2 hash
- Check `is_banned` → return 403
- Generate JWT tokens
- Log activity: `user_logged_in`
- Return: `{ user, access_token, refresh_token }`

**POST /api/auth/refresh**
- Body: `{ refresh_token }`
- Verify refresh token, extract user_id
- Lookup user still exists + not banned
- Generate new access + refresh tokens (rotation)
- Return: `{ access_token, refresh_token }`

**GET /api/auth/me** (JWT required)
- Return current user profile

**POST /api/auth/change-password** (JWT required)
- Body: `{ current_password, new_password }`
- Verify current password, hash new, update DB

**DELETE /api/auth/me** (JWT required)
- Delete user + cascade clips + api_keys
- Log activity: `user_deleted`

3.4 `/website/src/api/auth.rs` — API key endpoints:

**GET /api/auth/api-keys** (JWT required)
- List user's API keys (exclude hash, show prefix + name + last_used)
- Return: `{ api_keys: [{ id, prefix, name, last_used_at, created_at }] }`

**POST /api/auth/api-keys** (JWT required)
- Body: `{ name? }`
- Generate random 48-char key: `prism_<32-hex-chars>`
- Hash with SHA-256, store hash + prefix (first 8 chars after `prism_`)
- Return: `{ key: "prism_...", key_id }` — full key shown only once

**DELETE /api/auth/api-keys/:id** (JWT required)
- Revoke key (set `is_revoked = true`)
- Log activity: `api_key_revoked`

3.5 `/website/src/auth/api_key.rs`:
   - Parse API key from `Authorization: Bearer prism_...` header
   - SHA-256 hash the key, lookup by prefix + hash
   - Check not revoked, update `last_used_at`
   - Return user_id

---

## Step 4 — Clip Upload Endpoint

**Goal:** Allow authenticated uploads with multipart form data, file storage, and thumbnail generation.

### Tasks

4.1 `/website/src/storage/mod.rs` — `StorageBackend` trait:
```rust
#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn store(&self, path: &str, data: &[u8]) -> Result<()>;
    async fn retrieve(&self, path: &str) -> Result<Vec<u8>>;
    async fn delete(&self, path: &str) -> Result<()>;
    async fn exists(&self, path: &str) -> Result<bool>;
}
```

4.2 `/website/src/storage/local.rs` — `LocalStorage` impl:
- Root path from `STORAGE_PATH` env var
- `store()` writes file atomically (write to .tmp, rename)
- `delete()` removes file
- Directories created as needed (`/data/clips/{user_id}/`, `/data/thumbs/`)

4.3 `/website/src/thumbnail.rs`:
- `generate_thumbnail(video_path, output_path) → Result`
- Opens the MP4 file with `mp4::Mp4Reader` to get dimensions + first frame
- For now: extracts first frame via ffmpeg call (or image crate if we embed a frame extractor)
- Resizes to 320px wide, encodes as JPEG quality 80
- Saves to `/data/thumbs/{clip_id}.jpg`

4.4 `POST /api/clips/upload` in `/website/src/api/clips.rs`:
- Auth: `ApiKeyOrJwtAuth` — accepts JWT or API key
- Parse `multipart/form-data` via `axum::extract::Multipart`
- Fields: `file` (required), `title`, `game`, `duration_secs`, `width`, `height`, `codec`, `visibility`
- Validate file extension (.mp4 only)
- Check file size ≤ `MAX_UPLOAD_SIZE_MB`
- Check user's `storage_used_bytes + file_size ≤ max_storage_bytes`
- Generate UUID filename, store at `/data/clips/{user_id}/{uuid}.mp4`
- Generate thumbnail → `/data/thumbs/{clip_id}.jpg`
- Insert clip record in DB
- Update `storage_used_bytes` on user
- Log activity: `clip_uploaded`
- Return: `{ id, share_url: "/s/{share_id}", title, game, duration_secs, size_bytes, created_at }`

---

## Step 5 — Clip CRUD Endpoints

**Goal:** List, view, update, delete clips with filtering and pagination.

### Tasks

5.1 `GET /api/clips` — list user's clips:
- Auth: JWT
- Query params: `page` (default 1), `per_page` (default 50, max 100), `search` (filename ILIKE), `game`, `visibility`, `sort_by` (created_at|size|duration|title), `sort_dir` (asc|desc), `date_from`, `date_to`
- Returns: `{ clips: [...], total, page, per_page, total_pages }`
- Only returns clips owned by the authenticated user

5.2 `GET /api/clips/:id` — single clip detail:
- Auth: JWT
- Verify ownership (or admin)
- Returns full clip object

5.3 `PATCH /api/clips/:id` — update clip metadata:
- Auth: JWT
- Body: `{ title?, game?, visibility? }`
- Verify ownership (or admin)
- Update clip in DB, set `updated_at = NOW()`
- Return updated clip

5.4 `DELETE /api/clips/:id` — delete clip:
- Auth: JWT
- Verify ownership (or admin)
- Remove file from filesystem
- Remove thumbnail
- Delete from DB (cascade removes tags)
- Release storage (`storage_used_bytes -= clip.size_bytes`)
- Log activity: `clip_deleted`
- Return 204

5.5 `POST /api/clips/:id/regenerate-share` — new share URL:
- Auth: JWT
- Verify ownership
- Generate new random `share_id`
- Update DB
- Return: `{ share_id, share_url }`

---

## Step 6 — Public Player Page

**Goal:** Lightweight, fast-loading page to play shared clips.

### Tasks

6.1 `GET /api/s/:shareId/meta` — JSON metadata endpoint:
- No auth
- Lookup clip by share_id
- Only return if visibility is `public` or `unlisted`
- Return: `{ id, title, game, duration_secs, width, height, file_size, codec, created_at, download_url }`

6.2 Player page component at `/website/frontend/src/pages/PlayerPage.tsx`:
- No auth required
- Fetches clip metadata from `/api/s/{shareId}/meta` on mount
- Native `<video>` element with controls (play/pause, seek, volume, fullscreen)
- Thumbnail as poster until video loads
- Info overlay: clip title, game, duration, date recorded
- Download button (streams file via direct URL)
- Share button (copies current URL to clipboard)
- OG meta tags for Discord/Twitter link previews:
  - `og:title` = clip title
  - `og:description` = "Game: {game} · Duration: {duration}"
  - `og:image` = thumbnail JPEG URL
  - `og:type` = video.other
  - `twitter:card` = player

6.3 `GET /s/:shareId` — serves the player page:
- Axum route serves the SPA HTML with the shareId embedded
- The React app detects the `/s/:shareId` route and renders the player component

6.4 Server-side OG meta tags (for crawlers):
- When `GET /s/:shareId` is hit with a bot User-Agent, return a server-rendered HTML snippet with OG tags instead of the SPA
- Use a simple `match` on user-agent string for known bots (Discord, Twitter, Slack, Google)

---

## Step 7 — Web Dashboard (React SPA)

**Goal:** Full-featured clip management web application.

### Tasks

7.1 `/website/frontend/src/App.tsx` — router setup:
```tsx
<BrowserRouter>
  <Routes>
    <Route path="/login" element={<LoginPage />} />
    <Route path="/register" element={<RegisterPage />} />
    <Route element={<AuthGuard />}>
      <Route element={<DashboardLayout />}>
        <Route path="/" element={<HomePage />} />
        <Route path="/library" element={<LibraryPage />} />
        <Route path="/clip/:id" element={<ClipDetailPage />} />
        <Route path="/settings" element={<SettingsPage />} />
        <Route element={<AdminGuard />}>
          <Route path="/admin" element={<AdminDashboard />} />
          <Route path="/admin/users" element={<AdminUsersPage />} />
          <Route path="/admin/users/:id" element={<AdminUserDetailPage />} />
          <Route path="/admin/clips" element={<AdminClipsPage />} />
          <Route path="/admin/settings" element={<AdminSettingsPage />} />
          <Route path="/admin/logs" element={<AdminLogsPage />} />
        </Route>
      </Route>
    </Route>
  </Routes>
</BrowserRouter>
```

7.2 `/website/frontend/src/lib/api.ts` — typed API client:
- `api.get<T>(path, params?)` with auth header injection
- `api.post<T>(path, body)` with auth header
- `api.patch<T>(path, body)`
- `api.delete(path)`
- Token management: store access + refresh in localStorage, auto-refresh on 401
- Multipart upload function for clip upload
- Auth state store (Zustand) with `login()`, `register()`, `logout()`, `refreshAuth()`

7.3 **HomePage** (`/`):
- Dashboard overview after login
- Recent clips (last 5) with thumbnails in a row
- Storage usage meter (used / total with progress bar)
- Quick actions: Upload clip button, Open library, Create API key
- Server stats summary (total clips, storage used, latest upload)

7.4 **LoginPage** (`/login`):
- Email + password form
- Error display (invalid credentials, banned account)
- Link to register
- On success: redirect to dashboard

7.5 **RegisterPage** (`/register`):
- Email + password + confirm password + display name form
- Validation (email format, password 8+ chars, passwords match)
- On success: auto-login, redirect to dashboard

7.6 **LibraryPage** (`/library`):
- Thumbnail grid (responsive: 2-4 columns)
- Search bar (searches filename + title)
- Filter dropdowns: game, visibility
- Sort dropdown: date, size, duration, title
- Clip card shows: thumbnail, title, game, duration, size, date
- Bulk select (checkbox per card, select all toggle)
- Bulk delete (confirmation dialog)
- Pagination
- Empty state illustration when no clips
- Skeleton loading state

7.7 **ClipDetailPage** (`/clip/:id`):
- Video player (same component as public player page but in-page)
- Edit inline: title (text input), game (text input), visibility (dropdown)
- Delete button (with confirmation)
- Share URL display + copy button
- Clip info panel: file size, dimensions, codec, duration, created_at, download count

7.8 **SettingsPage** (`/settings`):
- Display name edit
- Change password form (current + new + confirm)
- API keys management:
  - List of existing keys (prefix, name, last used, created)
  - Generate new key button → shows key once in dialog
  - Revoke key button (with confirmation)
- Theme toggle (light/dark) — stored in localStorage
- Danger zone: delete account (requires confirmation + password)

7.9 **DashboardLayout** — shared layout wrapper:
- Sidebar navigation: Home, Library, Settings
- Admin section in sidebar (only visible if user role = admin): Admin Dashboard, Users, All Clips, Logs
- User menu top-right (display name, logout)
- Built with Tailwind v4, dark theme matching desktop app aesthetic (zinc/gray palette)

7.10 **AuthGuard** component:
- Checks for valid JWT on mount
- If no auth, redirect to `/login`
- `AdminGuard` extends this, redirects to `/` if not admin role

---

## Step 8 — Admin Panel

**Goal:** Full administrative interface for server management.

### Tasks

8.1 **AdminDashboard** (`/admin`):
- Stats cards row: Total users, Total clips, Storage used (GB), Uploads today
- Sparkline charts (simple CSS-based) for uploads over last 7 days
- Recent activity feed (last 20 log entries)
- Quick links to users, clips, settings pages

8.2 **AdminUsersPage** (`/admin/users`):
- Table with columns: Email, Display name, Role, Clips count, Storage used, Created, Status (active/banned)
- Search by email or display name
- Pagination
- Click row → navigate to user detail

8.3 **AdminUserDetailPage** (`/admin/users/:id`):
- User info card: email, display name, role, created, storage usage bar
- Actions: Change role (user/admin dropdown), Ban/Unban toggle, Delete user (with confirmation)
- Clip list (all clips by this user) with search + pagination
- User storage stats: used vs limit, clips count

8.4 **AdminClipsPage** (`/admin/clips`):
- Table/grid of ALL clips across all users
- Columns: Thumbnail, Title, User, Game, Size, Duration, Visibility, Created
- Search, filter by user/visibility/game
- Click → navigate to clip detail (admin view)
- Bulk delete

8.5 **AdminSettingsPage** (`/admin/settings`):
- Form to update server configuration:
  - Default max storage per user (GB)
  - Max upload file size (MB)
  - Rate limit (requests/min per user)
- Read-only display of current config values
- Server environment info (version, uptime, storage path)

8.6 **AdminLogsPage** (`/admin/logs`):
- Activity log viewer with table
- Columns: Timestamp, User, Action, Level, IP, Details
- Filter by action type, level, date range
- Pagination (100 per page)
- Clear logs button (with confirmation)

8.7 Admin API endpoints in `/website/src/api/admin.rs`:
- `GET /api/admin/users` — list all users (paginated, searchable by email/name)
- `GET /api/admin/users/:id` — user with stats
- `PATCH /api/admin/users/:id` — update role, ban status, storage limit
- `DELETE /api/admin/users/:id` — delete user + cascade
- `GET /api/admin/stats` — server-wide aggregate stats
- `GET /api/admin/clips` — list all clips across users
- `DELETE /api/admin/clips/:id` — delete any clip
- `GET /api/admin/logs` — activity logs (paginated, filterable)

---

## Step 9 — API Key Auth for Desktop App

**Goal:** Allow the desktop app to authenticate with long-lived API keys.

### Tasks

9.1 API key verification in `/website/src/auth/api_key.rs`:
- Extract key from `Authorization: Bearer prism_<key>` header
- SHA-256 hash the received key
- Look up by prefix (first 8 chars after `prism_`) in DB
- Verify hash matches, check not revoked
- Update `last_used_at` timestamp
- Return user_id

9.2 `ApiKeyOrJwtAuth` extractor in `/website/src/auth/mod.rs`:
- Try API key auth first (faster, no DB lookup for JWT decode)
- Fall back to JWT auth
- Used on upload endpoint so both desktop (API key) and web (JWT) can upload

9.3 Update desktop upload module (`src-tauri/src/upload/client.rs`):
- Settings already has server URL + auth token fields
- Desktop app generates API key via web dashboard settings page
- Stores API key in settings
- Sends `Authorization: Bearer prism_<key>` header on upload requests
- Upload currently implemented as placeholder — wires up to real API

---

## Step 10 — Rate Limiting & Security

**Goal:** Production-grade security middleware.

### Tasks

10.1 `/website/src/middleware/rate_limit.rs`:
- Token bucket rate limiter per user_id (authenticated) or IP (unauthenticated)
- Configurable via `RATE_LIMIT_REQUESTS_PER_MIN` env var
- Implemented as Tower `Layer` + `Service`
- Returns `429 Too Many Requests` with `Retry-After` header when exceeded
- Uses `HashMap<Uuid, TokenBucket>` for authenticated, `HashMap<IpAddr, TokenBucket>` for anonymous
- Periodic cleanup of stale buckets (every 5 min)

10.2 `/website/src/middleware/cors.rs`:
- In production: restrict to the server's own origin (SPA served from same origin is fine)
- In development: allow `FRONTEND_URL` env var (or `http://localhost:5173`)
- Use `tower_http::cors::CorsLayer`

10.3 Security headers (via `tower_http::set_header::SetResponseHeaderLayer`):
- `X-Content-Type-Options: nosniff`
- `X-Frame-Options: DENY`
- `Referrer-Policy: strict-origin-when-cross-origin`
- `Permissions-Policy: camera=(), microphone=()`
- CSP: `default-src 'self'; media-src 'self' data:; img-src 'self' data:; style-src 'self' 'unsafe-inline'`

10.4 File upload validation:
- Check file extension is `.mp4` before accepting
- Enforce `MAX_UPLOAD_SIZE_MB` at the Axum layer (before buffering)
- Validate MIME type from `mime_guess`

10.5 Input validation:
- serde validation for all request bodies
- Email format regex
- Password length checks
- Strip/trim all text inputs

---

## Step 11 — Docker Packaging

**Goal:** Production-ready multi-stage Docker build.

### Tasks

11.1 Final `Dockerfile`:

```dockerfile
# Stage 1: Build frontend
FROM node:22-alpine AS frontend
WORKDIR /build/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend/ .
RUN npm run build

# Stage 2: Build server
FROM rust:1.85-slim-bookworm AS server
WORKDIR /build/server
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release 2>/dev/null || true
COPY src/ src/
RUN cargo build --release

# Stage 3: Runtime
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 libgcc-s1 && rm -rf /var/lib/apt/lists/*
RUN groupadd -r prism && useradd -r -g prism -d /data -s /sbin/nologin prism
COPY --from=server /build/server/target/release/prism-server /usr/local/bin/prism-server
COPY --from=frontend /build/frontend/dist /app/frontend
RUN mkdir -p /data/clips /data/thumbs && chown -R prism:prism /data /app
USER prism
WORKDIR /app
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s \
  CMD wget -qO- http://localhost:8080/api/health || exit 1
ENTRYPOINT ["prism-server"]
```

11.2 `docker-compose.yml`:
```yaml
services:
  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_DB: prism
      POSTGRES_USER: prism
      POSTGRES_PASSWORD: ${DB_PASSWORD:?required}
    volumes:
      - pgdata:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U prism"]
      interval: 5s
      timeout: 3s
    restart: unless-stopped

  app:
    build: .
    ports:
      - "${PORT:-8080}:8080"
    environment:
      DATABASE_URL: postgres://prism:${DB_PASSWORD:?required}@postgres:5432/prism
      JWT_SECRET: ${JWT_SECRET:?required}
      STORAGE_PATH: /data
      RUST_LOG: ${RUST_LOG:-info}
    volumes:
      - clipdata:/data
    depends_on:
      postgres:
        condition: service_healthy
    restart: unless-stopped

volumes:
  pgdata:
  clipdata:
```

11.3 `.env.example`:
```env
# Required
DB_PASSWORD=change_me_to_a_secure_password
JWT_SECRET=change_me_to_a_random_64_character_string

# Optional
PORT=8080
RUST_LOG=info
```

11.4 Build: `docker compose build` completes, image size < 100MB

---

## Step 12 — Documentation & Testing

**Goal:** Comprehensive docs and integration tests.

### Tasks

12.1 `/website/README.md`:
- What is the Prism Cloud Server
- Quick start: `cp .env.example .env` → edit secrets → `docker compose up`
- Architecture overview
- API documentation (list all endpoints with curl examples)
- Environment variable reference
- Development setup (local Postgres, `cargo run`)
- Production deployment guide (reverse proxy, SSL, backups)

12.2 Integration tests in `/website/tests/`:
- `test_auth_flow.rs` — register → login → refresh → me → change password
- `test_clip_upload.rs` — upload clip → list clips → get clip → update → delete
- `test_api_key.rs` — create key → upload with key → revoke key → verify revocation
- `test_admin.rs` — admin user management, stats, clip management
- `test_public.rs` — share page metadata, visibility enforcement
- `test_rate_limit.rs` — hit rate limit, verify 429

12.3 Test helpers:
- Test database setup (docker compose for tests or separate test DB)
- `test_app()` function that starts the server with test config
- Seed data helpers (create users, clips, API keys)

12.4 CI integration:
- GitHub Actions workflow in `.github/workflows/server.yml`
- Steps: checkout → rust-cache → cargo build → cargo test
- Runs sqlx prepare check
- Optional: Docker build test

---

## Frontend Routes Summary

| Route | Component | Auth | Description |
|-------|-----------|------|-------------|
| `/` | **HomePage** | JWT | Dashboard — recent clips, storage meter, quick actions |
| `/login` | LoginPage | None | Login form |
| `/register` | RegisterPage | None | Registration form |
| `/library` | LibraryPage | JWT | Clip grid with search/filter/sort/bulk actions |
| `/clip/:id` | ClipDetailPage | JWT | Player + edit metadata + delete + share |
| `/settings` | SettingsPage | JWT | Password change, API keys, delete account |
| `/admin` | AdminDashboard | Admin | Server stats overview |
| `/admin/users` | AdminUsersPage | Admin | User management table |
| `/admin/users/:id` | AdminUserDetailPage | Admin | Single user + clips + actions |
| `/admin/clips` | AdminClipsPage | Admin | All clips across users |
| `/admin/settings` | AdminSettingsPage | Admin | Server configuration form |
| `/admin/logs` | AdminLogsPage | Admin | Activity log viewer |
| `/s/:shareId` | PlayerPage | None | Public player (also server-rendered OG tags) |

---

## Project Structure (Final)

```
/website/
├── Cargo.toml
├── Cargo.lock
├── .env.example
├── Dockerfile
├── docker-compose.yml
├── sqlx-data.json
├── README.md
│
├── src/
│   ├── main.rs
│   ├── config.rs          # Env config struct
│   ├── errors.rs           # Unified AppError type
│   ├── frontend.rs         # Static file serving
│   │
│   ├── db/
│   │   ├── mod.rs          # Pool init, migration runner
│   │   ├── users.rs        # User queries
│   │   ├── clips.rs        # Clip queries
│   │   └── api_keys.rs     # API key queries
│   │   └── migrations/
│   │       ├── 001_initial_schema.sql
│   │       ├── 002_api_keys.sql
│   │       └── 003_activity_logs.sql
│   │
│   ├── auth/
│   │   ├── mod.rs          # AuthUser/AdminAuth extractors
│   │   ├── jwt.rs          # JWT encode/decode
│   │   └── api_key.rs      # API key verify
│   │
│   ├── api/
│   │   ├── mod.rs          # Router composition
│   │   ├── auth.rs         # Register, login, refresh, me, API keys
│   │   ├── clips.rs        # Upload, list, get, update, delete
│   │   ├── admin.rs        # Admin endpoints
│   │   └── public.rs       # Public share metadata
│   │
│   ├── storage/
│   │   ├── mod.rs          # StorageBackend trait
│   │   └── local.rs        # Local filesystem impl
│   │
│   ├── thumbnail.rs        # JPEG thumbnail generation
│   │
│   └── middleware/
│       ├── rate_limit.rs   # Token bucket rate limiter
│       └── cors.rs         # CORS config
│
├── frontend/
│   ├── package.json
│   ├── vite.config.ts
│   ├── tsconfig.json
│   ├── index.html
│   │
│   └── src/
│       ├── main.tsx
│       ├── App.tsx
│       ├── index.css       # Tailwind v4 imports
│       │
│       ├── lib/
│       │   └── api.ts      # Typed API client + auth helpers
│       │
│       ├── stores/
│       │   └── auth.ts     # Auth state (Zustand)
│       │
│       ├── components/
│       │   ├── ui/         # Button, Input, Modal, etc.
│       │   ├── AuthGuard.tsx
│       │   ├── AdminGuard.tsx
│       │   ├── DashboardLayout.tsx
│       │   ├── ClipCard.tsx
│       │   ├── VideoPlayer.tsx
│       │   ├── StorageMeter.tsx
│       │   └── Pagination.tsx
│       │
│       └── pages/
│           ├── HomePage.tsx
│           ├── LoginPage.tsx
│           ├── RegisterPage.tsx
│           ├── LibraryPage.tsx
│           ├── ClipDetailPage.tsx
│           ├── SettingsPage.tsx
│           ├── PlayerPage.tsx
│           ├── admin/
│           │   ├── AdminDashboard.tsx
│           │   ├── AdminUsersPage.tsx
│           │   ├── AdminUserDetailPage.tsx
│           │   ├── AdminClipsPage.tsx
│           │   ├── AdminSettingsPage.tsx
│           │   └── AdminLogsPage.tsx
│           └── not found page
│
└── tests/
    ├── test_auth_flow.rs
    ├── test_clip_upload.rs
    ├── test_api_key.rs
    ├── test_admin.rs
    ├── test_public.rs
    └── test_rate_limit.rs
```

---

## Key Design Decisions

1. **Single binary serving everything** — Axum serves the built SPA as static files. No separate Node server, no nginx needed in dev. One `docker-compose up` and everything runs.

2. **API key auth for desktop** — Long-lived API keys (not JWTs) for the desktop upload client. Keys can be generated/revoked from the web dashboard. Desktop stores the key in settings.

3. **File isolation** — `/data/clips/{user_id}/{uuid}.mp4`. The filesystem path is never exposed to API responses. All access goes through DB lookups by UUID or share_id.

4. **Share IDs** — 12-hex-char random string from 6 bytes of `gen_random_bytes()`. Collision probability: ~1 in 2^48. Can be regenerated by clip owner at any time.

5. **Storage enforcement** — `users.storage_used_bytes` is updated atomically on upload/delete. Checked before upload begins. Admin can adjust `max_storage_bytes` per user.

6. **Activity logging** — Key events logged to `activity_logs` table for audit trail and admin panel display. Uses typed enum columns for queryable action filtering.

7. **Multi-stage Docker** — Frontend built with Node, server built with Rust, final image is Debian slim (~50-80MB). No build tools in the final image.

8. **Rate limiting** — Token bucket per user (authenticated) or per IP (anonymous). Prevents abuse while allowing legitimate burst uploads.

---

## Acceptance Criteria

- [ ] `docker-compose up` starts Postgres + Axum server with zero manual setup beyond editing `.env`
- [ ] `POST /api/auth/register` creates user, returns JWT tokens, shows in DB
- [ ] `POST /api/auth/login` authenticates and returns tokens
- [ ] JWT middleware correctly returns 401 for missing/invalid tokens, 403 for banned users
- [ ] `POST /api/clips/upload` accepts multipart upload, stores file on disk, records in DB
- [ ] Thumbnail generated and saved alongside the clip file
- [ ] `GET /api/clips` returns paginated, filterable clip list (user-scoped)
- [ ] `GET /s/:shareId` serves a functional video player page with OG meta tags
- [ ] Dashboard HomePage shows recent clips, storage meter, quick actions
- [ ] LibraryPage: search, filter by game, sort, bulk delete, pagination all work
- [ ] ClipDetailPage: video plays, title/game/visibility editable, share URL copies
- [ ] SettingsPage: API key generation works, keys listed/revocable, password change works
- [ ] AdminDashboard: stats cards show correct aggregate data
- [ ] AdminUsersPage: list users, search, ban/unban, change role, delete user
- [ ] AdminClipsPage: see all clips across users, bulk delete
- [ ] AdminLogsPage: activity feed renders, filterable
- [ ] API key auth works for desktop-style upload (pass `prism_...` key, get 200)
- [ ] Rate limiting returns 429 after exceeding limit
- [ ] `POST /api/auth/me` with DELETE deletes user + cascades clips + keys
- [ ] `GET /api/health` returns `{ status: "ok" }`
- [ ] Multi-stage Docker build succeeds, final image < 100MB
- [ ] README with complete setup instructions
