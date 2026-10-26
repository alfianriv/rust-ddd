# rust-ddd

Rust monorepo (Axum + SeaORM), ports & adapters, use cases shared across apps.

## Layout

```
crates/core       domain + use cases (package `app-core`), no infra deps
crates/infra      adapters: SeaORM repositories
crates/migration  sea-orm-migration, shared by all apps
deploy/           deployment placeholder
```

Planned: `crates/bootstrap`, `apps/{apis, apis-internal, reports}`.

Dependency rule: `apps/* -> bootstrap -> infra -> core`. `core` never depends on axum, sea-orm, redis, etc.

## Dev

```sh
cp .env.example .env
docker compose up -d
export $(cat .env | xargs)

cargo run -p migration -- up          # apply migrations (uses DATABASE_URL)
cargo test --workspace                # unit tests; DB tests skipped

# DB integration tests (use a separate database, they leave test rows behind)
docker compose exec mysql mysql -uroot -proot -e "CREATE DATABASE IF NOT EXISTS app_test; GRANT ALL ON app_test.* TO 'app'@'%'"
TEST_DATABASE_URL=mysql://app:app@127.0.0.1:3306/app_test cargo test -p infra
```
