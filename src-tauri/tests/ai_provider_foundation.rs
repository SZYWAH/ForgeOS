use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

use async_trait::async_trait;
use forgeos_lib::{
    application::ai_provider::{
        AiMessageInput, CreateAiProviderInput, InvokeAiProviderInput, SetAiProviderEnabledInput,
        UpdateAiProviderInput,
    },
    application::{AiProviderError, AiProviderKernel},
    infrastructure::{database, repositories::SqliteProjectRepository},
    ports::{
        AiClientError, AiCompletion, AiMessage, AiMessageRole, AiProviderClient, AiTokenUsage,
        ApiKeyStore, SecretStoreError,
    },
};
use tempfile::TempDir;

#[derive(Default)]
struct MemoryApiKeyStore {
    values: Mutex<HashMap<String, String>>,
}

#[async_trait]
impl ApiKeyStore for MemoryApiKeyStore {
    async fn set(&self, provider_id: &str, api_key: &str) -> Result<(), SecretStoreError> {
        self.values
            .lock()
            .expect("secret store lock should work")
            .insert(provider_id.to_owned(), api_key.to_owned());
        Ok(())
    }

    async fn get(&self, provider_id: &str) -> Result<Option<String>, SecretStoreError> {
        Ok(self
            .values
            .lock()
            .expect("secret store lock should work")
            .get(provider_id)
            .cloned())
    }

    async fn clear(&self, provider_id: &str) -> Result<(), SecretStoreError> {
        self.values
            .lock()
            .expect("secret store lock should work")
            .remove(provider_id);
        Ok(())
    }
}

#[derive(Default)]
struct FakeAiClient {
    calls: AtomicUsize,
    last_key: Mutex<Option<String>>,
    last_messages: Mutex<Vec<AiMessage>>,
}

#[async_trait]
impl AiProviderClient for FakeAiClient {
    async fn complete(
        &self,
        provider: &forgeos_lib::domain::AiProvider,
        api_key: Option<String>,
        messages: &[AiMessage],
    ) -> Result<AiCompletion, AiClientError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        *self.last_key.lock().expect("key lock should work") = api_key;
        *self.last_messages.lock().expect("message lock should work") = messages.to_vec();
        Ok(AiCompletion {
            content: "explicit test response".into(),
            model: provider.model().into(),
            finish_reason: Some("stop".into()),
            usage: Some(AiTokenUsage {
                prompt_tokens: Some(5),
                completion_tokens: Some(3),
                total_tokens: Some(8),
            }),
        })
    }
}

fn provider_input() -> CreateAiProviderInput {
    CreateAiProviderInput {
        name: "Local Test Provider".into(),
        base_url: "https://provider.example.test/v1".into(),
        model: "qwen3:8b".into(),
        protocol: "openai_chat_completions".into(),
        auth_scheme: "bearer".into(),
        secret_source: "credential_manager".into(),
        secret_required: true,
        environment_variable: None,
        custom_auth_header: None,
        api_version: None,
    }
}

#[tokio::test]
async fn provider_management_explicit_invocation_and_persistence() {
    let directory = TempDir::new().expect("temporary directory should be created");
    let database_path = directory.path().join("forgeos-ai-provider-test.db");
    let database = database::connect(&database_path)
        .await
        .expect("database should initialize");
    let repository = Arc::new(SqliteProjectRepository::new(database.pool.clone()));
    let secrets = Arc::new(MemoryApiKeyStore::default());
    let client = Arc::new(FakeAiClient::default());
    let kernel = AiProviderKernel::new(repository, secrets.clone(), client.clone());

    let created = kernel
        .create_provider(provider_input())
        .await
        .expect("Provider should be created");
    assert!(!created.provider.enabled());
    assert!(!created.api_key_configured);
    assert!(!created.secret_available);
    assert_eq!(created.provider.version(), 1);

    assert!(matches!(
        kernel.create_provider(provider_input()).await,
        Err(AiProviderError::NameConflict)
    ));

    let with_key = kernel
        .set_api_key(created.provider.id(), "test-secret-key".into())
        .await
        .expect("API key should be configured");
    assert!(with_key.api_key_configured);
    assert!(with_key.secret_available);

    let messages = InvokeAiProviderInput {
        messages: vec![AiMessageInput {
            role: "user".into(),
            content: "ping".into(),
        }],
    };
    assert!(matches!(
        kernel
            .invoke_provider(created.provider.id(), messages.clone())
            .await,
        Err(AiProviderError::Validation(
            forgeos_lib::domain::DomainError::AiProviderDisabled
        ))
    ));
    assert_eq!(client.calls.load(Ordering::SeqCst), 0);

    let enabled = kernel
        .set_provider_enabled(
            created.provider.id(),
            SetAiProviderEnabledInput {
                enabled: true,
                expected_version: created.provider.version(),
            },
        )
        .await
        .expect("Provider should enable");
    assert!(enabled.provider.enabled());
    assert_eq!(enabled.provider.version(), 2);

    assert!(matches!(
        kernel
            .update_provider(
                enabled.provider.id(),
                UpdateAiProviderInput {
                    name: "Stale".into(),
                    base_url: "http://localhost:1234/v1".into(),
                    model: "stale-model".into(),
                    protocol: "openai_compatible".into(),
                    auth_scheme: "none".into(),
                    secret_source: "none".into(),
                    secret_required: false,
                    environment_variable: None,
                    custom_auth_header: None,
                    api_version: None,
                    expected_version: 1,
                },
            )
            .await,
        Err(AiProviderError::VersionConflict {
            expected: 1,
            actual: 2
        })
    ));

    let result = kernel
        .invoke_provider(enabled.provider.id(), messages)
        .await
        .expect("explicit invocation should use the client");
    assert_eq!(result.completion.content, "explicit test response");
    assert_eq!(client.calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        client
            .last_key
            .lock()
            .expect("key lock should work")
            .as_deref(),
        Some("test-secret-key")
    );
    let recorded_messages = client
        .last_messages
        .lock()
        .expect("message lock should work");
    assert_eq!(recorded_messages.len(), 1);
    assert_eq!(recorded_messages[0].role, AiMessageRole::User);
    drop(recorded_messages);

    let updated = kernel
        .update_provider(
            enabled.provider.id(),
            UpdateAiProviderInput {
                name: "LM Studio".into(),
                base_url: "http://localhost:1234/v1/".into(),
                model: "local-model".into(),
                protocol: "openai_compatible".into(),
                auth_scheme: "none".into(),
                secret_source: "none".into(),
                secret_required: false,
                environment_variable: None,
                custom_auth_header: None,
                api_version: None,
                expected_version: enabled.provider.version(),
            },
        )
        .await
        .expect("Provider should update");
    assert_eq!(updated.provider.base_url(), "http://localhost:1234/v1");
    assert_eq!(updated.provider.version(), 3);

    let cleared = kernel
        .clear_api_key(updated.provider.id())
        .await
        .expect("API key should clear");
    assert!(!cleared.api_key_configured);

    let columns: Vec<String> =
        sqlx::query_scalar("SELECT name FROM pragma_table_info('ai_providers')")
            .fetch_all(&database.pool)
            .await
            .expect("provider schema should be readable");
    assert!(!columns.iter().any(|column| column == "api_key"));
    for expected in [
        "protocol",
        "auth_scheme",
        "secret_source",
        "secret_required",
        "environment_variable",
        "custom_auth_header",
        "api_version",
    ] {
        assert!(columns.iter().any(|column| column == expected));
    }
    let project_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects")
        .fetch_one(&database.pool)
        .await
        .expect("Project table should remain readable");
    assert_eq!(
        project_count, 0,
        "AI invocation must not create Project facts"
    );
    let evidence_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM evidences")
        .fetch_one(&database.pool)
        .await
        .expect("Evidence table should remain readable");
    assert_eq!(
        evidence_count, 0,
        "AI invocation must not create Evidence facts"
    );

    let integrity: String = sqlx::query_scalar("PRAGMA integrity_check")
        .fetch_one(&database.pool)
        .await
        .expect("integrity_check should run");
    assert_eq!(integrity, "ok");
    let foreign_key_violations: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM pragma_foreign_key_check")
            .fetch_one(&database.pool)
            .await
            .expect("foreign_key_check should run");
    assert_eq!(foreign_key_violations, 0);

    let provider_id = updated.provider.id().to_owned();
    database.pool.close().await;
    drop(kernel);

    let reopened = database::connect(&database_path)
        .await
        .expect("database should reopen");
    let reopened_repository = Arc::new(SqliteProjectRepository::new(reopened.pool.clone()));
    let reopened_kernel = AiProviderKernel::new(reopened_repository, secrets, client);
    let persisted = reopened_kernel
        .get_provider(&provider_id)
        .await
        .expect("Provider configuration should persist");
    assert_eq!(persisted.provider.name(), "LM Studio");
    assert!(persisted.provider.enabled());
    assert_eq!(persisted.provider.version(), 3);
    assert!(!persisted.api_key_configured);
    reopened.pool.close().await;
}

#[tokio::test]
async fn environment_credentials_are_resolved_explicitly_without_sqlite_values() {
    const ENVIRONMENT_NAME: &str = "FORGEOS_TEST_AI_PROVIDER_SECRET";
    const SECRET: &str = "runtime-only-test-secret";
    // SAFETY: this test uses a process-unique variable name and removes it before returning.
    unsafe { std::env::set_var(ENVIRONMENT_NAME, SECRET) };

    let directory = TempDir::new().expect("temporary directory should be created");
    let database = database::connect(&directory.path().join("environment-provider.db"))
        .await
        .expect("database should initialize");
    let client = Arc::new(FakeAiClient::default());
    let kernel = AiProviderKernel::new(
        Arc::new(SqliteProjectRepository::new(database.pool.clone())),
        Arc::new(MemoryApiKeyStore::default()),
        client.clone(),
    );
    let created = kernel
        .create_provider(CreateAiProviderInput {
            name: "Environment Provider".into(),
            base_url: "https://api.example.test/v1".into(),
            model: "model".into(),
            protocol: "openai_compatible".into(),
            auth_scheme: "bearer".into(),
            secret_source: "environment".into(),
            secret_required: true,
            environment_variable: Some(ENVIRONMENT_NAME.into()),
            custom_auth_header: None,
            api_version: None,
        })
        .await
        .expect("environment Provider should be created");
    assert!(created.secret_available);
    assert!(!created.api_key_configured);
    let enabled = kernel
        .set_provider_enabled(
            created.provider.id(),
            SetAiProviderEnabledInput {
                enabled: true,
                expected_version: created.provider.version(),
            },
        )
        .await
        .expect("environment Provider should enable");
    kernel
        .invoke_provider(
            enabled.provider.id(),
            InvokeAiProviderInput {
                messages: vec![AiMessageInput {
                    role: "user".into(),
                    content: "ping".into(),
                }],
            },
        )
        .await
        .expect("environment credential should be resolved at invocation");
    assert_eq!(
        client
            .last_key
            .lock()
            .expect("key lock should work")
            .as_deref(),
        Some(SECRET)
    );
    let serialized_rows: Vec<String> = sqlx::query_scalar(
        "SELECT name || '|' || base_url || '|' || model || '|' || COALESCE(environment_variable, '') FROM ai_providers",
    )
    .fetch_all(&database.pool)
    .await
    .expect("Provider rows should be readable");
    assert!(serialized_rows.iter().all(|row| !row.contains(SECRET)));

    unsafe { std::env::remove_var(ENVIRONMENT_NAME) };
    database.pool.close().await;
}
