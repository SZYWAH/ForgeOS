use std::{str::FromStr, sync::Arc};

use chrono::Utc;
use thiserror::Error;

use crate::{
    domain::{AppPreferences, DomainError, LanguageMode},
    ports::{AppPreferencesRepository, RepositoryError},
};

#[derive(Clone)]
pub struct AppPreferencesKernel {
    repository: Arc<dyn AppPreferencesRepository>,
}

#[derive(Debug)]
pub struct UpdateAppPreferencesInput {
    pub language_mode: String,
    pub show_technical_terms: bool,
    pub expected_version: i64,
}

#[derive(Debug, Error)]
pub enum AppPreferencesError {
    #[error(transparent)]
    Validation(#[from] DomainError),
    #[error("application preferences are missing")]
    NotFound,
    #[error("application preferences changed after they were read")]
    VersionConflict,
    #[error("stored application preferences are invalid: {0}")]
    CorruptData(String),
    #[error("application preferences storage is unavailable: {0}")]
    Storage(String),
}

impl AppPreferencesKernel {
    pub fn new(repository: Arc<dyn AppPreferencesRepository>) -> Self {
        Self { repository }
    }

    pub async fn get(&self) -> Result<AppPreferences, AppPreferencesError> {
        self.repository
            .get()
            .await
            .map_err(map_repository_error)?
            .ok_or(AppPreferencesError::NotFound)
    }

    pub async fn update(
        &self,
        input: UpdateAppPreferencesInput,
    ) -> Result<AppPreferences, AppPreferencesError> {
        let current = self.get().await?;
        if current.version() != input.expected_version {
            return Err(AppPreferencesError::VersionConflict);
        }
        let next = current.update(
            LanguageMode::from_str(&input.language_mode)?,
            input.show_technical_terms,
            Utc::now(),
        )?;
        self.repository
            .update(&next, input.expected_version)
            .await
            .map_err(map_repository_error)?;
        Ok(next)
    }
}

fn map_repository_error(error: RepositoryError) -> AppPreferencesError {
    match error {
        RepositoryError::OptimisticConflict | RepositoryError::Conflict => {
            AppPreferencesError::VersionConflict
        }
        RepositoryError::CorruptData(message) => AppPreferencesError::CorruptData(message),
        RepositoryError::Storage(message) => AppPreferencesError::Storage(message),
        other => AppPreferencesError::Storage(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;

    use super::*;

    struct MemoryRepository(Mutex<AppPreferences>);

    #[async_trait]
    impl AppPreferencesRepository for MemoryRepository {
        async fn get(&self) -> Result<Option<AppPreferences>, RepositoryError> {
            Ok(Some(self.0.lock().unwrap().clone()))
        }

        async fn update(
            &self,
            preferences: &AppPreferences,
            expected_version: i64,
        ) -> Result<(), RepositoryError> {
            let mut current = self.0.lock().unwrap();
            if current.version() != expected_version {
                return Err(RepositoryError::OptimisticConflict);
            }
            *current = preferences.clone();
            Ok(())
        }
    }

    #[tokio::test]
    async fn update_requires_the_current_version() {
        let now = Utc::now();
        let repository = Arc::new(MemoryRepository(Mutex::new(
            AppPreferences::restore(LanguageMode::System, false, 1, now, now).unwrap(),
        )));
        let kernel = AppPreferencesKernel::new(repository);
        let updated = kernel
            .update(UpdateAppPreferencesInput {
                language_mode: "zh-CN".into(),
                show_technical_terms: true,
                expected_version: 1,
            })
            .await
            .unwrap();
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
    }
}
