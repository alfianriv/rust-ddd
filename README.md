# rust-ddd

Rust monorepo (Axum + SeaORM + PostgreSQL), ports & adapters, use cases shared across apps.

## Layout

```
crates/core       domain + use cases (package `app-core`), no infra deps
crates/infra      adapters: SeaORM repositories, argon2 hasher, JWT tokens
crates/migration  sea-orm-migration, shared by all apps
crates/bootstrap  Config (TOML + env), factories (config -> adapter), Container, tracing
config/           default.toml + {APP_ENV}.toml
apps/apis         public HTTP API (Axum), JWT auth, OpenAPI docs
deploy/           deployment placeholder
```

Planned: `apps/{apis-internal, reports}`.

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

cargo run -p apis                     # http://localhost:8080, docs at /docs
```

## apis endpoints

| Method | Path | Auth |
|---|---|---|
| GET | `/health` | - |
| POST | `/auth/register` | - |
| POST | `/auth/login` | - |
| GET, PUT | `/users/{id}` | Bearer, `{id}` = JWT user |
| GET, POST | `/addresses` | Bearer (owner = JWT user) |
| PUT, DELETE | `/addresses/{id}` | Bearer, owner |
| PUT | `/addresses/{id}/default` | Bearer, owner |

Conventions:
- No `PATCH`: updates are full-replace `PUT`.
- No `/me` and no nested features (`/users/{id}/addresses`): one top-level path per feature.
- Ownership comes from the JWT `sub`. `/users/{id}`: `OwnedUser` extractor checks `{id}`.
  Other resources: the use case checks the record's owner (`DomainError::Forbidden`).
- 401 no/invalid token, 403 someone else's resource, 404 doesn't exist.

Errors are always `{"error": {"code", "message"}}`. OpenAPI spec: `/openapi.json`, UI: `/docs`.
