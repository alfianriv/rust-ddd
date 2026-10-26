# Deploy (placeholder)

Not decided yet. To fill in:

- [ ] Target: Docker Compose / Kubernetes / other
- [ ] Image per app: `apis`, `apis-internal`, `reports` (root `Dockerfile`, `--build-arg APP=<name>`)
- [ ] Build cache with `cargo chef`
- [ ] Migration run step (before rollout)
- [ ] Secrets: `DATABASE_URL`, `REDIS_URL`, `JWT_SECRET`, internal API keys
- [ ] Health check endpoints per app
