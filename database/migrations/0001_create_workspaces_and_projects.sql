CREATE TABLE workspaces (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    timezone TEXT NOT NULL DEFAULT 'Asia/Shanghai',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE projects (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL,
    slug TEXT COLLATE NOCASE NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CONSTRAINT projects_workspace_fk
        FOREIGN KEY (workspace_id)
        REFERENCES workspaces (id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    CONSTRAINT projects_workspace_slug_unique
        UNIQUE (workspace_id, slug)
);

CREATE INDEX projects_workspace_index ON projects (workspace_id);

