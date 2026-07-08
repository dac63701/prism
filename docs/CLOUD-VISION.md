# Cloud Sharing & Server Vision

> **Status:** Future phase — foundations started in desktop app (upload API client, queue), full server infra built after MVP.

---

## Overview

Self-hosted sharing platform for clips. Users upload clips from the desktop app, get shareable links, manage clips via a web UI, and optionally use external S3-compatible storage.

```
Desktop App ──upload──▶ Server API ──store──▶ S3 / B2 / R2 / Local FS
                           │
                           ▼
                     Web Player Page (share link)
                           │
                           ▼
                    Web Dashboard (clip management)
                           │
                           ▼
                     Admin Panel (server config)
```

---

## Desktop Upload Flow

- **Share button** on each clip in library → triggers upload
- Upload runs in **background** — progress shown in tray and library
- On completion: **share link is copied to clipboard** automatically
- If offline: **queues the task**, retries when connection resumes
- **Retry logic** — exponential backoff, max 3 attempts
- Configurable server URL + auth token in settings

---

## Server Backend

### Recommended Stack
| Component | Tech | Reason |
|-----------|------|--------|
| API Server | Rust (Axum) or Go | Matches desktop stack or simple Go deploy |
| Database | SQLite (single-user) / PostgreSQL (multi-user) | Scale as needed |
| Storage | Local FS → S3 / B2 / R2 | Start simple, swap backend |
| Auth | JWT (access + refresh tokens) | Stateless, simple |
| Frontend | React / plain HTML + htmx | Dashboard + player pages |

### Core API Endpoints

```
POST   /api/auth/register          — Create account
POST   /api/auth/login             — Login, get JWT
POST   /api/auth/refresh           — Refresh token

POST   /api/clips/upload           — Upload clip file + metadata
GET    /api/clips                  — List user's clips (paginated)
GET    /api/clips/:id              — Clip metadata
DELETE /api/clips/:id              — Delete clip
PATCH  /api/clips/:id              — Update title / visibility

GET    /s/:shareId                 — Public player page (no auth needed)
```

### Upload Endpoint Design
```
POST /api/clips/upload
Content-Type: multipart/form-data

Fields:
  - file: video file (binary)
  - title: string (optional)
  - game: string (optional)
  - duration: number (seconds)

Response:
{
  "id": "clip-uuid",
  "share_url": "https://clips.example.com/s/abc123",
  "size": 52428800,
  "created_at": "2026-07-08T12:00:00Z"
}
```

---

## Web Player Page

- URL: `https://clips.example.com/s/<shareId>`
- Clean, minimal, fast-loading page
- **No JS framework** — plain HTML + CSS + video.js or native `<video>`
- Features:
  - Video player with controls (play/pause, seek, volume, fullscreen)
  - Clip title, game name, duration, date
  - Download button (if owner allows)
  - Share button (copies URL)
  - Embed code (iframe)
- Optional: password-protected clips
- OG meta tags for Discord/Twitter link previews

---

## Web Dashboard

- Login / Register page
- Clip library:
  - Thumbnail grid view
  - Search, filter by game/date
  - Sort by date, size, duration
  - Bulk select actions (delete, change visibility)
  - Edit title, set public/private/unlisted
- Storage usage meter (used / total)
- Account settings (change password, delete account)
- API key management (for desktop app)

---

## Admin Panel

- **User management:** list, ban, change roles
- **Storage management:** per-user usage, total server usage
- **Server stats:** uploads today/this week/total, bandwidth used, storage used
- **Configuration:**
  - Default per-user limits (max clips, max storage, max clip duration)
  - Storage backend config (S3/B2/R2/Local path)
  - Rate limiting settings
  - Max file size per upload
- **Logs:** recent upload activity, errors

---

## Storage Backends

| Backend | Pros | Cons |
|---------|------|------|
| **Local FS** | Simple, no extra cost, zero config | Not scalable, manual backup |
| **S3 (AWS)** | Scalable, CDN integration (CloudFront) | Egress costs per GB |
| **B2 (Backblaze)** | Cheap egress ($0.01/GB), S3-compatible API | Less CDN optimization |
| **R2 (Cloudflare)** | Zero egress fees, built-in CDN | Limited regions |

Storage backend should be **swappable via config** — trait-based interface:

```rust
#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn store(&self, key: &str, data: &[u8]) -> Result<String>;
    async fn retrieve(&self, key: &str) -> Result<Vec<u8>>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn exists(&self, key: &str) -> Result<bool>;
}
```

---

## Usage Limits (Configurable)

| Limit | Default | Admin Editable |
|-------|---------|----------------|
| Max clip duration | 5 minutes | Yes |
| Max file size per clip | 500 MB | Yes |
| Max total storage per user | 10 GB | Yes |
| Max uploads per day per user | 50 | Yes |
| Max clips per user | 500 | Yes |
| Rate limit (API) | 100 req/min per user | Yes |

---

## Implementation Roadmap

### Phase 1: Desktop Foundation (in MVP)
- Upload API client module in Rust (`reqwest`-based)
- Upload queue with persistence + retry
- Upload progress UI (library + tray)
- Server URL + auth config in settings

### Phase 2: Minimum Server
- Rust/Go HTTP server with file upload endpoint
- Local FS storage
- Simple player page (`/s/<shareId>`)
- JWT auth (register/login)

### Phase 3: Web Dashboard
- Clip library web UI
- Search, filter, sort
- Account management

### Phase 4: Admin Panel
- User management
- Usage limits enforcement
- Server stats dashboard
- Storage backend config UI

### Phase 5: Scale
- Multi-backend storage (S3, B2, R2)
- CDN integration
- HLS transcoding for adaptive streaming
- OAuth login (Google, Discord)
- Team/shared clip collections
