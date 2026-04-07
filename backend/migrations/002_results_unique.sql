CREATE UNIQUE INDEX IF NOT EXISTS idx_results_problem_model
    ON results(problem_id, model_id);
