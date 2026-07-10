-- D1.1 Project Identity Kernel
--
-- D0 already created workspaces and a minimal projects table. Rebuilding the
-- projects table lets D1.1 add complete constraints while preserving D0 rows.

-- ForgeOS D1.1 supports exactly one Workspace. The expression index keeps the
-- invariant in SQLite as well as in the application use case.
CREATE UNIQUE INDEX workspaces_singleton_index ON workspaces ((1));

CREATE TABLE projects_d1 (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL,
    slug TEXT COLLATE NOCASE NOT NULL
        CHECK (length(trim(slug)) BETWEEN 2 AND 64),
    name TEXT NOT NULL
        CHECK (length(trim(name)) BETWEEN 1 AND 200),
    description TEXT NOT NULL DEFAULT ''
        CHECK (length(description) <= 2000),
    vision TEXT NOT NULL DEFAULT '',
    target_user TEXT NOT NULL DEFAULT '',
    scope TEXT NOT NULL DEFAULT '',
    non_goals TEXT NOT NULL DEFAULT '',
    tech_stack TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'idea'
        CHECK (status IN (
            'idea',
            'planned',
            'active',
            'paused',
            'maintenance',
            'archived'
        )),
    lifecycle_stage TEXT NOT NULL DEFAULT 'discovery'
        CHECK (lifecycle_stage IN (
            'discovery',
            'definition',
            'design',
            'implementation',
            'validation',
            'release',
            'operation',
            'retrospective'
        )),
    priority INTEGER NOT NULL DEFAULT 50
        CHECK (priority BETWEEN 0 AND 100),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1
        CHECK (version >= 1),
    CONSTRAINT projects_workspace_fk
        FOREIGN KEY (workspace_id)
        REFERENCES workspaces (id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    CONSTRAINT projects_workspace_slug_unique
        UNIQUE (workspace_id, slug)
);

INSERT INTO projects_d1 (
    id,
    workspace_id,
    slug,
    name,
    description,
    vision,
    target_user,
    scope,
    non_goals,
    tech_stack,
    status,
    lifecycle_stage,
    priority,
    created_at,
    updated_at,
    version
)
SELECT
    id,
    workspace_id,
    slug,
    name,
    COALESCE(description, ''),
    '',
    '',
    '',
    '',
    '',
    'idea',
    'discovery',
    50,
    created_at,
    updated_at,
    1
FROM projects;

DROP TABLE projects;
ALTER TABLE projects_d1 RENAME TO projects;

CREATE INDEX projects_workspace_index ON projects (workspace_id);
CREATE INDEX projects_workspace_updated_index
    ON projects (workspace_id, updated_at DESC);
CREATE INDEX projects_workspace_status_updated_index
    ON projects (workspace_id, status, updated_at DESC);
CREATE INDEX projects_workspace_lifecycle_index
    ON projects (workspace_id, lifecycle_stage);
