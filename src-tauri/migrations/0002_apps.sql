-- Migration 2: the programs pinned to the dock (spec 0006 §3.2). STRICT, like
-- the first migration, so a value of the wrong type is refused.

-- One row per pinned program. The frontend never sees `position` or
-- `created_at`: the order of the list it gets is the order of the dock.
CREATE TABLE pinned_apps (
  id TEXT PRIMARY KEY NOT NULL,                 -- UUID v7, never reused
  app_id TEXT NOT NULL UNIQUE COLLATE NOCASE,   -- parsing name in the shell's Apps folder
  name TEXT NOT NULL,                           -- the name the shell showed when it was pinned
  position INTEGER NOT NULL,                    -- 0-based, contiguous, order in the dock
  created_at INTEGER NOT NULL                   -- Unix time in milliseconds
) STRICT;
