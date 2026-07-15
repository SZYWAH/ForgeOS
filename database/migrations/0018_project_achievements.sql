-- D7.2 Structured project achievement memory
--
-- Achievements are reviewable career and retrospective assets. Automatically
-- derived records remain drafts until the user explicitly confirms them.

CREATE TABLE project_achievements (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,
    category TEXT NOT NULL CHECK (category IN (
        'feature', 'problem_solution', 'release', 'technical_highlight'
    )),
    title TEXT NOT NULL DEFAULT '',
    problem TEXT NOT NULL DEFAULT '',
    approach TEXT NOT NULL DEFAULT '',
    result TEXT NOT NULL DEFAULT '',
    validation TEXT NOT NULL DEFAULT '',
    impact TEXT NOT NULL DEFAULT '',
    technologies TEXT NOT NULL DEFAULT '[]'
        CHECK (json_valid(technologies) AND json_type(technologies) = 'array'),
    resume_bullet TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'draft'
        CHECK (status IN ('draft', 'confirmed', 'archived')),
    source_mode TEXT NOT NULL CHECK (source_mode IN ('manual', 'pulse_rule')),
    featured INTEGER NOT NULL DEFAULT 0 CHECK (featured IN (0, 1)),
    source_pulse_id TEXT,
    confirmed_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
    CONSTRAINT project_achievements_project_fk
        FOREIGN KEY (project_id) REFERENCES projects (id)
        ON UPDATE CASCADE ON DELETE RESTRICT,
    CONSTRAINT project_achievements_pulse_project_fk
        FOREIGN KEY (source_pulse_id, project_id) REFERENCES project_pulses (id, project_id)
        ON UPDATE CASCADE ON DELETE RESTRICT,
    CONSTRAINT project_achievements_confirmed_shape CHECK (
        status <> 'confirmed' OR (
            length(trim(title)) > 0
            AND length(trim(result)) > 0
            AND (
                length(trim(approach)) > 0
                OR length(trim(validation)) > 0
                OR length(trim(impact)) > 0
            )
            AND confirmed_at IS NOT NULL
        )
    )
);

CREATE UNIQUE INDEX project_achievements_identity_project_index
    ON project_achievements (id, project_id);

CREATE UNIQUE INDEX project_achievements_one_draft_per_pulse_index
    ON project_achievements (source_pulse_id)
    WHERE source_pulse_id IS NOT NULL;

CREATE INDEX project_achievements_project_status_index
    ON project_achievements (project_id, status, featured DESC, updated_at DESC, id DESC);

CREATE TABLE project_achievement_sources (
    achievement_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    source_type TEXT NOT NULL CHECK (source_type IN (
        'pulse', 'snapshot', 'session', 'evidence', 'codex_observation', 'repository_commit'
    )),
    source_id TEXT NOT NULL CHECK (length(trim(source_id)) > 0),
    source_order INTEGER NOT NULL DEFAULT 0 CHECK (source_order >= 0),
    PRIMARY KEY (achievement_id, source_type, source_id),
    CONSTRAINT achievement_sources_achievement_project_fk
        FOREIGN KEY (achievement_id, project_id) REFERENCES project_achievements (id, project_id)
        ON UPDATE CASCADE ON DELETE RESTRICT
);

CREATE INDEX achievement_sources_project_index
    ON project_achievement_sources (project_id, source_type, source_id);

CREATE TRIGGER project_achievement_sources_validate_insert
BEFORE INSERT ON project_achievement_sources
BEGIN
    SELECT CASE
        WHEN NEW.source_type = 'pulse' AND NOT EXISTS (
            SELECT 1 FROM project_pulses s
            WHERE s.id = NEW.source_id AND s.project_id = NEW.project_id
        ) THEN RAISE(ABORT, 'achievement_source_project_mismatch')
        WHEN NEW.source_type = 'snapshot' AND NOT EXISTS (
            SELECT 1 FROM project_snapshots s
            WHERE s.id = NEW.source_id AND s.project_id = NEW.project_id
        ) THEN RAISE(ABORT, 'achievement_source_project_mismatch')
        WHEN NEW.source_type = 'session' AND NOT EXISTS (
            SELECT 1 FROM development_sessions s
            WHERE s.id = NEW.source_id AND s.project_id = NEW.project_id
        ) THEN RAISE(ABORT, 'achievement_source_project_mismatch')
        WHEN NEW.source_type = 'evidence' AND NOT EXISTS (
            SELECT 1 FROM evidences s
            WHERE s.id = NEW.source_id AND s.project_id = NEW.project_id
        ) THEN RAISE(ABORT, 'achievement_source_project_mismatch')
        WHEN NEW.source_type = 'codex_observation' AND NOT EXISTS (
            SELECT 1 FROM codex_activity_observations s
            WHERE s.id = NEW.source_id AND s.project_id = NEW.project_id
        ) THEN RAISE(ABORT, 'achievement_source_project_mismatch')
        WHEN NEW.source_type = 'repository_commit' AND NOT EXISTS (
            SELECT 1 FROM repository_commit_observations s
            WHERE s.id = NEW.source_id AND s.project_id = NEW.project_id
        ) THEN RAISE(ABORT, 'achievement_source_project_mismatch')
    END;
END;

CREATE TRIGGER project_achievement_sources_validate_update
BEFORE UPDATE OF achievement_id, project_id, source_type, source_id
    ON project_achievement_sources
BEGIN
    SELECT CASE
        WHEN NEW.source_type = 'pulse' AND NOT EXISTS (
            SELECT 1 FROM project_pulses s WHERE s.id = NEW.source_id AND s.project_id = NEW.project_id
        ) THEN RAISE(ABORT, 'achievement_source_project_mismatch')
        WHEN NEW.source_type = 'snapshot' AND NOT EXISTS (
            SELECT 1 FROM project_snapshots s WHERE s.id = NEW.source_id AND s.project_id = NEW.project_id
        ) THEN RAISE(ABORT, 'achievement_source_project_mismatch')
        WHEN NEW.source_type = 'session' AND NOT EXISTS (
            SELECT 1 FROM development_sessions s WHERE s.id = NEW.source_id AND s.project_id = NEW.project_id
        ) THEN RAISE(ABORT, 'achievement_source_project_mismatch')
        WHEN NEW.source_type = 'evidence' AND NOT EXISTS (
            SELECT 1 FROM evidences s WHERE s.id = NEW.source_id AND s.project_id = NEW.project_id
        ) THEN RAISE(ABORT, 'achievement_source_project_mismatch')
        WHEN NEW.source_type = 'codex_observation' AND NOT EXISTS (
            SELECT 1 FROM codex_activity_observations s WHERE s.id = NEW.source_id AND s.project_id = NEW.project_id
        ) THEN RAISE(ABORT, 'achievement_source_project_mismatch')
        WHEN NEW.source_type = 'repository_commit' AND NOT EXISTS (
            SELECT 1 FROM repository_commit_observations s WHERE s.id = NEW.source_id AND s.project_id = NEW.project_id
        ) THEN RAISE(ABORT, 'achievement_source_project_mismatch')
    END;
END;
