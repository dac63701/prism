# syntax=docker/dockerfile:1.7

# ── Frontend builder ──
FROM node:20-alpine AS frontend

WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN --mount=type=cache,target=/root/.npm npm ci --no-audit --no-fund

COPY frontend/ ./
RUN --mount=type=cache,target=/root/.npm \
    --mount=type=cache,target=/app/frontend/.next/cache \
    npm run build

# ── Rust builder ──
FROM rust:1.88-alpine AS backend

RUN apk add --no-cache musl-dev pkgconfig

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ src/

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release --locked && \
    cp /app/target/release/prism-server /usr/local/bin/prism-server

# ── Runtime ──
FROM alpine:3.21

RUN apk add --no-cache ca-certificates

WORKDIR /app
COPY --from=backend /usr/local/bin/prism-server ./
COPY --from=frontend /app/frontend/dist ./frontend/dist

EXPOSE 8080

ENV SERVER_HOST=0.0.0.0
ENV SERVER_PORT=8080
ENV STORAGE_PATH=/data

VOLUME /data

CMD ["./prism-server"]
