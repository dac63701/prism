---
plan name: prism-cloud-server
plan description: Full cloud server + dashboard
plan status: done
---

## Idea
Build the complete cloud sharing platform for Prism clips — an Axum HTTP server with PostgreSQL, multi-user JWT auth, file upload/storage, a React web dashboard for clip management, an admin panel for server management, and a lightweight public player page. All containerized via Docker. The server lives in a new `/website/` top-level directory with its own Cargo.toml for the Rust server and `/website/frontend/` for the React SPA.

## Implementation
- Step 1: Scaffold /website/ project — Axum server crate, React+Vite frontend with Tailwind, Dockerfile, docker-compose with Postgres. Config from env vars, single binary that also serves the built frontend statically.
- Step 2: Database schema & migrations — users table (id, email, password_hash, display_name, role, storage_used_bytes, max_storage_bytes, created_at), clips table (id, user_id, filename, original_filename, size_bytes, duration_secs, width, height, codec, game, title, visibility enum[public/private/unlisted], share_id, storage_path, thumbnail_path, created_at, updated_at), clip_tags, api_keys table. Use sqlx migrations.
- Step 3: Auth system — POST /api/auth/register, POST /api/auth/login, POST /api/auth/refresh (JWT access+refresh tokens). Password hashing with argon2. Token validation middleware for protected routes. Multi-role support (user, admin).
- Step 4: Clip upload endpoint — POST /api/clips/upload with multipart/form-data (file + metadata fields). Store file on local filesystem under /data/clips/{user_id}/{uuid}.mp4. Generate thumbnail server-side. Store metadata in Postgres. Return clip ID + share URL.
- Step 5: Clip CRUD endpoints — GET /api/clips (paginated, filterable by game/date/visibility), GET /api/clips/:id (single clip detail), PATCH /api/clips/:id (update title/visibility), DELETE /api/clips/:id (remove file + DB row). Authorization: users can only access their own clips (admin sees all).
- Step 6: Public player page — GET /s/:shareId serves a React page with video player (native <video> + hls.js for potential HLS). No auth needed. OG meta tags for link previews. Minimal, fast-loading, no JS framework overhead beyond the React bundle. Clip info overlay (title, game, duration, date). Download + share buttons.
- Step 7: Web dashboard (React SPA) — Login/Register pages, clip library with thumbnail grid, search by filename, filter by game/date, sort by date/size/duration, bulk select+delete, edit title/visibility inline, storage usage meter, account settings (change password, API key management). All pages behind auth guard.
- Step 8: Admin panel — User list with search/filter, user detail (clips, storage, ban/change role), server stats dashboard (uploads today/week/total, bandwidth, storage used, active users), config UI (max upload size, default per-user limits, storage backend toggle, rate limits), activity log viewer.
- Step 9: API key auth for desktop app — POST /api/auth/api-keys (generate, list, revoke). Desktop app authenticates via API key header instead of JWT for programmatic access (upload endpoint only). No token refresh needed for machine auth.
- Step 10: Rate limiting + security — Tower middleware for rate limiting (token bucket per user/IP). File upload size limits enforced at Axum layer. Request validation with serde. CORS config for dev. Helmet-like security headers.
- Step 11: Docker packaging — Multi-stage Dockerfile: frontend build (Node), then Rust build, then final slim image with only the binary + static assets. docker-compose.yml with Postgres service + app service. Health check endpoint. Environment variable configuration (.env.example).
- Step 12: Documentation & testing — README with setup instructions (docker-compose up). API docs (list endpoints with request/response examples). Integration tests for auth flow, upload, CRUD. SQLx offline mode for compile-time query checking.

## Required Specs
<!-- SPECS_START -->
- agents-guidelines
- cloud-server-spec
<!-- SPECS_END -->