# rust-ddd

Rust monorepo (Axum + SeaORM + PostgreSQL), ports & adapters, use cases shared across apps.

## Layout

```
crates/core       domain + use cases (package `app-core`), no infra deps
crates/infra      adapters: SeaORM repositories, argon2 hasher, JWT tokens
crates/migration  sea-orm-migration, shared by all apps
crates/bootstrap  Config (TOML + env), factories (config -> adapter), Container, tracing
config/           default.toml + {APP_ENV}.toml
deploy/           deployment placeholder
```

Planned: `apps/{apis, apis-internal, reports}`.

Dependency rule: `apps/* -> bootstrap -> infra -> core`. `core` never depends on axum, sea-orm, redis, etc.

Schema convention: no FOREIGN KEY constraints. Relations live in code (SeaORM entity relations + use cases).

## Config

Load order (later wins): `config/default.toml` -> `config/{APP_ENV}.toml` -> `APP_*` env vars.
`APP_ENV` defaults to `development`. Env override = section + `__` + key:

```sh
APP_DATABASE__URL=postgres://...   # [database] url
APP_TOKEN__SECRET=...              # [token] secret, >= 32 bytes, required outside development
```

Each section picks its adapter with `driver` (e.g. `[token] driver = "jwt"`).
New adapter: add a variant to the config enum (`bootstrap/src/config.rs`) and a match arm in `bootstrap/src/factory.rs`.

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
