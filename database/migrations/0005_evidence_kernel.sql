-- D1.4 Evidence Kernel
--
-- Evidence stores manually curated support for engineering facts. Links use a
-- polymorphic target, so SQLite triggers enforce target existence and Project
-- ownership where a conventional foreign key cannot express the relationship.

CREATE TABLE evidences (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,
    title TEXT NOT NULL
        CHECK (length(trim(title)) > 0),
    description TEXT NOT NULL DEFAULT '',
    type TEXT NOT NULL
        CHECK (type IN ('document', 'screenshot', 'link', 'test_result', 'note', 'artifact')),
    source_type TEXT NOT NULL
        CHECK (source_type IN ('local_file', 'external_url', 'manual_note')),
    source_uri TEXT NOT NULL DEFAULT '',
    verification_status TEXT NOT NULL DEFAULT 'unverified'
        CHECK (verification_status IN ('unverified', 'verified', 'invalid')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1
        CHECK (version >= 1),
    CONSTRAINT evidences_project_fk
        FOREIGN KEY (project_id)
        REFERENCES projects (id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    CONSTRAINT evidences_source_check CHECK (
        source_type = 'manual_note' OR length(trim(source_uri)) > 0
    )
);

CREATE INDEX evidences_project_updated_index
    ON evidences (project_id, updated_at DESC, created_at DESC);

CREATE INDEX evidences_project_verification_index
    ON evidences (project_id, verification_status);

CREATE TABLE evidence_links (
    id TEXT PRIMARY KEY NOT NULL,
    evidence_id TEXT NOT NULL,
    target_type TEXT NOT NULL
        CHECK (target_type IN ('project', 'snapshot', 'session')),
    target_id TEXT NOT NULL
        CHECK (length(trim(target_id)) > 0),
    relation_type TEXT NOT NULL
        CHECK (relation_type IN ('supports', 'relates_to')),
    CONSTRAINT evidence_links_evidence_fk
        FOREIGN KEY (evidence_id)
        REFERENCES evidences (id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    CONSTRAINT evidence_links_unique
        UNIQUE (evidence_id, target_type, target_id, relation_type)
);

CREATE INDEX evidence_links_evidence_index
    ON evidence_links (evidence_id, target_type, target_id);

CREATE INDEX evidence_links_target_index
    ON evidence_links (target_type, target_id);

CREATE TRIGGER evidence_links_validate_target_insert
BEFORE INSERT ON evidence_links
BEGIN
    SELECT CASE
        WHEN NEW.target_type = 'project' AND NOT EXISTS (
            SELECT 1
            FROM evidences e
            JOIN projects p ON p.id = NEW.target_id
            WHERE e.id = NEW.evidence_id AND p.id = e.project_id
        ) THEN RAISE(ABORT, 'evidence_link_target_invalid')
        WHEN NEW.target_type = 'snapshot' AND NOT EXISTS (
            SELECT 1
            FROM evidences e
            JOIN project_snapshots s ON s.id = NEW.target_id
            WHERE e.id = NEW.evidence_id AND s.project_id = e.project_id
        ) THEN RAISE(ABORT, 'evidence_link_target_invalid')
        WHEN NEW.target_type = 'session' AND NOT EXISTS (
            SELECT 1
            FROM evidences e
            JOIN development_sessions s ON s.id = NEW.target_id
            WHERE e.id = NEW.evidence_id AND s.project_id = e.project_id
        ) THEN RAISE(ABORT, 'evidence_link_target_invalid')
    END;
END;

CREATE TRIGGER evidence_links_validate_target_update
BEFORE UPDATE OF evidence_id, target_type, target_id ON evidence_links
BEGIN
    SELECT CASE
        WHEN NEW.target_type = 'project' AND NOT EXISTS (
            SELECT 1
            FROM evidences e
            JOIN projects p ON p.id = NEW.target_id
            WHERE e.id = NEW.evidence_id AND p.id = e.project_id
        ) THEN RAISE(ABORT, 'evidence_link_target_invalid')
        WHEN NEW.target_type = 'snapshot' AND NOT EXISTS (
            SELECT 1
            FROM evidences e
            JOIN project_snapshots s ON s.id = NEW.target_id
            WHERE e.id = NEW.evidence_id AND s.project_id = e.project_id
        ) THEN RAISE(ABORT, 'evidence_link_target_invalid')
        WHEN NEW.target_type = 'session' AND NOT EXISTS (
            SELECT 1
            FROM evidences e
            JOIN development_sessions s ON s.id = NEW.target_id
            WHERE e.id = NEW.evidence_id AND s.project_id = e.project_id
        ) THEN RAISE(ABORT, 'evidence_link_target_invalid')
    END;
END;
