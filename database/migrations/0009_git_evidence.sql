-- D2 Git Evidence Integration
--
-- ForgeOS observes Git through read-only commands. Repository metadata and
-- immutable WorkingCopy observations can be converted into Evidence, but no
-- Git mutation capability is represented in this schema.

CREATE TABLE repositories (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,
    path TEXT NOT NULL CHECK (length(trim(path)) > 0),
    remote_url TEXT,
    default_branch TEXT NOT NULL CHECK (length(trim(default_branch)) > 0),
    last_checked_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
    CONSTRAINT repositories_project_fk
        FOREIGN KEY (project_id) REFERENCES projects (id)
        ON UPDATE CASCADE ON DELETE RESTRICT
);

CREATE UNIQUE INDEX repositories_path_unique_index
    ON repositories (path COLLATE NOCASE);

CREATE INDEX repositories_project_checked_index
    ON repositories (project_id, last_checked_at DESC, id);

CREATE UNIQUE INDEX repositories_identity_project_index
    ON repositories (id, project_id);

CREATE TABLE working_copies (
    id TEXT PRIMARY KEY NOT NULL,
    repository_id TEXT NOT NULL,
    branch TEXT NOT NULL CHECK (length(trim(branch)) > 0),
    head_commit TEXT NOT NULL CHECK (
        length(head_commit) IN (40, 64)
        AND head_commit NOT GLOB '*[^0-9a-f]*'
    ),
    is_dirty INTEGER NOT NULL CHECK (is_dirty IN (0, 1)),
    changed_files INTEGER NOT NULL DEFAULT 0 CHECK (changed_files >= 0),
    staged_files INTEGER NOT NULL DEFAULT 0 CHECK (staged_files >= 0),
    unstaged_files INTEGER NOT NULL DEFAULT 0 CHECK (unstaged_files >= 0),
    untracked_files INTEGER NOT NULL DEFAULT 0 CHECK (untracked_files >= 0),
    insertions INTEGER NOT NULL DEFAULT 0 CHECK (insertions >= 0),
    deletions INTEGER NOT NULL DEFAULT 0 CHECK (deletions >= 0),
    checked_at TEXT NOT NULL,
    CONSTRAINT working_copies_repository_fk
        FOREIGN KEY (repository_id) REFERENCES repositories (id)
        ON UPDATE CASCADE ON DELETE RESTRICT
);

CREATE INDEX working_copies_repository_checked_index
    ON working_copies (repository_id, checked_at DESC, id DESC);

-- SQLite cannot alter the Evidence type CHECK in place. Rebuild the two
-- Evidence tables inside this new forward migration while preserving data and
-- recreating the D1.4 indexes/triggers.

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
        'git_commit', 'git_status'
    )),
    source_type TEXT NOT NULL CHECK (source_type IN ('local_file', 'external_url', 'manual_note')),
    source_uri TEXT NOT NULL DEFAULT '',
    verification_status TEXT NOT NULL DEFAULT 'unverified'
        CHECK (verification_status IN ('unverified', 'verified', 'invalid')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
    CONSTRAINT evidences_project_fk
        FOREIGN KEY (project_id) REFERENCES projects (id)
        ON UPDATE CASCADE ON DELETE RESTRICT,
    CONSTRAINT evidences_source_check CHECK (
        source_type = 'manual_note' OR length(trim(source_uri)) > 0
    )
);

INSERT INTO evidences (
    id, project_id, title, description, type, source_type, source_uri,
    verification_status, created_at, updated_at, version
)
SELECT
    id, project_id, title, description, type, source_type, source_uri,
    verification_status, created_at, updated_at, version
FROM evidences_legacy;

CREATE TABLE evidence_links (
    id TEXT PRIMARY KEY NOT NULL,
    evidence_id TEXT NOT NULL,
    target_type TEXT NOT NULL CHECK (target_type IN ('project', 'snapshot', 'session')),
    target_id TEXT NOT NULL CHECK (length(trim(target_id)) > 0),
    relation_type TEXT NOT NULL CHECK (relation_type IN ('supports', 'relates_to')),
    CONSTRAINT evidence_links_evidence_fk
        FOREIGN KEY (evidence_id) REFERENCES evidences (id)
        ON UPDATE CASCADE ON DELETE RESTRICT,
    CONSTRAINT evidence_links_unique
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
CREATE UNIQUE INDEX evidences_identity_project_index
    ON evidences (id, project_id);

CREATE INDEX evidence_links_evidence_index
    ON evidence_links (evidence_id, target_type, target_id);
CREATE INDEX evidence_links_target_index
    ON evidence_links (target_type, target_id);

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
            length(commit_sha) IN (40, 64)
            AND commit_sha NOT GLOB '*[^0-9a-f]*'
        )
    ),
    captured_at TEXT NOT NULL,
    CONSTRAINT repository_evidence_evidence_project_fk
        FOREIGN KEY (evidence_id, project_id) REFERENCES evidences (id, project_id)
        ON UPDATE CASCADE ON DELETE RESTRICT,
    CONSTRAINT repository_evidence_repository_project_fk
        FOREIGN KEY (repository_id, project_id) REFERENCES repositories (id, project_id)
        ON UPDATE CASCADE ON DELETE RESTRICT
);

CREATE INDEX repository_evidence_repository_captured_index
    ON repository_evidence_links (repository_id, captured_at DESC, evidence_id);
