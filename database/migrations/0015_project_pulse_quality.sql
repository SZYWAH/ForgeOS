ALTER TABLE project_pulses
ADD COLUMN activity_type TEXT NOT NULL DEFAULT 'unknown'
    CHECK (activity_type IN (
        'implementation',
        'debugging',
        'release',
        'review',
        'support_question',
        'unknown'
    ));

ALTER TABLE project_pulses
ADD COLUMN analysis_revision TEXT NOT NULL DEFAULT 'legacy-v1';

ALTER TABLE project_pulses
ADD COLUMN source_window_started_at TEXT;

ALTER TABLE project_pulses
ADD COLUMN source_window_ended_at TEXT;

ALTER TABLE project_pulses
ADD COLUMN selected_observation_count INTEGER NOT NULL DEFAULT 0
    CHECK (selected_observation_count >= 0);

ALTER TABLE project_pulses
ADD COLUMN available_observation_count INTEGER NOT NULL DEFAULT 0
    CHECK (available_observation_count >= 0);

CREATE INDEX idx_project_pulses_revision
    ON project_pulses (project_id, analysis_revision, generated_at DESC);
