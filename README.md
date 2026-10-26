# rust-monorepo

Rust monorepo (Axum + SeaORM), ports & adapters, use cases shared across apps.

## Layout

```
crates/core     domain + use cases (package `app-core`), no infra deps
deploy/         deployment placeholder
```

Planned: `crates/infra`, `crates/migration`, `crates/bootstrap`, `apps/{apis, apis-internal, reports}`.

Dependency rule: `apps/* -> bootstrap -> infra -> core`. `core` never depends on axum, sea-orm, redis, etc.

## Dev

```sh
cp .env.example .env
docker compose up -d
cargo test --workspace
```
