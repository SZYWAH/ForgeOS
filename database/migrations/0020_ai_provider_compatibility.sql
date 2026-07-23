-- AI Provider Compatibility Layer
--
-- Secrets remain outside SQLite. Only the selected credential source and an
-- optional environment variable name are persisted.

ALTER TABLE ai_providers ADD COLUMN protocol TEXT NOT NULL
    DEFAULT 'openai_chat_completions'
    CHECK (protocol IN (
        'openai_chat_completions',
        'openai_responses',
        'anthropic_messages',
        'azure_openai',
        'openai_compatible'
    ));

ALTER TABLE ai_providers ADD COLUMN auth_scheme TEXT NOT NULL
    DEFAULT 'bearer'
    CHECK (auth_scheme IN ('bearer', 'api_key', 'x_api_key', 'custom_header', 'none'));

ALTER TABLE ai_providers ADD COLUMN secret_source TEXT NOT NULL
    DEFAULT 'credential_manager'
    CHECK (secret_source IN ('credential_manager', 'environment', 'none'));

ALTER TABLE ai_providers ADD COLUMN secret_required INTEGER NOT NULL DEFAULT 0
    CHECK (secret_required IN (0, 1));

ALTER TABLE ai_providers ADD COLUMN environment_variable TEXT NULL
    CHECK (environment_variable IS NULL OR length(environment_variable) BETWEEN 1 AND 128);

ALTER TABLE ai_providers ADD COLUMN custom_auth_header TEXT NULL
    CHECK (custom_auth_header IS NULL OR length(custom_auth_header) BETWEEN 1 AND 128);

ALTER TABLE ai_providers ADD COLUMN api_version TEXT NULL
    CHECK (api_version IS NULL OR length(api_version) BETWEEN 1 AND 100);

UPDATE ai_providers
SET secret_required = 1
WHERE lower(base_url) NOT LIKE 'http://localhost%'
  AND lower(base_url) NOT LIKE 'http://127.0.0.1%'
  AND lower(base_url) NOT LIKE 'http://[::1]%';

UPDATE ai_providers
SET auth_scheme = 'none', secret_source = 'none', secret_required = 0
WHERE lower(base_url) LIKE 'http://localhost%'
   OR lower(base_url) LIKE 'http://127.0.0.1%'
   OR lower(base_url) LIKE 'http://[::1]%';

CREATE INDEX ai_providers_protocol_enabled_index
    ON ai_providers (protocol, enabled, updated_at DESC);
