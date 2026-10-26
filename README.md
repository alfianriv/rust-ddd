# rust-ddd

Rust monorepo (Axum + SeaORM + PostgreSQL), ports & adapters, use cases shared across apps.

## Layout

```
crates/core       domain + use cases (package `app-core`), no infra deps
crates/infra      adapters: SeaORM repositories, argon2 hasher, JWT tokens
crates/migration  sea-orm-migration, shared by all apps
crates/bootstrap  Config (env) + Container (all use cases wired) + tracing setup
deploy/           deployment placeholder
```

Planned: `apps/{apis, apis-internal, reports}`.

Dependency rule: `apps/* -> bootstrap -> infra -> core`. `core` never depends on axum, sea-orm, redis, etc.

Schema convention: no FOREIGN KEY constraints. Relations live in code (SeaORM entity relations + use cases).

## Dev

```sh
cp .env.example .env
docker compose up -d
export $(grep -v '^#' .env | xargs)

cargo run -p migration -- up          # apply migrations (uses DATABASE_URL)
cargo test --workspace                # unit tests; DB tests skipped

# DB integration tests (use a separate database, they leave test rows behind)
docker compose exec postgres createdb -U app app_test
TEST_DATABASE_URL=postgres://app:app@127.0.0.1:5432/app_test cargo test --workspace
```
