-- D6.2 Project Pulse
--
-- Pulse rows are reviewable observation drafts. Editing a draft never changes
-- its source fingerprint, and formal Snapshot/Session state is only updated by
-- the explicit confirmation transaction.

ALTER TABLE working_copies ADD COLUMN upstream_branch TEXT;
ALTER TABLE working_copies ADD COLUMN ahead_count INTEGER CHECK (ahead_count IS NULL OR ahead_count >= 0);
ALTER TABLE working_copies ADD COLUMN behind_count INTEGER CHECK (behind_count IS NULL OR behind_count >= 0);

CREATE UNIQUE INDEX project_snapshots_identity_project_index
    ON project_snapshots (id, project_id);

CREATE UNIQUE INDEX development_sessions_identity_project_index
    ON development_sessions (id, project_id);

CREATE TABLE project_pulses (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending_review'
        CHECK (status IN ('pending_review', 'confirmed', 'dismissed', 'superseded')),
    analysis_mode TEXT NOT NULL CHECK (analysis_mode IN ('rules', 'ai')),
    ai_provider_id TEXT,
    ai_model TEXT,
    current_goal TEXT NOT NULL CHECK (length(trim(current_goal)) > 0),
    status_summary TEXT NOT NULL CHECK (length(trim(status_summary)) > 0),
    achievements TEXT NOT NULL DEFAULT '[]'
        CHECK (json_valid(achievements) AND json_type(achievements) = 'array'),
    blockers TEXT NOT NULL DEFAULT '[]'
        CHECK (json_valid(blockers) AND json_type(blockers) = 'array'),
    next_steps TEXT NOT NULL DEFAULT '[]'
        CHECK (json_valid(next_steps) AND json_type(next_steps) = 'array'),
    validation_summary TEXT NOT NULL DEFAULT '',
    handoff TEXT NOT NULL DEFAULT '',
    git_summary TEXT NOT NULL DEFAULT '',
    snapshot_basis_version INTEGER CHECK (snapshot_basis_version IS NULL OR snapshot_basis_version >= 1),
    session_basis_id TEXT,
    session_basis_version INTEGER CHECK (session_basis_version IS NULL OR session_basis_version >= 1),
    working_copy_id TEXT,
    content_hash TEXT NOT NULL CHECK (length(content_hash) = 64),
    generated_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
    confirmed_snapshot_id TEXT,
    confirmed_session_id TEXT,
    CONSTRAINT pulses_project_fk
        FOREIGN KEY (project_id) REFERENCES projects (id)
        ON UPDATE CASCADE ON DELETE RESTRICT,
    CONSTRAINT pulses_provider_fk
        FOREIGN KEY (ai_provider_id) REFERENCES ai_providers (id)
        ON UPDATE CASCADE ON DELETE RESTRICT,
    CONSTRAINT pulses_session_basis_project_fk
        FOREIGN KEY (session_basis_id, project_id) REFERENCES development_sessions (id, project_id)
        ON UPDATE CASCADE ON DELETE RESTRICT,
    CONSTRAINT pulses_working_copy_fk
        FOREIGN KEY (working_copy_id) REFERENCES working_copies (id)
        ON UPDATE CASCADE ON DELETE RESTRICT,
    CONSTRAINT pulses_confirmed_snapshot_project_fk
        FOREIGN KEY (confirmed_snapshot_id, project_id) REFERENCES project_snapshots (id, project_id)
        ON UPDATE CASCADE ON DELETE RESTRICT,
    CONSTRAINT pulses_confirmed_session_project_fk
        FOREIGN KEY (confirmed_session_id, project_id) REFERENCES development_sessions (id, project_id)
        ON UPDATE CASCADE ON DELETE RESTRICT,
    CONSTRAINT pulses_analysis_provider_check CHECK (
        analysis_mode = 'rules' OR ai_provider_id IS NOT NULL
    )
);

CREATE UNIQUE INDEX pulses_one_pending_per_project_index
    ON project_pulses (project_id)
    WHERE status = 'pending_review';

CREATE INDEX pulses_project_generated_index
    ON project_pulses (project_id, generated_at DESC, id DESC);

CREATE UNIQUE INDEX pulses_identity_project_index
    ON project_pulses (id, project_id);

CREATE TRIGGER project_pulses_validate_working_copy_insert
BEFORE INSERT ON project_pulses
WHEN NEW.working_copy_id IS NOT NULL
     AND NOT EXISTS (
        SELECT 1
        FROM working_copies wc
        JOIN repositories r ON r.id = wc.repository_id
        WHERE wc.id = NEW.working_copy_id AND r.project_id = NEW.project_id
     )
BEGIN
    SELECT RAISE(ABORT, 'project_pulse_working_copy_project_mismatch');
END;

CREATE TRIGGER project_pulses_validate_working_copy_update
BEFORE UPDATE OF working_copy_id, project_id ON project_pulses
WHEN NEW.working_copy_id IS NOT NULL
     AND NOT EXISTS (
        SELECT 1
        FROM working_copies wc
        JOIN repositories r ON r.id = wc.repository_id
        WHERE wc.id = NEW.working_copy_id AND r.project_id = NEW.project_id
     )
BEGIN
    SELECT RAISE(ABORT, 'project_pulse_working_copy_project_mismatch');
END;

CREATE TABLE project_pulse_sources (
    pulse_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    observation_id TEXT NOT NULL,
    source_order INTEGER NOT NULL CHECK (source_order >= 0),
    PRIMARY KEY (pulse_id, observation_id),
    UNIQUE (pulse_id, source_order),
    CONSTRAINT pulse_sources_pulse_project_fk
        FOREIGN KEY (pulse_id, project_id) REFERENCES project_pulses (id, project_id)
        ON UPDATE CASCADE ON DELETE RESTRICT,
    CONSTRAINT pulse_sources_observation_project_fk
        FOREIGN KEY (observation_id, project_id) REFERENCES codex_activity_observations (id, project_id)
        ON UPDATE CASCADE ON DELETE RESTRICT
);

CREATE INDEX pulse_sources_observation_index
    ON project_pulse_sources (observation_id, pulse_id);
