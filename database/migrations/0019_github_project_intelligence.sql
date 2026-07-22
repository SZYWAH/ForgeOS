-- D9.1 Public GitHub project intelligence.
--
-- Analysis runs are external, reviewable reports. They never create a
-- ForgeOS Project and never store complete repository files.

CREATE TABLE github_analysis_runs (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL,
    github_repository_id TEXT NOT NULL CHECK (length(trim(github_repository_id)) > 0),
    owner TEXT NOT NULL CHECK (length(trim(owner)) > 0),
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    repository_url TEXT NOT NULL CHECK (repository_url LIKE 'https://github.com/%'),
    default_branch TEXT NOT NULL CHECK (length(trim(default_branch)) > 0),
    source_commit_sha TEXT NOT NULL CHECK (length(source_commit_sha) BETWEEN 7 AND 64),
    focus_type TEXT NOT NULL CHECK (focus_type IN (
        'overview', 'architecture', 'learning', 'reproduction',
        'secondary_development', 'custom'
    )),
    custom_prompt TEXT,
    status TEXT NOT NULL CHECK (status IN (
        'queued', 'collecting', 'analyzing', 'ready', 'partial', 'failed', 'archived'
    )),
    stage TEXT CHECK (stage IS NULL OR stage IN (
        'resolving', 'collecting_tree', 'reading_sources',
        'rules_analysis', 'ai_analysis', 'persisting'
    )),
    analysis_mode TEXT NOT NULL CHECK (analysis_mode IN ('rules', 'ai')),
    readiness TEXT NOT NULL CHECK (readiness IN ('ready', 'warning', 'incomplete', 'blocked')),
    analyzer_revision TEXT NOT NULL CHECK (length(trim(analyzer_revision)) > 0),
    request_hash TEXT NOT NULL CHECK (length(request_hash) = 64),
    report_json TEXT CHECK (report_json IS NULL OR json_valid(report_json)),
    report_markdown TEXT,
    content_hash TEXT CHECK (content_hash IS NULL OR length(content_hash) = 64),
    ai_provider_id TEXT,
    ai_model TEXT,
    imported_project_id TEXT,
    tree_entries_seen INTEGER NOT NULL DEFAULT 0 CHECK (tree_entries_seen >= 0),
    files_read INTEGER NOT NULL DEFAULT 0 CHECK (files_read >= 0),
    files_skipped INTEGER NOT NULL DEFAULT 0 CHECK (files_skipped >= 0),
    source_bytes INTEGER NOT NULL DEFAULT 0 CHECK (source_bytes >= 0),
    api_requests_used INTEGER NOT NULL DEFAULT 0 CHECK (api_requests_used >= 0),
    api_requests_remaining INTEGER CHECK (api_requests_remaining IS NULL OR api_requests_remaining >= 0),
    api_rate_limit_reset_at TEXT,
    ai_invoked INTEGER NOT NULL DEFAULT 0 CHECK (ai_invoked IN (0, 1)),
    error_code TEXT,
    error_summary TEXT,
    started_at TEXT,
    completed_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
    CONSTRAINT github_analysis_workspace_fk
        FOREIGN KEY (workspace_id) REFERENCES workspaces (id)
        ON UPDATE CASCADE ON DELETE RESTRICT,
    CONSTRAINT github_analysis_provider_fk
        FOREIGN KEY (ai_provider_id) REFERENCES ai_providers (id)
        ON UPDATE CASCADE ON DELETE SET NULL,
    CONSTRAINT github_analysis_project_fk
        FOREIGN KEY (imported_project_id) REFERENCES projects (id)
        ON UPDATE CASCADE ON DELETE SET NULL,
    UNIQUE (workspace_id, request_hash)
);

CREATE UNIQUE INDEX github_analysis_one_active_workspace_index
    ON github_analysis_runs (workspace_id)
    WHERE status IN ('queued', 'collecting', 'analyzing');

CREATE INDEX github_analysis_history_index
    ON github_analysis_runs (workspace_id, created_at DESC, id DESC);

CREATE INDEX github_analysis_repository_head_index
    ON github_analysis_runs (
        workspace_id, github_repository_id, source_commit_sha, created_at DESC
    );

CREATE TABLE github_analysis_sources (
    id TEXT PRIMARY KEY NOT NULL,
    analysis_run_id TEXT NOT NULL,
    source_type TEXT NOT NULL CHECK (source_type IN (
        'metadata', 'tree', 'readme', 'documentation', 'manifest', 'license',
        'workflow', 'test_config', 'build_config', 'release', 'commit', 'sbom', 'entrypoint'
    )),
    source_path TEXT NOT NULL DEFAULT '',
    source_sha TEXT,
    content_hash TEXT NOT NULL CHECK (length(content_hash) = 64),
    excerpt TEXT NOT NULL CHECK (length(excerpt) <= 8000),
    source_order INTEGER NOT NULL CHECK (source_order >= 0),
    CONSTRAINT github_analysis_source_run_fk
        FOREIGN KEY (analysis_run_id) REFERENCES github_analysis_runs (id)
        ON UPDATE CASCADE ON DELETE CASCADE,
    CONSTRAINT github_analysis_source_relative_path CHECK (
        source_path = '' OR (
            source_path NOT LIKE '/%'
            AND source_path NOT LIKE '\\%'
            AND instr(source_path, ':') = 0
            AND instr('/' || source_path || '/', '/../') = 0
        )
    ),
    UNIQUE (analysis_run_id, source_type, source_path)
);

CREATE INDEX github_analysis_sources_order_index
    ON github_analysis_sources (analysis_run_id, source_order, id);
