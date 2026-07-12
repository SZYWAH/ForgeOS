-- D1.5 Context Pack Kernel
--
-- Context exports are immutable derived records built from confirmed Project
-- facts. Only delivery status and its optimistic version may change.

CREATE TABLE context_exports (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,
    template_type TEXT NOT NULL
        CHECK (template_type IN ('planning', 'coding', 'debugging', 'review', 'handoff')),
    snapshot_version INTEGER
        CHECK (snapshot_version IS NULL OR snapshot_version >= 1),
    generated_from_session_ids TEXT NOT NULL DEFAULT '[]'
        CHECK (json_valid(generated_from_session_ids) AND json_type(generated_from_session_ids) = 'array'),
    content_markdown TEXT NOT NULL
        CHECK (length(content_markdown) > 0),
    content_hash TEXT NOT NULL
        CHECK (length(content_hash) = 64 AND content_hash NOT GLOB '*[^0-9a-f]*'),
    status TEXT NOT NULL DEFAULT 'generated'
        CHECK (status IN ('generated', 'sent_to_ai')),
    created_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1
        CHECK (version >= 1),
    CONSTRAINT context_exports_project_fk
        FOREIGN KEY (project_id)
        REFERENCES projects (id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
);

CREATE INDEX context_exports_project_created_index
    ON context_exports (project_id, created_at DESC, id DESC);

CREATE INDEX context_exports_project_template_index
    ON context_exports (project_id, template_type, created_at DESC);
