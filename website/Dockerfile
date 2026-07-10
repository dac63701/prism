# ── Frontend builder ──
FROM node:20-alpine AS frontend

WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci

COPY frontend/ ./
RUN npm run build

# ── Rust builder ──
FROM rust:1.88-alpine AS backend

RUN apk add --no-cache musl-dev pkgconfig

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ src/

RUN cargo build --release

# ── Runtime ──
FROM alpine:3.21

RUN apk add --no-cache ca-certificates

WORKDIR /app
COPY --from=backend /app/target/release/prism-server ./
COPY --from=frontend /app/frontend/dist ./frontend/dist

EXPOSE 8080

ENV SERVER_HOST=0.0.0.0
ENV SERVER_PORT=8080
ENV STORAGE_PATH=/data

VOLUME /data

CMD ["./prism-server"]
