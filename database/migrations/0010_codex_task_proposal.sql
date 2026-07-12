-- D4 Codex Task Proposal Foundation

CREATE TABLE codex_task_proposals (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,
    context_export_id TEXT NOT NULL,
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    description TEXT NOT NULL DEFAULT '',
    goal TEXT NOT NULL CHECK (length(trim(goal)) > 0),
    non_goals TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(non_goals) AND json_type(non_goals) = 'array'),
    allowed_scope TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(allowed_scope) AND json_type(allowed_scope) = 'array'),
    forbidden_scope TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(forbidden_scope) AND json_type(forbidden_scope) = 'array'),
    acceptance_criteria TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(acceptance_criteria) AND json_type(acceptance_criteria) = 'array'),
    validation_commands TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(validation_commands) AND json_type(validation_commands) = 'array'),
    status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'proposed', 'approved', 'rejected', 'archived')),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CONSTRAINT codex_task_proposals_project_fk FOREIGN KEY (project_id)
        REFERENCES projects (id) ON UPDATE CASCADE ON DELETE RESTRICT,
    CONSTRAINT codex_task_proposals_context_project_fk FOREIGN KEY (context_export_id, project_id)
        REFERENCES context_exports (id, project_id) ON UPDATE CASCADE ON DELETE RESTRICT
);

CREATE INDEX codex_task_proposals_project_status_updated_index
    ON codex_task_proposals (project_id, status, updated_at DESC, created_at DESC);
CREATE INDEX codex_task_proposals_context_index
    ON codex_task_proposals (context_export_id, created_at DESC);

CREATE TABLE codex_task_proposal_status_history (
    id TEXT PRIMARY KEY NOT NULL,
    proposal_id TEXT NOT NULL,
    from_status TEXT,
    to_status TEXT NOT NULL CHECK (to_status IN ('draft', 'proposed', 'approved', 'rejected', 'archived')),
    proposal_version INTEGER NOT NULL CHECK (proposal_version >= 1),
    changed_at TEXT NOT NULL,
    CHECK (from_status IS NULL OR from_status IN ('draft', 'proposed', 'approved', 'rejected', 'archived')),
    FOREIGN KEY (proposal_id) REFERENCES codex_task_proposals (id) ON UPDATE CASCADE ON DELETE RESTRICT
);

CREATE INDEX codex_task_proposal_history_proposal_version_index
    ON codex_task_proposal_status_history (proposal_id, proposal_version ASC);
