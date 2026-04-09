-- Convert starter_code from plain text to JSONB
ALTER TABLE problems
  ALTER COLUMN starter_code TYPE JSONB
  USING jsonb_build_object('python3', starter_code);

-- Add language to submissions
ALTER TABLE submissions ADD COLUMN language TEXT NOT NULL DEFAULT 'python3';
