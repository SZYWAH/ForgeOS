-- D1.6 AI Candidate Foundation
--
-- Candidates govern AI output without promoting it to a formal engineering
-- fact. Status changes record human review only and never mutate source facts.

CREATE UNIQUE INDEX context_exports_identity_project_index
    ON context_exports (id, project_id);

CREATE TABLE ai_candidates (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,
    candidate_type TEXT NOT NULL
        CHECK (candidate_type IN (
            'snapshot_update',
            'session_summary',
            'evidence_suggestion',
            'risk_finding',
            'general_note'
        )),
    source_context_export_id TEXT NOT NULL,
    title TEXT NOT NULL
        CHECK (length(trim(title)) > 0),
    content_markdown TEXT NOT NULL
        CHECK (length(trim(content_markdown)) > 0),
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'accepted', 'rejected', 'archived')),
    version INTEGER NOT NULL DEFAULT 1
        CHECK (version >= 1),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CONSTRAINT ai_candidates_project_fk
        FOREIGN KEY (project_id)
        REFERENCES projects (id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    CONSTRAINT ai_candidates_context_project_fk
        FOREIGN KEY (source_context_export_id, project_id)
        REFERENCES context_exports (id, project_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
);

CREATE INDEX ai_candidates_project_status_updated_index
    ON ai_candidates (project_id, status, updated_at DESC, created_at DESC);

CREATE INDEX ai_candidates_project_created_index
    ON ai_candidates (project_id, created_at DESC, id DESC);

CREATE INDEX ai_candidates_source_context_index
    ON ai_candidates (source_context_export_id, created_at DESC);
