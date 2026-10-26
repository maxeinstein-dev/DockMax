-- Migration 3: what the launcher opened (spec 0012 §5). STRICT, like the
-- others, so a value of the wrong type is refused.

-- One row per entry opened through the launcher: its weight when it was last
-- opened, which halves every 14 days (`domain::launcher::decay`), and when
-- that was. Only the launcher writes here; the dock never does.
CREATE TABLE launcher_usage (
  key TEXT PRIMARY KEY NOT NULL,   -- `app:<parsing name>`
  score REAL NOT NULL,             -- the weight at `last_used`
  last_used INTEGER NOT NULL       -- Unix time in milliseconds
) STRICT;
