CREATE TABLE app_preferences (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    language_mode TEXT NOT NULL DEFAULT 'system'
        CHECK (language_mode IN ('system', 'zh-CN', 'en-US')),
    show_technical_terms INTEGER NOT NULL DEFAULT 0
        CHECK (show_technical_terms IN (0, 1)),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

INSERT INTO app_preferences (
    id,
    language_mode,
    show_technical_terms,
    version,
    created_at,
    updated_at
)
VALUES (
    1,
    'system',
    0,
    1,
    strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
    strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
);
