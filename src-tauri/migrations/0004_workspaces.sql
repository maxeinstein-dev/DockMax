-- Migration 4: workspaces (spec 0013 C1). STRICT, like the others, so a value
-- of the wrong type is refused.

-- One row per workspace. Its position orders the switcher's dots and the
-- Ctrl+Alt+number shortcuts.
CREATE TABLE workspaces (
  id TEXT PRIMARY KEY NOT NULL,      -- UUID v7, never reused
  name TEXT NOT NULL,
  color TEXT NOT NULL,               -- one of the palette, #RRGGBB
  position INTEGER NOT NULL,         -- 0-based, contiguous
  created_at INTEGER NOT NULL        -- Unix time in milliseconds
) STRICT;

-- What each workspace shows, in dock order: the keys the dock uses
-- (`app:<pinned_apps.id>`, `widget:<widget_instances.id>`). The programs and
-- the widgets themselves stay in their tables; an item leaves them only when
-- no workspace has it. The first workspace is made by Rust the first time
-- the database is read after this migration, with what the dock showed.
CREATE TABLE workspace_items (
  workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  item_key TEXT NOT NULL,
  position INTEGER NOT NULL,         -- 0-based, contiguous within the workspace
  PRIMARY KEY (workspace_id, item_key)
) STRICT;
