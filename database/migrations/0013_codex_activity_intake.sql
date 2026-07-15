-- D6.1 Codex Activity Intake
--
-- Codex activity is imported through the read-only App Server protocol. Only
-- normalized observations are retained; raw conversations and reasoning are
-- deliberately outside the ForgeOS persistence boundary.

CREATE TABLE codex_connector_settings (
    workspace_id TEXT PRIMARY KEY NOT NULL,
    executable_path TEXT,
    last_detected_version TEXT,
    last_health_check_at TEXT,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CONSTRAINT codex_connector_workspace_fk
        FOREIGN KEY (workspace_id) REFERENCES workspaces (id)
        ON UPDATE CASCADE ON DELETE RESTRICT
);

CREATE TABLE project_activity_settings (
    project_id TEXT PRIMARY KEY NOT NULL,
    sync_mode TEXT NOT NULL DEFAULT 'open_manual'
        CHECK (sync_mode IN ('open_manual', 'interval_30m')),
    ai_enabled INTEGER NOT NULL DEFAULT 0 CHECK (ai_enabled IN (0, 1)),
    ai_provider_id TEXT,
    initial_backfill_completed INTEGER NOT NULL DEFAULT 0
        CHECK (initial_backfill_completed IN (0, 1)),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CONSTRAINT activity_settings_project_fk
        FOREIGN KEY (project_id) REFERENCES projects (id)
        ON UPDATE CASCADE ON DELETE RESTRICT,
    CONSTRAINT activity_settings_provider_fk
        FOREIGN KEY (ai_provider_id) REFERENCES ai_providers (id)
        ON UPDATE CASCADE ON DELETE RESTRICT,
    CONSTRAINT activity_settings_ai_provider_check CHECK (
        ai_enabled = 0 OR ai_provider_id IS NOT NULL
    )
);

CREATE TABLE project_activity_sync_runs (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,
    trigger TEXT NOT NULL CHECK (trigger IN ('project_open', 'manual', 'interval')),
    status TEXT NOT NULL CHECK (status IN ('queued', 'running', 'succeeded', 'partial', 'failed')),
    stage TEXT NOT NULL CHECK (stage IN ('connecting', 'reading_codex', 'reading_git', 'analyzing', 'persisting')),
    threads_scanned INTEGER NOT NULL DEFAULT 0 CHECK (threads_scanned >= 0),
    turns_imported INTEGER NOT NULL DEFAULT 0 CHECK (turns_imported >= 0),
    observations_created INTEGER NOT NULL DEFAULT 0 CHECK (observations_created >= 0),
    ai_invoked INTEGER NOT NULL DEFAULT 0 CHECK (ai_invoked IN (0, 1)),
    started_at TEXT NOT NULL,
    completed_at TEXT,
    error_code TEXT,
    error_summary TEXT,
    CONSTRAINT activity_runs_project_fk
        FOREIGN KEY (project_id) REFERENCES projects (id)
        ON UPDATE CASCADE ON DELETE RESTRICT
);

CREATE INDEX activity_runs_project_started_index
    ON project_activity_sync_runs (project_id, started_at DESC, id DESC);

CREATE UNIQUE INDEX activity_runs_one_active_project_index
    ON project_activity_sync_runs (project_id)
    WHERE status IN ('queued', 'running');

CREATE TABLE codex_sync_cursors (
    repository_id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,
    last_thread_updated_at TEXT,
    last_success_at TEXT,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
    CONSTRAINT codex_cursor_repository_project_fk
        FOREIGN KEY (repository_id, project_id) REFERENCES repositories (id, project_id)
        ON UPDATE CASCADE ON DELETE RESTRICT
);

CREATE INDEX codex_sync_cursors_project_index
    ON codex_sync_cursors (project_id, last_success_at DESC);

CREATE TABLE codex_activity_observations (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,
    repository_id TEXT NOT NULL,
    thread_id TEXT NOT NULL CHECK (length(trim(thread_id)) > 0),
    turn_id TEXT NOT NULL CHECK (length(trim(turn_id)) > 0),
    source_kind TEXT NOT NULL CHECK (length(trim(source_kind)) > 0),
    turn_status TEXT NOT NULL CHECK (turn_status IN ('in_progress', 'completed', 'interrupted', 'failed', 'unknown')),
    goal_hint TEXT NOT NULL DEFAULT '',
    activity_summary TEXT NOT NULL DEFAULT '',
    outcome_summary TEXT NOT NULL DEFAULT '',
    validation_summary TEXT NOT NULL DEFAULT '',
    handoff_summary TEXT NOT NULL DEFAULT '',
    blockers TEXT NOT NULL DEFAULT '[]'
        CHECK (json_valid(blockers) AND json_type(blockers) = 'array'),
    source_updated_at TEXT NOT NULL,
    source_hash TEXT NOT NULL CHECK (length(source_hash) = 64),
    observed_at TEXT NOT NULL,
    CONSTRAINT observations_repository_project_fk
        FOREIGN KEY (repository_id, project_id) REFERENCES repositories (id, project_id)
        ON UPDATE CASCADE ON DELETE RESTRICT,
    CONSTRAINT observations_source_unique
        UNIQUE (repository_id, thread_id, turn_id)
);

CREATE INDEX observations_project_source_index
    ON codex_activity_observations (project_id, source_updated_at DESC, id DESC);

CREATE INDEX observations_repository_source_index
    ON codex_activity_observations (repository_id, source_updated_at DESC, id DESC);

CREATE UNIQUE INDEX observations_identity_project_index
    ON codex_activity_observations (id, project_id);

-- A crash may leave an active run behind. Application startup marks these
-- records failed/interrupted before new work is accepted.
