use async_trait::async_trait;

use crate::domain::AppPreferences;

use super::RepositoryError;

#[async_trait]
pub trait AppPreferencesRepository: Send + Sync {
    async fn get(&self) -> Result<Option<AppPreferences>, RepositoryError>;

    async fn update(
        &self,
        preferences: &AppPreferences,
        expected_version: i64,
    ) -> Result<(), RepositoryError>;
}
