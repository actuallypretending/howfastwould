CREATE TABLE IF NOT EXISTS problems (
    id          TEXT PRIMARY KEY,
    lc_id       INTEGER NOT NULL,
    title       TEXT NOT NULL,
    difficulty  TEXT NOT NULL CHECK(difficulty IN ('Easy','Medium','Hard')),
    description TEXT NOT NULL,
    starter_code TEXT NOT NULL,
    test_cases  TEXT NOT NULL,
    source      TEXT NOT NULL DEFAULT 'leetcode',
    cached_at   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS models (
    id          TEXT PRIMARY KEY,
    provider    TEXT NOT NULL,
    name        TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    api_key_env TEXT NOT NULL,
    is_active   INTEGER NOT NULL DEFAULT 1,
    is_new      INTEGER NOT NULL DEFAULT 0,
    is_human    INTEGER NOT NULL DEFAULT 0,
    human_times TEXT,
    added_at    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS results (
    id          TEXT PRIMARY KEY,
    problem_id  TEXT NOT NULL REFERENCES problems(id),
    model_id    TEXT NOT NULL REFERENCES models(id),
    solved      INTEGER NOT NULL DEFAULT 0,
    time_ms     INTEGER,
    attempts    INTEGER NOT NULL DEFAULT 1,
    run_at      TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS races (
    id          TEXT PRIMARY KEY,
    problem_id  TEXT NOT NULL REFERENCES problems(id),
    started_at  TEXT NOT NULL,
    finished_at TEXT
);
