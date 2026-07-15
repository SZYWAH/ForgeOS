-- D7.1 Unified project development history
--
-- Git remains a read-only external fact source. ForgeOS stores only bounded,
-- non-sensitive commit metadata so development history can be reconstructed
-- without converting every commit into Evidence.

ALTER TABLE project_activity_sync_runs
    ADD COLUMN commits_imported INTEGER NOT NULL DEFAULT 0
        CHECK (commits_imported >= 0);

CREATE TABLE repository_commit_observations (
    id TEXT PRIMARY KEY NOT NULL,
    repository_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    commit_sha TEXT NOT NULL CHECK (
        length(commit_sha) IN (40, 64)
        AND commit_sha NOT GLOB '*[^0-9a-f]*'
    ),
    message TEXT NOT NULL CHECK (length(trim(message)) > 0),
    author_name TEXT NOT NULL DEFAULT '',
    authored_at TEXT NOT NULL,
    observed_at TEXT NOT NULL,
    source_hash TEXT NOT NULL CHECK (
        length(source_hash) = 64
        AND source_hash NOT GLOB '*[^0-9a-f]*'
    ),
    CONSTRAINT repository_commit_repository_project_fk
        FOREIGN KEY (repository_id, project_id) REFERENCES repositories (id, project_id)
        ON UPDATE CASCADE ON DELETE RESTRICT,
    CONSTRAINT repository_commit_unique UNIQUE (repository_id, commit_sha)
);

CREATE INDEX repository_commits_project_authored_index
    ON repository_commit_observations (project_id, authored_at DESC, id DESC);

CREATE INDEX repository_commits_repository_authored_index
    ON repository_commit_observations (repository_id, authored_at DESC, id DESC);

CREATE UNIQUE INDEX repository_commits_identity_project_index
    ON repository_commit_observations (id, project_id);

CREATE TABLE repository_history_cursors (
    repository_id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,
    initial_backfill_completed INTEGER NOT NULL DEFAULT 0
        CHECK (initial_backfill_completed IN (0, 1)),
    last_scanned_head_commit TEXT CHECK (
        last_scanned_head_commit IS NULL OR (
            length(last_scanned_head_commit) IN (40, 64)
            AND last_scanned_head_commit NOT GLOB '*[^0-9a-f]*'
        )
    ),
    last_success_at TEXT,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
    CONSTRAINT repository_history_cursor_repository_project_fk
        FOREIGN KEY (repository_id, project_id) REFERENCES repositories (id, project_id)
        ON UPDATE CASCADE ON DELETE RESTRICT
);

CREATE INDEX repository_history_cursors_project_index
    ON repository_history_cursors (project_id, last_success_at DESC, repository_id);
