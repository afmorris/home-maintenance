CREATE TABLE IF NOT EXISTS locations (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    name TEXT NOT NULL,
    kind TEXT NOT NULL CHECK(kind IN ('room','floor','outbuilding','outdoor','other')),
    parent_id TEXT NULL REFERENCES locations(id)
);

CREATE TABLE IF NOT EXISTS assets (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    name TEXT NOT NULL,
    location_id TEXT NULL REFERENCES locations(id),
    category TEXT NOT NULL,
    make TEXT,
    model TEXT,
    serial TEXT,
    install_date TEXT,
    warranty_end TEXT,
    notes TEXT,
    archived INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS supplies (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    name TEXT NOT NULL,
    spec TEXT,
    purchase_url TEXT,
    notes TEXT
);

CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    asset_id TEXT NULL REFERENCES assets(id),
    name TEXT NOT NULL,
    description TEXT,
    schedule_mode TEXT NOT NULL CHECK(schedule_mode IN ('floating','fixed')),
    interval_value INTEGER NULL,
    interval_unit TEXT NULL CHECK(interval_unit IN ('day','week','month','year')),
    season_anchor TEXT NULL,
    fixed_interval_years INTEGER NULL DEFAULT 1,
    estimated_minutes INTEGER NULL,
    active INTEGER NOT NULL DEFAULT 1,
    CHECK (
        (schedule_mode = 'floating' AND interval_value IS NOT NULL AND interval_unit IS NOT NULL)
        OR (schedule_mode = 'fixed' AND season_anchor IS NOT NULL)
    )
);

CREATE TABLE IF NOT EXISTS task_supplies (
    task_id TEXT NOT NULL REFERENCES tasks(id),
    supply_id TEXT NOT NULL REFERENCES supplies(id),
    quantity TEXT,
    PRIMARY KEY (task_id, supply_id)
);

CREATE TABLE IF NOT EXISTS reminders (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    task_id TEXT NOT NULL UNIQUE REFERENCES tasks(id),
    due_date TEXT NOT NULL,
    snoozed_until TEXT NULL,
    last_notified_at TEXT NULL
);

CREATE TABLE IF NOT EXISTS log_entries (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    task_id TEXT NULL REFERENCES tasks(id),
    asset_id TEXT NULL REFERENCES assets(id),
    kind TEXT NOT NULL CHECK(kind IN ('service','repair','upgrade','inspection')),
    scheduled_date TEXT NULL,
    completed_date TEXT NOT NULL,
    cost_cents INTEGER NULL,
    vendor TEXT,
    performed_by TEXT,
    notes TEXT
);

CREATE TABLE IF NOT EXISTS attachments (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    owner_type TEXT NOT NULL CHECK(owner_type IN ('asset','log_entry')),
    owner_id TEXT NOT NULL,
    filename TEXT NOT NULL,
    content_type TEXT NOT NULL,
    byte_size INTEGER NOT NULL,
    caption TEXT
);

CREATE TABLE IF NOT EXISTS tags (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS entry_tags (
    entry_id TEXT NOT NULL REFERENCES log_entries(id),
    tag_id TEXT NOT NULL REFERENCES tags(id),
    PRIMARY KEY (entry_id, tag_id)
);

CREATE INDEX IF NOT EXISTS idx_reminders_due_date ON reminders(due_date);
CREATE INDEX IF NOT EXISTS idx_log_entries_asset_completed ON log_entries(asset_id, completed_date);
CREATE INDEX IF NOT EXISTS idx_log_entries_completed ON log_entries(completed_date);
CREATE INDEX IF NOT EXISTS idx_attachments_owner ON attachments(owner_type, owner_id);
