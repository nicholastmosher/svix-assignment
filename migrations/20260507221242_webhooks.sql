-- Add migration script here
CREATE TABLE webhooks (
    id TEXT PRIMARY KEY,
    deadline TEXT NOT NULL,
    url TEXT NOT NULL,
    body TEXT NOT NULL,
    -- Null before execution, non-null after
    executed_at TEXT
);
