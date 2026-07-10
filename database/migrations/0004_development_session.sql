-- D1.3 Development Session Kernel
--
-- A Development Session is the smallest durable record of an engineering
-- activity. A partial unique index guarantees that a Project cannot have two
-- open Sessions, including when concurrent writers race.

CREATE TABLE development_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'open'
        CHECK (status IN ('open', 'completed', 'abandoned')),
    goal TEXT NOT NULL
        CHECK (length(trim(goal)) > 0),
    notes TEXT NOT NULL DEFAULT '',
    outcome TEXT NOT NULL DEFAULT '',
    validation TEXT NOT NULL DEFAULT '',
    handoff TEXT NOT NULL DEFAULT '',
    started_at TEXT NOT NULL,
    completed_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1
        CHECK (version >= 1),
    CONSTRAINT development_sessions_project_fk
        FOREIGN KEY (project_id)
        REFERENCES projects (id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    CONSTRAINT development_sessions_lifecycle_check CHECK (
        (
            status = 'open'
            AND completed_at IS NULL
            AND outcome = ''
            AND validation = ''
            AND handoff = ''
        )
        OR (
            status = 'completed'
            AND completed_at IS NOT NULL
            AND length(trim(outcome)) > 0
            AND length(trim(validation)) > 0
            AND length(trim(handoff)) > 0
        )
        OR (
            status = 'abandoned'
            AND completed_at IS NOT NULL
            AND outcome = ''
            AND validation = ''
            AND length(trim(handoff)) > 0
        )
    )
);

CREATE UNIQUE INDEX development_sessions_one_open_per_project_index
    ON development_sessions (project_id)
    WHERE status = 'open';

CREATE INDEX development_sessions_project_started_index
    ON development_sessions (project_id, started_at DESC, created_at DESC);
