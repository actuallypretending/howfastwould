CREATE TABLE submissions (
    id           TEXT PRIMARY KEY,
    problem_id   TEXT NOT NULL REFERENCES problems(id),
    ip_hash      TEXT NOT NULL,
    solved       BOOLEAN NOT NULL DEFAULT false,
    time_ms      BIGINT,
    attempts     BIGINT NOT NULL DEFAULT 1,
    code         TEXT NOT NULL,
    submitted_at TEXT NOT NULL
);

CREATE INDEX idx_submissions_problem_id ON submissions(problem_id);

CREATE TABLE execution_details (
    id           TEXT PRIMARY KEY,
    result_id    TEXT NOT NULL REFERENCES results(id) ON DELETE CASCADE,
    code         TEXT NOT NULL,
    test_results TEXT NOT NULL,
    stderr       TEXT NOT NULL DEFAULT ''
);

CREATE UNIQUE INDEX idx_execution_details_result_id ON execution_details(result_id);
