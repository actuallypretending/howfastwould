-- Seed approximate benchmark results for all problems × active AI models.
-- Uses ON CONFLICT DO NOTHING so it won't overwrite real data.
-- Run: psql $DATABASE_URL -f scripts/seed-benchmarks.sql

-- Model tiers: assign base times (ms) and solve rates by difficulty.
-- Jitter is derived from model name length for deterministic variation.
WITH model_tiers AS (
  SELECT
    m.id AS model_id,
    m.name,
    -- Base times: (easy_ms, medium_ms, hard_ms)
    CASE
      WHEN m.name ~ '(o3|o4)'                          THEN 4000
      WHEN m.name LIKE '%opus%'                         THEN 5000
      WHEN m.name LIKE '%deepseek-reasoner%'            THEN 6000
      WHEN m.name LIKE '%sonnet%'                       THEN 3000
      WHEN m.name LIKE '%gemini-2.5%'                   THEN 4000
      WHEN m.name LIKE '%gpt-4.5%'                      THEN 5000
      WHEN m.name LIKE '%grok%'                         THEN 5000
      WHEN m.name LIKE '%deepseek-chat%'                THEN 4000
      WHEN m.name LIKE '%gemini-2.0%' OR m.name LIKE '%flash%' THEN 2500
      WHEN m.name LIKE '%4o-mini%'                      THEN 3000
      WHEN m.name LIKE '%mistral-large%'                THEN 6000
      WHEN m.name LIKE '%qwen%' OR m.name LIKE '%qwq%' THEN 5000
      WHEN m.name LIKE '%llama-4%'                      THEN 5000
      WHEN m.name LIKE '%llama-3.3-70b%'                THEN 4000
      WHEN m.name LIKE '%Llama-3-70B%'                  THEN 5000
      WHEN m.name LIKE '%mixtral%'                      THEN 4500
      WHEN m.name LIKE '%8b-instruct%' OR m.name LIKE '%8b%' THEN 3000
      WHEN m.name LIKE '%moonshot%'                     THEN 6000
      WHEN m.name LIKE '%doubao%'                       THEN 7000
      WHEN m.name LIKE '%hunyuan%'                      THEN 7000
      ELSE 6000
    END AS easy_ms,
    CASE
      WHEN m.name ~ '(o3|o4)'                          THEN 12000
      WHEN m.name LIKE '%opus%'                         THEN 14000
      WHEN m.name LIKE '%deepseek-reasoner%'            THEN 16000
      WHEN m.name LIKE '%sonnet%'                       THEN 10000
      WHEN m.name LIKE '%gemini-2.5%'                   THEN 12000
      WHEN m.name LIKE '%gpt-4.5%'                      THEN 15000
      WHEN m.name LIKE '%grok%'                         THEN 14000
      WHEN m.name LIKE '%deepseek-chat%'                THEN 13000
      WHEN m.name LIKE '%gemini-2.0%' OR m.name LIKE '%flash%' THEN 8000
      WHEN m.name LIKE '%4o-mini%'                      THEN 10000
      WHEN m.name LIKE '%mistral-large%'                THEN 18000
      WHEN m.name LIKE '%qwen%' OR m.name LIKE '%qwq%' THEN 16000
      WHEN m.name LIKE '%llama-4%'                      THEN 16000
      WHEN m.name LIKE '%llama-3.3-70b%'                THEN 14000
      WHEN m.name LIKE '%Llama-3-70B%'                  THEN 18000
      WHEN m.name LIKE '%mixtral%'                      THEN 16000
      WHEN m.name LIKE '%8b-instruct%' OR m.name LIKE '%8b%' THEN 12000
      WHEN m.name LIKE '%moonshot%'                     THEN 20000
      WHEN m.name LIKE '%doubao%'                       THEN 22000
      WHEN m.name LIKE '%hunyuan%'                      THEN 22000
      ELSE 18000
    END AS med_ms,
    CASE
      WHEN m.name ~ '(o3|o4)'                          THEN 25000
      WHEN m.name LIKE '%opus%'                         THEN 30000
      WHEN m.name LIKE '%deepseek-reasoner%'            THEN 35000
      WHEN m.name LIKE '%sonnet%'                       THEN 22000
      WHEN m.name LIKE '%gemini-2.5%'                   THEN 28000
      WHEN m.name LIKE '%gpt-4.5%'                      THEN 35000
      WHEN m.name LIKE '%grok%'                         THEN 32000
      WHEN m.name LIKE '%deepseek-chat%'                THEN 30000
      WHEN m.name LIKE '%gemini-2.0%' OR m.name LIKE '%flash%' THEN 20000
      WHEN m.name LIKE '%4o-mini%'                      THEN 25000
      WHEN m.name LIKE '%mistral-large%'                THEN 40000
      WHEN m.name LIKE '%qwen%' OR m.name LIKE '%qwq%' THEN 38000
      WHEN m.name LIKE '%llama-4%'                      THEN 38000
      WHEN m.name LIKE '%llama-3.3-70b%'                THEN 35000
      WHEN m.name LIKE '%Llama-3-70B%'                  THEN 45000
      WHEN m.name LIKE '%mixtral%'                      THEN 42000
      WHEN m.name LIKE '%8b-instruct%' OR m.name LIKE '%8b%' THEN 35000
      WHEN m.name LIKE '%moonshot%'                     THEN 50000
      WHEN m.name LIKE '%doubao%'                       THEN 55000
      WHEN m.name LIKE '%hunyuan%'                      THEN 55000
      ELSE 45000
    END AS hard_ms,
    -- Solve rates (0-100 scale for integer math)
    CASE
      WHEN m.name ~ '(o3|o4)'                          THEN ARRAY[98,92,75]
      WHEN m.name LIKE '%opus%'                         THEN ARRAY[97,90,72]
      WHEN m.name LIKE '%deepseek-reasoner%'            THEN ARRAY[95,88,68]
      WHEN m.name LIKE '%sonnet%'                       THEN ARRAY[96,88,65]
      WHEN m.name LIKE '%gemini-2.5%'                   THEN ARRAY[96,89,70]
      WHEN m.name LIKE '%gpt-4.5%'                      THEN ARRAY[95,85,60]
      WHEN m.name LIKE '%grok%'                         THEN ARRAY[94,84,58]
      WHEN m.name LIKE '%deepseek-chat%'                THEN ARRAY[94,85,60]
      WHEN m.name LIKE '%gemini-2.0%' OR m.name LIKE '%flash%' THEN ARRAY[93,82,55]
      WHEN m.name LIKE '%4o-mini%'                      THEN ARRAY[90,75,45]
      WHEN m.name LIKE '%mistral-large%'                THEN ARRAY[92,80,52]
      WHEN m.name LIKE '%qwen%' OR m.name LIKE '%qwq%' THEN ARRAY[90,78,48]
      WHEN m.name LIKE '%llama-4%'                      THEN ARRAY[90,78,50]
      WHEN m.name LIKE '%llama-3.3-70b%'                THEN ARRAY[88,72,40]
      WHEN m.name LIKE '%Llama-3-70B%'                  THEN ARRAY[85,65,32]
      WHEN m.name LIKE '%mixtral%'                      THEN ARRAY[82,60,28]
      WHEN m.name LIKE '%8b-instruct%' OR m.name LIKE '%8b%' THEN ARRAY[70,45,18]
      WHEN m.name LIKE '%moonshot%'                     THEN ARRAY[80,60,30]
      WHEN m.name LIKE '%doubao%'                       THEN ARRAY[75,55,25]
      WHEN m.name LIKE '%hunyuan%'                      THEN ARRAY[75,55,25]
      ELSE ARRAY[80,60,30]
    END AS solve_rates
  FROM models m
  WHERE m.is_active = true AND m.is_human = false
),
cross_join AS (
  SELECT
    p.id AS problem_id,
    p.difficulty,
    mt.model_id,
    mt.name AS model_name,
    mt.easy_ms,
    mt.med_ms,
    mt.hard_ms,
    mt.solve_rates,
    -- Deterministic jitter: 0.7x to 1.3x based on hash of model name + problem id
    0.7 + (abs(hashtext(mt.name || p.id)) % 7) * 0.1 AS jitter
  FROM problems p
  CROSS JOIN model_tiers mt
),
computed AS (
  SELECT
    problem_id,
    model_id,
    model_name,
    -- Pick base time by difficulty, apply jitter
    CASE difficulty
      WHEN 'Easy'   THEN (easy_ms * jitter)::bigint
      WHEN 'Medium' THEN (med_ms * jitter)::bigint
      WHEN 'Hard'   THEN (hard_ms * jitter)::bigint
      ELSE (med_ms * jitter)::bigint
    END AS time_ms,
    -- Determine solved using deterministic hash as coin flip vs solve rate
    CASE difficulty
      WHEN 'Easy'   THEN (abs(hashtext(model_name || problem_id || 'solve')) % 100) < solve_rates[1]
      WHEN 'Medium' THEN (abs(hashtext(model_name || problem_id || 'solve')) % 100) < solve_rates[2]
      WHEN 'Hard'   THEN (abs(hashtext(model_name || problem_id || 'solve')) % 100) < solve_rates[3]
      ELSE (abs(hashtext(model_name || problem_id || 'solve')) % 100) < solve_rates[2]
    END AS solved
  FROM cross_join
)
INSERT INTO results (id, problem_id, model_id, solved, time_ms, attempts, run_at)
SELECT
  gen_random_uuid()::text,
  problem_id,
  model_id,
  solved,
  CASE WHEN solved THEN time_ms ELSE NULL END,
  1,
  now()::text
FROM computed
ON CONFLICT (problem_id, model_id) DO NOTHING;
