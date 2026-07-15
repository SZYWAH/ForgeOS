use std::sync::Arc;

use forgeos_lib::{
    application::app_preferences::{
        AppPreferencesError, AppPreferencesKernel, UpdateAppPreferencesInput,
    },
    infrastructure::{app_preferences::SqliteAppPreferencesRepository, database},
};

#[tokio::test]
async fn application_preferences_are_singleton_persistent_and_versioned() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("forgeos-preferences.db");
    let data = database::connect(&path).await.unwrap();

    let seeded = sqlx::query_as::<_, (i64, String, bool, i64)>(
        "SELECT id, language_mode, show_technical_terms, version FROM app_preferences",
    )
    .fetch_all(&data.pool)
    .await
    .unwrap();
    assert_eq!(seeded, vec![(1, "system".into(), false, 1)]);

    let repository = Arc::new(SqliteAppPreferencesRepository::new(data.pool.clone()));
    let kernel = AppPreferencesKernel::new(repository);
    let updated = kernel
        .update(UpdateAppPreferencesInput {
            language_mode: "zh-CN".into(),
            show_technical_terms: true,
            expected_version: 1,
        })
        .await
        .unwrap();
    assert_eq!(updated.language_mode().as_str(), "zh-CN");
    assert!(updated.show_technical_terms());
    assert_eq!(updated.version(), 2);

    assert!(matches!(
        kernel
            .update(UpdateAppPreferencesInput {
                language_mode: "en-US".into(),
                show_technical_terms: false,
                expected_version: 1,
            })
            .await,
        Err(AppPreferencesError::VersionConflict)
    ));

    data.pool.close().await;
    let reopened = database::connect(&path).await.unwrap();
    let restored = AppPreferencesKernel::new(Arc::new(SqliteAppPreferencesRepository::new(
        reopened.pool.clone(),
    )))
    .get()
    .await
    .unwrap();
    assert_eq!(restored.language_mode().as_str(), "zh-CN");
    assert!(restored.show_technical_terms());
    assert_eq!(restored.version(), 2);

    assert!(sqlx::query("INSERT INTO app_preferences (id, language_mode, show_technical_terms, version, created_at, updated_at) VALUES (2, 'system', 0, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')")
        .execute(&reopened.pool)
        .await
        .is_err());
    assert!(
        sqlx::query("UPDATE app_preferences SET language_mode = 'fr-FR' WHERE id = 1")
            .execute(&reopened.pool)
            .await
            .is_err()
    );
}
