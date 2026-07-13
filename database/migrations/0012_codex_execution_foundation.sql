-- D5 Codex Execution Foundation
--
-- An Execution is an auditable attempt linked to an approved Codex Task
-- Proposal. D5 records requests, lifecycle changes, and externally supplied
-- results only; it does not execute Codex or validation commands.

CREATE UNIQUE INDEX codex_task_proposals_identity_project_index
    ON codex_task_proposals (id, project_id);

CREATE TABLE codex_execution_records (
    id TEXT PRIMARY KEY NOT NULL,
    proposal_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'running', 'success', 'failed', 'cancelled')),
    start_time TEXT,
    end_time TEXT,
    output_summary TEXT NOT NULL DEFAULT '',
    validation_result TEXT NOT NULL DEFAULT '',
    version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (proposal_id, project_id)
        REFERENCES codex_task_proposals (id, project_id)
        ON UPDATE CASCADE ON DELETE RESTRICT,
    FOREIGN KEY (project_id)
        REFERENCES projects (id)
        ON UPDATE CASCADE ON DELETE RESTRICT,
    CHECK (
        (status = 'pending' AND start_time IS NULL AND end_time IS NULL)
        OR (status = 'running' AND start_time IS NOT NULL AND end_time IS NULL)
        OR (status IN ('success', 'failed') AND start_time IS NOT NULL AND end_time IS NOT NULL)
        OR (status = 'cancelled' AND end_time IS NOT NULL)
    ),
    CHECK (end_time IS NULL OR start_time IS NULL OR end_time >= start_time),
    CHECK (updated_at >= created_at)
);

CREATE INDEX codex_execution_records_project_created_index
    ON codex_execution_records (project_id, created_at DESC, id DESC);
CREATE INDEX codex_execution_records_proposal_created_index
    ON codex_execution_records (proposal_id, created_at DESC, id DESC);
CREATE INDEX codex_execution_records_project_status_index
    ON codex_execution_records (project_id, status, updated_at DESC);

CREATE TABLE codex_execution_status_history (
    id TEXT PRIMARY KEY NOT NULL,
    execution_id TEXT NOT NULL,
    from_status TEXT,
    to_status TEXT NOT NULL
        CHECK (to_status IN ('pending', 'running', 'success', 'failed', 'cancelled')),
    execution_version INTEGER NOT NULL CHECK (execution_version >= 1),
    changed_at TEXT NOT NULL,
    CHECK (
        from_status IS NULL
        OR from_status IN ('pending', 'running', 'success', 'failed', 'cancelled')
    ),
    FOREIGN KEY (execution_id)
        REFERENCES codex_execution_records (id)
        ON UPDATE CASCADE ON DELETE RESTRICT
);

CREATE INDEX codex_execution_history_execution_version_index
    ON codex_execution_status_history (execution_id, execution_version ASC);
