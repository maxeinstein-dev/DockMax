-- Migration 1: widget instances, their notes and the database's own state
-- (spec 0003 §2.4 item 4). Tables are STRICT so a value of the wrong type is
-- refused by SQLite instead of being stored as something else.

-- One row per widget in the dock. The frontend never sees `position`,
-- `config_version` or `created_at`: the order of the list it gets is the
-- order of the dock.
CREATE TABLE widget_instances (
  id TEXT PRIMARY KEY NOT NULL,      -- UUID v7, never reused
  kind TEXT NOT NULL,                -- WidgetKind in kebab-case
  label TEXT,                        -- NULL = the kind's name
  position INTEGER NOT NULL,         -- 0-based, contiguous, order in the dock
  config TEXT NOT NULL,              -- JSON of the kind's config
  config_version INTEGER NOT NULL,   -- format version of `config`, per kind
  created_at INTEGER NOT NULL        -- Unix time in milliseconds
) STRICT;

-- The text of a note widget, one row per instance at most. Removing the
-- instance removes its text (`foreign_keys` is turned on per connection).
CREATE TABLE notes (
  instance_id TEXT PRIMARY KEY NOT NULL
    REFERENCES widget_instances(id) ON DELETE CASCADE,
  body TEXT NOT NULL DEFAULT '',
  updated_at INTEGER NOT NULL        -- Unix time in milliseconds
) STRICT;

-- Internal state of the database itself, such as the pending recovery
-- notice (`recovery_notice` = `pending`). Never user data.
CREATE TABLE meta (
  key TEXT PRIMARY KEY NOT NULL,
  value TEXT NOT NULL
) STRICT;
