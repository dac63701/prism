.PHONY: all dev serve db backend frontend clean docker-build docker-up docker-down frontend-build

all: dev

# ── Local Development ───────────────────────────────────────────────

dev: db backend frontend-dev

serve: db frontend-build
	cargo run

db:
	docker compose up -d postgres

backend:
	cargo run

frontend-dev:
	cd frontend && npm run dev

frontend-build:
	cd frontend && npm run build

# ── Docker ──────────────────────────────────────────────────────────

docker-build:
	docker build -t prism-server .

docker-up:
	docker compose up --build -d

docker-down:
	docker compose down

# ── Maintenance ─────────────────────────────────────────────────────

clean:
	cargo clean
	rm -rf frontend/node_modules frontend/dist data/
