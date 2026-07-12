-- AI Provider Foundation
--
-- Provider metadata is local configuration, not an engineering fact. API keys
-- are intentionally absent and live in the operating-system credential store.

CREATE TABLE ai_providers (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL
        CHECK (length(trim(name)) > 0),
    base_url TEXT NOT NULL
        CHECK (
            length(trim(base_url)) > 0
            AND (base_url LIKE 'https://%' OR base_url LIKE 'http://%')
        ),
    model TEXT NOT NULL
        CHECK (length(trim(model)) > 0),
    enabled INTEGER NOT NULL DEFAULT 0
        CHECK (enabled IN (0, 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1
        CHECK (version >= 1)
);

CREATE UNIQUE INDEX ai_providers_name_unique_index
    ON ai_providers (name COLLATE NOCASE);

CREATE INDEX ai_providers_enabled_updated_index
    ON ai_providers (enabled, updated_at DESC, name COLLATE NOCASE);
