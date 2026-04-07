# Hosting Design: howfastwould.com

## Goal

Deploy howfastwould — Rust/Axum backend + Next.js frontend — to production at `howfastwould.com`, with Railway handling compute and Vercel handling the frontend.

## Architecture

```
howfastwould.com         → Vercel (Next.js, CDN-served)
api.howfastwould.com     → Railway backend (Axum)

Railway project: "howfastwould"
  ├── postgres   (Railway managed Postgres plugin)
  ├── piston     (Docker: ghcr.io/engineer-man/piston + persistent volume)
  └── backend    (Rust binary, Nixpacks build, SQLX_OFFLINE=true)

DNS: Cloudflare (registered at Cloudflare Registrar)
```

All Railway services communicate over Railway's **private internal network**. Postgres and Piston have no public URLs — only the backend service is publicly exposed.

---

## PostgreSQL Migration

The backend currently uses SQLite. Migrating to Railway's managed Postgres requires the following code changes:

### Cargo.toml
Replace the `sqlite` sqlx feature with `postgres`:
```toml
sqlx = { version = "0.7", features = ["runtime-tokio", "postgres", "migrate", "json"] }
```

### Pool type
Replace `SqlitePool` / `SqlitePoolOptions` with `PgPool` / `PgPoolOptions` throughout `db.rs`, `routes/`, `sync.rs`, and anywhere `SqlitePool` is referenced.

### Schema (`migrations/001_initial.sql`)
- Replace boolean `INTEGER` columns with `BOOLEAN` (allows using real Rust `bool` in models)
- Replace `INSERT OR REPLACE` with `INSERT ... ON CONFLICT (id) DO UPDATE SET ...`
- `ORDER BY ... NULLS LAST`, `CHECK`, `REFERENCES` — all standard SQL, no changes needed

Updated boolean columns in models.rs: `is_active`, `is_new`, `is_human`, `solved` can return to `bool` (removing the `i64` workaround and `i64_as_bool` serializer).

### Query macros
Postgres properly infers `NOT NULL` from schema constraints, so the `"field!" ` override alias pattern used throughout can be replaced with plain `SELECT *` or simple column lists.

`COUNT(*)` in `sync.rs` returns `i64` in Postgres (currently `i32` for SQLite) — update `query_scalar!` result type.

### sqlx offline mode
Railway builds without a live database available. Generate `.sqlx/` query metadata locally and commit it:

```bash
# From backend/
DATABASE_URL=postgresql://... cargo sqlx prepare
git add .sqlx/
git commit -m "chore: add sqlx offline query cache"
```

Set `SQLX_OFFLINE=true` as a Railway build environment variable.

---

## Railway Services

### 1. Postgres
Use Railway's built-in Postgres plugin. Railway automatically sets `${{Postgres.DATABASE_URL}}` as a reference variable — wire this to the backend service's `DATABASE_URL` env var in the Railway dashboard.

### 2. Piston
- **Image**: `ghcr.io/engineer-man/piston`
- **Volume**: mount a Railway persistent volume at `/piston/packages` — installed runtimes survive redeploys
- **Port**: 2000 (internal only, no public domain)
- **First-deploy setup** (one-time manual step after initial deploy):
  ```bash
  # From Railway shell or via curl from backend service
  curl -X POST http://piston.railway.internal:2000/api/v2/runtimes \
    -H "Content-Type: application/json" \
    -d '{"language": "python", "version": "3.10.0"}'
  ```
- No public domain assigned — internal access only

### 3. Backend
- **Build**: Nixpacks auto-detects Rust. Root directory: `backend/`
- **Start command**: `./server`
- **Build env var**: `SQLX_OFFLINE=true`
- **Runtime env vars** (set in Railway dashboard):

| Variable | Value |
|---|---|
| `DATABASE_URL` | `${{Postgres.DATABASE_URL}}` |
| `PORT` | (set by Railway automatically — no manual value needed) |
| `PISTON_URL` | `http://piston.railway.internal:2000` |
| `ANTHROPIC_API_KEY` | `sk-ant-...` |
| `OPENAI_API_KEY` | `sk-...` |
| `GOOGLE_API_KEY` | `...` |
| `XAI_API_KEY` | `...` |
| `FIREWORKS_API_KEY` | `...` |
| `DEEPSEEK_API_KEY` | `...` |
| `QWEN_API_KEY` | `...` |
| `MOONSHOT_API_KEY` | `...` |
| `DOUBAO_API_KEY` | `...` |
| `HUNYUAN_API_KEY` | `...` |
| `MISTRAL_API_KEY` | `...` |

- **Custom domain**: `api.howfastwould.com` (added in Railway dashboard after deploy)
- **Auto-deploy**: on push to `master`

`railway.toml` in `backend/`:
```toml
[build]
builder = "nixpacks"

[deploy]
startCommand = "./server"

[build.nixpacksPlan.variables]
SQLX_OFFLINE = "true"
```

---

## Vercel (Frontend)

- Connect GitHub repo to Vercel
- **Root directory**: `frontend/`
- **Framework**: Next.js (auto-detected)
- **Env var**: `NEXT_PUBLIC_API_URL=https://api.howfastwould.com`
- **Custom domains**: `howfastwould.com` and `www.howfastwould.com` (added in Vercel dashboard)
- **Redirect**: `www.howfastwould.com` → `howfastwould.com` (configured in Vercel dashboard)
- **Auto-deploy**: on push to `master`

---

## DNS (Cloudflare)

### Domain registration
Register `howfastwould.com` directly at [registrar.cloudflare.com](https://registrar.cloudflare.com) (~$10/yr). Nameservers are already Cloudflare — no transfer step needed.

### SSL mode
Set Cloudflare SSL/TLS mode to **Full** (not Flexible). Both Vercel and Railway issue their own TLS certificates. Using Flexible causes redirect loops.

### DNS records

| Name | Type | Value | Proxied |
|---|---|---|---|
| `howfastwould.com` | A | `76.76.21.21` | Yes ✓ |
| `www` | CNAME | `cname.vercel-dns.com` | Yes ✓ |
| `api` | CNAME | `<backend>.up.railway.app` | Yes ✓ |

The Railway backend URL (e.g. `backend-production-xxxx.up.railway.app`) is found in the Railway dashboard after the first deploy.

### What Cloudflare provides
- DDoS protection (useful if the site gets viral traction)
- One place to manage all DNS
- Free analytics and traffic overview
- Easy to add caching rules or Cloudflare Workers later

---

## Deploy Order

1. Create Railway project, add Postgres plugin
2. Deploy Piston service, attach volume, install Python runtime (one-time)
3. Make Postgres migration code changes locally, run `cargo sqlx prepare`, commit `.sqlx/`
4. Add `backend/railway.toml`
5. Deploy backend service on Railway, set all env vars, add custom domain
6. Connect frontend to Vercel, set `NEXT_PUBLIC_API_URL`, add custom domains
7. Register `howfastwould.com` at Cloudflare, add DNS records, set SSL to Full
8. Verify end-to-end: `https://howfastwould.com` → frontend, `https://api.howfastwould.com/models` → JSON
