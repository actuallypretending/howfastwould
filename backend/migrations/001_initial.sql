CREATE TABLE IF NOT EXISTS problems (
    id           TEXT PRIMARY KEY,
    lc_id        BIGINT NOT NULL,
    title        TEXT NOT NULL,
    difficulty   TEXT NOT NULL CHECK(difficulty IN ('Easy','Medium','Hard')),
    description  TEXT NOT NULL,
    starter_code TEXT NOT NULL,
    test_cases   TEXT NOT NULL,
    source       TEXT NOT NULL DEFAULT 'leetcode',
    cached_at    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS models (
    id           TEXT PRIMARY KEY,
    provider     TEXT NOT NULL,
    name         TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    api_key_env  TEXT NOT NULL,
    is_active    BOOLEAN NOT NULL DEFAULT true,
    is_new       BOOLEAN NOT NULL DEFAULT false,
    is_human     BOOLEAN NOT NULL DEFAULT false,
    human_times  TEXT,
    added_at     TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS results (
    id         TEXT PRIMARY KEY,
    problem_id TEXT NOT NULL REFERENCES problems(id),
    model_id   TEXT NOT NULL REFERENCES models(id),
    solved     BOOLEAN NOT NULL DEFAULT false,
    time_ms    BIGINT,
    attempts   BIGINT NOT NULL DEFAULT 1,
    run_at     TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS races (
    id          TEXT PRIMARY KEY,
    problem_id  TEXT NOT NULL REFERENCES problems(id),
    started_at  TEXT NOT NULL,
    finished_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_results_problem_id ON results(problem_id);
