# Spec: cloud-sharing

Scope: feature

# Cloud Sharing & Server Vision

## Overview
Self-hosted sharing platform for clips. Users can upload clips from the desktop app to their server, get shareable links, manage clips via a web UI, and optionally use external S3-compatible storage.

## Architecture (Future)
```
Desktop App ──upload──▶ Server API ──store──▶ S3/B2/Local FS
                           │
                           ▼
                     Web Player Page (share link)
                           │
                           ▼
                    Web Dashboard (clip management)
```

## Server Backend Options
- **Rust** (Actix/Axum) — matching language stack, good performance
- **Go** — simple deployment, excellent for API servers
- **Node.js** — quick to prototype if preferred

## Features

### Upload Flow (In-App)
1. Clip selected → share button → uploads in background
2. Progress bar in tray/UI
3. On completion → copies share link to clipboard
4. If offline → queues for later upload

### Server API Endpoints
```
POST   /api/clips/upload        — Upload clip file + metadata
GET    /api/clips/:id           — Clip metadata
DELETE /api/clips/:id           — Delete clip
GET    /api/clips               — List user's clips (pagination)
PATCH  /api/clips/:id           — Update title/visibility
POST   /api/auth/register       — User registration
POST   /api/auth/login          — JWT-based auth
```

### Web Dashboard
- Login/register page
- Clip library with thumbnails
- Search/filter/sort clips
- Edit title, set public/private
- Bulk delete
- View embed player
- Admin panel for server settings

### Player Page (Share Link)
- `https://clips.example.com/s/<id>` — clean shareable URL
- In-browser video player (HLS or progressive)
- Download button (if owner allows)
- Simple, fast-loading page (no JS framework needed, plain HTML+video.js)

### Admin Panel
- User management (ban, role assignment)
- Storage usage per user
- Total storage limits configurable
- Server stats (uploads, bandwidth, storage)
- bucket config (S3/B2/Local path)
- Max clip size, max duration per user
- Rate limiting config

### Storage Backends
| Backend | Pros | Cons |
|---------|------|------|
| Local FS | Simple, no extra cost | Not scalable, backup needed |
| S3 (AWS) | Scalable, CDN integration | Cost per GB |
| B2 (Backblaze) | Cheap egress, S3-compatible | Less CDN options |
| R2 (Cloudflare) | No egress fees | Limited regions |

### Usage Limits (Configurable)
- Max clip duration per upload
- Max file size per clip
- Max total storage per user
- Max uploads per day
- Rate limiting per IP/user

## Implementation Order
1. Desktop upload foundation (API client module, upload queue)
2. Simple Rust server with clip upload + basic player page
3. Auth system (JWT)
4. Web dashboard
5. Admin panel
6. Multi-storage backend support
7. Advanced features (HLS transcoding, CDN, embed)