-- Add migration script here

CREATE TABLE webhooks (
    id TEXT PRIMARY KEY NOT NULL,
    deadline TEXT NOT NULL,
    url TEXT NOT NULL,
    body TEXT NOT NULL,
    -- Null before execution, non-null after
    executed_at TEXT
);

CREATE TABLE hashtasks (
    id TEXT PRIMARY KEY NOT NULL,
    deadline TEXT NOT NULL,
    secret TEXT NOT NULL,
    executed_at TEXT
);
