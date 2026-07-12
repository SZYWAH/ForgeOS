use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use keyring::{Entry, Error as KeyringError};

use crate::ports::{ApiKeyStore, SecretStoreError};

const SERVICE_NAME: &str = "ForgeOS AI Provider";

#[derive(Clone, Default)]
pub struct WindowsCredentialApiKeyStore {
    gate: Arc<Mutex<()>>,
}

#[async_trait]
impl ApiKeyStore for WindowsCredentialApiKeyStore {
    async fn set(&self, provider_id: &str, api_key: &str) -> Result<(), SecretStoreError> {
        let provider_id = provider_id.to_owned();
        let api_key = api_key.to_owned();
        let gate = self.gate.clone();
        blocking(move || {
            let _guard = gate.lock().map_err(|error| {
                SecretStoreError::Unavailable(format!("credential lock poisoned: {error}"))
            })?;
            entry(&provider_id)?
                .set_password(&api_key)
                .map_err(secret_error)
        })
        .await
    }

    async fn get(&self, provider_id: &str) -> Result<Option<String>, SecretStoreError> {
        let provider_id = provider_id.to_owned();
        let gate = self.gate.clone();
        blocking(move || {
            let _guard = gate.lock().map_err(|error| {
                SecretStoreError::Unavailable(format!("credential lock poisoned: {error}"))
            })?;
            match entry(&provider_id)?.get_password() {
                Ok(value) => Ok(Some(value)),
                Err(KeyringError::NoEntry) => Ok(None),
                Err(error) => Err(secret_error(error)),
            }
        })
        .await
    }

    async fn clear(&self, provider_id: &str) -> Result<(), SecretStoreError> {
        let provider_id = provider_id.to_owned();
        let gate = self.gate.clone();
        blocking(move || {
            let _guard = gate.lock().map_err(|error| {
                SecretStoreError::Unavailable(format!("credential lock poisoned: {error}"))
            })?;
            match entry(&provider_id)?.delete_credential() {
                Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
                Err(error) => Err(secret_error(error)),
            }
        })
        .await
    }
}

fn entry(provider_id: &str) -> Result<Entry, SecretStoreError> {
    Entry::new(SERVICE_NAME, provider_id).map_err(secret_error)
}

async fn blocking<T, F>(operation: F) -> Result<T, SecretStoreError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, SecretStoreError> + Send + 'static,
{
    tokio::task::spawn_blocking(operation)
        .await
        .map_err(|error| SecretStoreError::Unavailable(error.to_string()))?
}

fn secret_error(error: KeyringError) -> SecretStoreError {
    SecretStoreError::Unavailable(error.to_string())
}
