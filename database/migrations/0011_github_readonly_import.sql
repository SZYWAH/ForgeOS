-- D4.5 GitHub Connector / Project Import Foundation
--
-- GitHub is an external, read-only fact source. Tokens are referenced here but
-- are stored only in Windows Credential Manager. Import is explicitly staged
-- through a human-reviewed draft before any Project or Evidence is created.

CREATE TABLE github_connectors (
    id TEXT PRIMARY KEY NOT NULL,
    provider TEXT NOT NULL DEFAULT 'github' CHECK (provider = 'github'),
    auth_type TEXT NOT NULL DEFAULT 'none' CHECK (auth_type IN ('none', 'token_reference')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
    UNIQUE (provider)
);

CREATE TABLE github_import_drafts (
    id TEXT PRIMARY KEY NOT NULL,
    connector_id TEXT NOT NULL,
    github_repository_id TEXT NOT NULL CHECK (length(trim(github_repository_id)) > 0),
    owner TEXT NOT NULL CHECK (length(trim(owner)) > 0),
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    url TEXT NOT NULL CHECK (url LIKE 'https://github.com/%'),
    description TEXT NOT NULL DEFAULT '',
    homepage TEXT,
    default_branch TEXT NOT NULL CHECK (length(trim(default_branch)) > 0),
    language TEXT,
    languages TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(languages) AND json_type(languages) = 'object'),
    topics TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(topics) AND json_type(topics) = 'array'),
    readme_summary TEXT NOT NULL DEFAULT '',
    recent_commits TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(recent_commits) AND json_type(recent_commits) = 'array'),
    latest_release TEXT CHECK (latest_release IS NULL OR json_valid(latest_release)),
    status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'reviewed', 'imported', 'rejected')),
    imported_project_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
    FOREIGN KEY (connector_id) REFERENCES github_connectors (id)
        ON UPDATE CASCADE ON DELETE RESTRICT,
    FOREIGN KEY (imported_project_id) REFERENCES projects (id)
        ON UPDATE CASCADE ON DELETE RESTRICT,
    UNIQUE (github_repository_id)
);

CREATE INDEX github_import_drafts_status_updated_index
    ON github_import_drafts (status, updated_at DESC, created_at DESC);
CREATE INDEX github_import_drafts_project_index
    ON github_import_drafts (imported_project_id, updated_at DESC);
CREATE UNIQUE INDEX github_import_drafts_identity_project_index
    ON github_import_drafts (id, imported_project_id);

-- Expand Evidence with GitHub fact types/source. SQLite cannot alter CHECK
-- constraints in place, so preserve both Evidence links and D2 repository links
-- while rebuilding the constrained table.
CREATE TEMP TABLE repository_evidence_links_backup AS
SELECT evidence_id, repository_id, project_id, commit_sha, captured_at
FROM repository_evidence_links;

DROP TABLE repository_evidence_links;
DROP TRIGGER evidence_links_validate_target_insert;
DROP TRIGGER evidence_links_validate_target_update;
ALTER TABLE evidence_links RENAME TO evidence_links_legacy;
ALTER TABLE evidences RENAME TO evidences_legacy;

CREATE TABLE evidences (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    description TEXT NOT NULL DEFAULT '',
    type TEXT NOT NULL CHECK (type IN (
        'document', 'screenshot', 'link', 'test_result', 'note', 'artifact',
        'git_commit', 'git_status', 'github_repository', 'github_commit', 'github_release'
    )),
    source_type TEXT NOT NULL CHECK (source_type IN (
        'local_file', 'external_url', 'manual_note', 'github_api'
    )),
    source_uri TEXT NOT NULL DEFAULT '',
    verification_status TEXT NOT NULL DEFAULT 'unverified'
        CHECK (verification_status IN ('unverified', 'verified', 'invalid')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
    FOREIGN KEY (project_id) REFERENCES projects (id)
        ON UPDATE CASCADE ON DELETE RESTRICT,
    CHECK (source_type = 'manual_note' OR length(trim(source_uri)) > 0)
);

INSERT INTO evidences (
    id, project_id, title, description, type, source_type, source_uri,
    verification_status, created_at, updated_at, version
)
SELECT id, project_id, title, description, type, source_type, source_uri,
       verification_status, created_at, updated_at, version
FROM evidences_legacy;

CREATE TABLE evidence_links (
    id TEXT PRIMARY KEY NOT NULL,
    evidence_id TEXT NOT NULL,
    target_type TEXT NOT NULL CHECK (target_type IN ('project', 'snapshot', 'session')),
    target_id TEXT NOT NULL CHECK (length(trim(target_id)) > 0),
    relation_type TEXT NOT NULL CHECK (relation_type IN ('supports', 'relates_to')),
    FOREIGN KEY (evidence_id) REFERENCES evidences (id)
        ON UPDATE CASCADE ON DELETE RESTRICT,
    UNIQUE (evidence_id, target_type, target_id, relation_type)
);

INSERT INTO evidence_links (id, evidence_id, target_type, target_id, relation_type)
SELECT id, evidence_id, target_type, target_id, relation_type
FROM evidence_links_legacy;

DROP TABLE evidence_links_legacy;
DROP TABLE evidences_legacy;

CREATE INDEX evidences_project_updated_index
    ON evidences (project_id, updated_at DESC, created_at DESC);
CREATE INDEX evidences_project_verification_index
    ON evidences (project_id, verification_status);
CREATE UNIQUE INDEX evidences_identity_project_index ON evidences (id, project_id);
CREATE INDEX evidence_links_evidence_index
    ON evidence_links (evidence_id, target_type, target_id);
CREATE INDEX evidence_links_target_index ON evidence_links (target_type, target_id);

CREATE TRIGGER evidence_links_validate_target_insert
BEFORE INSERT ON evidence_links
BEGIN
    SELECT CASE
        WHEN NEW.target_type = 'project' AND NOT EXISTS (
            SELECT 1 FROM evidences e JOIN projects p ON p.id = NEW.target_id
            WHERE e.id = NEW.evidence_id AND p.id = e.project_id
        ) THEN RAISE(ABORT, 'evidence_link_target_invalid')
        WHEN NEW.target_type = 'snapshot' AND NOT EXISTS (
            SELECT 1 FROM evidences e JOIN project_snapshots s ON s.id = NEW.target_id
            WHERE e.id = NEW.evidence_id AND s.project_id = e.project_id
        ) THEN RAISE(ABORT, 'evidence_link_target_invalid')
        WHEN NEW.target_type = 'session' AND NOT EXISTS (
            SELECT 1 FROM evidences e JOIN development_sessions s ON s.id = NEW.target_id
            WHERE e.id = NEW.evidence_id AND s.project_id = e.project_id
        ) THEN RAISE(ABORT, 'evidence_link_target_invalid')
    END;
END;

CREATE TRIGGER evidence_links_validate_target_update
BEFORE UPDATE OF evidence_id, target_type, target_id ON evidence_links
BEGIN
    SELECT CASE
        WHEN NEW.target_type = 'project' AND NOT EXISTS (
            SELECT 1 FROM evidences e JOIN projects p ON p.id = NEW.target_id
            WHERE e.id = NEW.evidence_id AND p.id = e.project_id
        ) THEN RAISE(ABORT, 'evidence_link_target_invalid')
        WHEN NEW.target_type = 'snapshot' AND NOT EXISTS (
            SELECT 1 FROM evidences e JOIN project_snapshots s ON s.id = NEW.target_id
            WHERE e.id = NEW.evidence_id AND s.project_id = e.project_id
        ) THEN RAISE(ABORT, 'evidence_link_target_invalid')
        WHEN NEW.target_type = 'session' AND NOT EXISTS (
            SELECT 1 FROM evidences e JOIN development_sessions s ON s.id = NEW.target_id
            WHERE e.id = NEW.evidence_id AND s.project_id = e.project_id
        ) THEN RAISE(ABORT, 'evidence_link_target_invalid')
    END;
END;

CREATE TABLE repository_evidence_links (
    evidence_id TEXT PRIMARY KEY NOT NULL,
    repository_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    commit_sha TEXT CHECK (
        commit_sha IS NULL OR (
            length(commit_sha) IN (40, 64) AND commit_sha NOT GLOB '*[^0-9a-f]*'
        )
    ),
    captured_at TEXT NOT NULL,
    FOREIGN KEY (evidence_id, project_id) REFERENCES evidences (id, project_id)
        ON UPDATE CASCADE ON DELETE RESTRICT,
    FOREIGN KEY (repository_id, project_id) REFERENCES repositories (id, project_id)
        ON UPDATE CASCADE ON DELETE RESTRICT
);

INSERT INTO repository_evidence_links (evidence_id, repository_id, project_id, commit_sha, captured_at)
SELECT evidence_id, repository_id, project_id, commit_sha, captured_at
FROM repository_evidence_links_backup;
DROP TABLE repository_evidence_links_backup;
CREATE INDEX repository_evidence_repository_captured_index
    ON repository_evidence_links (repository_id, captured_at DESC, evidence_id);

CREATE TABLE github_evidence_links (
    evidence_id TEXT PRIMARY KEY NOT NULL,
    draft_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    github_object_type TEXT NOT NULL CHECK (github_object_type IN ('repository', 'commit', 'release')),
    github_object_id TEXT NOT NULL CHECK (length(trim(github_object_id)) > 0),
    captured_at TEXT NOT NULL,
    FOREIGN KEY (evidence_id, project_id) REFERENCES evidences (id, project_id)
        ON UPDATE CASCADE ON DELETE RESTRICT,
    FOREIGN KEY (draft_id, project_id) REFERENCES github_import_drafts (id, imported_project_id)
        ON UPDATE CASCADE ON DELETE RESTRICT
);

CREATE INDEX github_evidence_draft_captured_index
    ON github_evidence_links (draft_id, captured_at DESC, evidence_id);
