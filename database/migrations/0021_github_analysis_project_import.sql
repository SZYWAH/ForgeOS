-- D9.3 GitHub analysis report human import.
--
-- An analysis import proposal extends the existing GitHub Import Draft without
-- changing the legacy import lifecycle. The proposal stores only a reviewed
-- Project identity draft and an immutable reference to the selected analysis.

CREATE TABLE github_analysis_import_proposals (
    draft_id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL,
    analysis_run_id TEXT NOT NULL,
    source_commit_sha TEXT NOT NULL
        CHECK (length(source_commit_sha) BETWEEN 7 AND 64),
    source_content_hash TEXT NOT NULL
        CHECK (length(source_content_hash) = 64),
    project_name TEXT NOT NULL
        CHECK (length(trim(project_name)) BETWEEN 1 AND 200),
    project_slug TEXT COLLATE NOCASE NOT NULL
        CHECK (length(trim(project_slug)) BETWEEN 2 AND 64),
    description TEXT NOT NULL DEFAULT ''
        CHECK (length(description) <= 2000),
    vision TEXT NOT NULL DEFAULT '',
    target_user TEXT NOT NULL DEFAULT '',
    scope TEXT NOT NULL DEFAULT '',
    non_goals TEXT NOT NULL DEFAULT '',
    tech_stack TEXT NOT NULL DEFAULT '',
    project_status TEXT NOT NULL DEFAULT 'active'
        CHECK (project_status IN (
            'idea', 'planned', 'active', 'paused', 'maintenance', 'archived'
        )),
    lifecycle_stage TEXT NOT NULL DEFAULT 'definition'
        CHECK (lifecycle_stage IN (
            'discovery', 'definition', 'design', 'implementation',
            'validation', 'release', 'operation', 'retrospective'
        )),
    priority INTEGER NOT NULL DEFAULT 50
        CHECK (priority BETWEEN 0 AND 100),
    partial_acknowledged INTEGER NOT NULL DEFAULT 0
        CHECK (partial_acknowledged IN (0, 1)),
    stale_acknowledged INTEGER NOT NULL DEFAULT 0
        CHECK (stale_acknowledged IN (0, 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
    CONSTRAINT github_analysis_import_draft_fk
        FOREIGN KEY (draft_id) REFERENCES github_import_drafts (id)
        ON UPDATE CASCADE ON DELETE RESTRICT,
    CONSTRAINT github_analysis_import_workspace_fk
        FOREIGN KEY (workspace_id) REFERENCES workspaces (id)
        ON UPDATE CASCADE ON DELETE RESTRICT,
    CONSTRAINT github_analysis_import_run_fk
        FOREIGN KEY (analysis_run_id) REFERENCES github_analysis_runs (id)
        ON UPDATE CASCADE ON DELETE RESTRICT,
    UNIQUE (analysis_run_id),
    UNIQUE (workspace_id, project_slug)
);

CREATE INDEX github_analysis_import_workspace_updated_index
    ON github_analysis_import_proposals (workspace_id, updated_at DESC);

CREATE TRIGGER github_analysis_import_validate_insert
BEFORE INSERT ON github_analysis_import_proposals
BEGIN
    SELECT CASE WHEN NOT EXISTS (
        SELECT 1
        FROM github_analysis_runs run
        JOIN github_import_drafts draft
          ON draft.id = NEW.draft_id
         AND draft.github_repository_id = run.github_repository_id
        WHERE run.id = NEW.analysis_run_id
          AND run.workspace_id = NEW.workspace_id
          AND run.source_commit_sha = NEW.source_commit_sha
          AND run.content_hash = NEW.source_content_hash
          AND run.status IN ('ready', 'partial')
          AND run.readiness <> 'blocked'
    ) THEN RAISE(ABORT, 'github_analysis_import_source_invalid') END;
END;

CREATE TRIGGER github_analysis_import_validate_update
BEFORE UPDATE OF
    draft_id, workspace_id, analysis_run_id, source_commit_sha, source_content_hash
ON github_analysis_import_proposals
BEGIN
    SELECT CASE WHEN NOT EXISTS (
        SELECT 1
        FROM github_analysis_runs run
        JOIN github_import_drafts draft
          ON draft.id = NEW.draft_id
         AND draft.github_repository_id = run.github_repository_id
        WHERE run.id = NEW.analysis_run_id
          AND run.workspace_id = NEW.workspace_id
          AND run.source_commit_sha = NEW.source_commit_sha
          AND run.content_hash = NEW.source_content_hash
          AND run.status IN ('ready', 'partial')
          AND run.readiness <> 'blocked'
    ) THEN RAISE(ABORT, 'github_analysis_import_source_invalid') END;
END;

CREATE TRIGGER github_analysis_imported_project_validate_insert
BEFORE INSERT ON github_analysis_runs
WHEN NEW.imported_project_id IS NOT NULL
BEGIN
    SELECT CASE WHEN NOT EXISTS (
        SELECT 1 FROM projects project
        WHERE project.id = NEW.imported_project_id
          AND project.workspace_id = NEW.workspace_id
    ) THEN RAISE(ABORT, 'github_analysis_import_project_invalid') END;
END;

CREATE TRIGGER github_analysis_imported_project_validate_update
BEFORE UPDATE OF imported_project_id, workspace_id ON github_analysis_runs
WHEN NEW.imported_project_id IS NOT NULL
BEGIN
    SELECT CASE WHEN NOT EXISTS (
        SELECT 1 FROM projects project
        WHERE project.id = NEW.imported_project_id
          AND project.workspace_id = NEW.workspace_id
    ) THEN RAISE(ABORT, 'github_analysis_import_project_invalid') END;
END;
