use crate::identities::{Identities, IdentitiesRepository};
use crate::purpose_keys::storage::PurposeKeysRepository;
use crate::{Vault, VaultStorage};

use ockam_core::compat::sync::Arc;

/// Builder for Identities services
#[derive(Clone)]
pub struct IdentitiesBuilder {
    pub(crate) vault: Vault,
    pub(crate) repository: Arc<dyn IdentitiesRepository>,
    pub(crate) purpose_keys_repository: Arc<dyn PurposeKeysRepository>,
}

/// Return a default identities
pub fn identities() -> Arc<Identities> {
    Identities::builder().build()
}

impl IdentitiesBuilder {
    /// With Software Vault with given Storage
    pub fn with_vault_storage(mut self, storage: VaultStorage) -> Self {
        self.vault = Vault::create_with_persistent_storage(storage);
        self
    }

    /// Set a Vault
    pub fn with_vault(mut self, vault: Vault) -> Self {
        self.vault = vault;
        self
    }

    /// Set a specific repository for identities
    pub fn with_identities_repository(mut self, repository: Arc<dyn IdentitiesRepository>) -> Self {
        self.repository = repository;
        self
    }

    /// Set a specific repository for Purpose Keys
    pub fn with_purpose_keys_repository(
        mut self,
        repository: Arc<dyn PurposeKeysRepository>,
    ) -> Self {
        self.purpose_keys_repository = repository;
        self
    }

    /// Build identities
    pub fn build(self) -> Arc<Identities> {
        Arc::new(Identities::new(
            self.vault,
            self.repository,
            self.purpose_keys_repository,
        ))
    }
}
