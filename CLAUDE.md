# howfastwould

Monorepo for [howfastwould.com](https://howfastwould.com) — a meme site that races AI models against each other on LeetCode problems and lets users generate roast memes from the results.

## Structure

- `backend/` — Rust API server (Axum 0.7, SQLx 0.7, Postgres)
- `frontend/` — Next.js 16 + React 19 + Tailwind 4 + TypeScript
- `docs/` — project documentation

## Backend

- **Language:** Rust 2021 edition
- **Framework:** Axum 0.7 with tower-http CORS
- **Database:** PostgreSQL via SQLx 0.7 (migrations in `backend/migrations/`)
- **Code execution:** Judge0 CE (runs AI-generated solutions in a sandbox)
- **AI providers:** OpenAI, Anthropic, Google, xAI, Fireworks, DeepSeek, Qwen, Moonshot, Doubao, Hunyuan, Mistral
- **Key modules:**
  - `src/routes/` — API endpoints (problems, models, races)
  - `src/runner.rs` — orchestrates model code generation + execution
  - `src/roast.rs` — generates meme roasts
  - `src/sync.rs` — seeds and syncs model list
  - `src/leetcode.rs` — LeetCode problem fetching
  - `src/piston.rs` — Judge0 CE sandbox client
- **Run:** `cargo run` from `backend/` (default port 3001)
- **Config:** all via env vars — `DATABASE_URL`, `PORT`, `PISTON_URL`, `ALLOWED_ORIGINS`, plus API keys per provider

## Frontend

- **Framework:** Next.js 16 (App Router) + React 19
- **Styling:** Tailwind CSS 4
- **Key components:** MemeCard, ProblemHeader, RaceEditor, RaceResults, SearchBar, WinnerCard
- **API client:** `app/lib/api.ts`
- **Types:** `app/lib/types.ts`
- **Run:** `npm run dev` from `frontend/`

## Development

- Backend listens on port 3001, frontend on port 3000 by default
- Backend syncs model list on startup and every 24h
- Background benchmarks spawn automatically when models are missing results for a problem
