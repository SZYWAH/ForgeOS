-- D1.2 Current Snapshot Kernel
--
-- Snapshot rows are immutable. The current state of a Project is the row with
-- the highest version; updates append a new row instead of overwriting history.

CREATE TABLE project_snapshots (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,
    version INTEGER NOT NULL
        CHECK (version >= 1),
    current_goal TEXT NOT NULL
        CHECK (length(trim(current_goal)) > 0),
    status_summary TEXT NOT NULL
        CHECK (length(trim(status_summary)) > 0),
    achievements TEXT NOT NULL DEFAULT '[]'
        CHECK (json_valid(achievements) AND json_type(achievements) = 'array'),
    blockers TEXT NOT NULL DEFAULT '[]'
        CHECK (json_valid(blockers) AND json_type(blockers) = 'array'),
    next_steps TEXT NOT NULL DEFAULT '[]'
        CHECK (json_valid(next_steps) AND json_type(next_steps) = 'array'),
    confirmed_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CONSTRAINT project_snapshots_project_fk
        FOREIGN KEY (project_id)
        REFERENCES projects (id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    CONSTRAINT project_snapshots_project_version_unique
        UNIQUE (project_id, version)
);

CREATE INDEX project_snapshots_project_version_index
    ON project_snapshots (project_id, version DESC);
