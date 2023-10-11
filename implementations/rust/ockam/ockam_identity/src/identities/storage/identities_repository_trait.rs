use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Result;
use ockam_core::{async_trait, Error};

use crate::models::{ChangeHistory, Identifier};
use crate::{AttributesEntry, Identity};

/// Repository for data related to identities: key changes and attributes
#[async_trait]
pub trait IdentitiesRepository:
    IdentityAttributesReader + IdentityAttributesWriter + IdentitiesReader + IdentitiesWriter
{
    /// Restrict this repository as a reader for attributes
    fn as_attributes_reader(&self) -> Arc<dyn IdentityAttributesReader>;

    /// Restrict this repository as a writer for attributes
    fn as_attributes_writer(&self) -> Arc<dyn IdentityAttributesWriter>;

    /// Restrict this repository as a reader for identities
    fn as_identities_reader(&self) -> Arc<dyn IdentitiesReader>;

    /// Restrict this repository as a writer for identities
    fn as_identities_writer(&self) -> Arc<dyn IdentitiesWriter>;
}

/// Trait implementing read access to attributes
#[async_trait]
pub trait IdentityAttributesReader: Send + Sync + 'static {
    /// Get the attributes associated with the given identity identifier
    async fn get_attributes(&self, identity: &Identifier) -> Result<Option<AttributesEntry>>;

    /// List all identities with their attributes
    async fn list(&self) -> Result<Vec<(Identifier, AttributesEntry)>>;
}

/// Trait implementing write access to attributes
#[async_trait]
pub trait IdentityAttributesWriter: Send + Sync + 'static {
    /// Set the attributes associated with the given identity identifier.
    /// Previous values gets overridden.
    async fn put_attributes(&self, identity: &Identifier, entry: AttributesEntry) -> Result<()>;

    /// Store an attribute name/value pair for a given identity
    async fn put_attribute_value(
        &self,
        subject: &Identifier,
        attribute_name: Vec<u8>,
        attribute_value: Vec<u8>,
    ) -> Result<()>;

    /// Remove all attributes for a given identity identifier
    async fn delete(&self, identity: &Identifier) -> Result<()>;
}

/// Trait implementing write access to identities
#[async_trait]
pub trait IdentitiesWriter: Send + Sync + 'static {
    /// Store changes if there are new key changes associated to that identity
    async fn create_identity(&self, identity: &Identity, name: Option<&str>) -> Result<()>;

    /// Store changes if there are new key changes associated to that identity
    async fn update_identity(&self, identity: &Identity) -> Result<()>;

    /// Delete an identity given its identifier
    async fn delete_identity(&self, identifier: &Identifier) -> Result<()>;

    /// Delete an identity given its name
    async fn delete_identity_by_name(&self, name: &str) -> Result<()>;
}

/// Trait implementing read access to identities
#[async_trait]
pub trait IdentitiesReader: Send + Sync + 'static {
    /// Return the change history of a persisted identity
    async fn get_change_history_optional(
        &self,
        identifier: &Identifier,
    ) -> Result<Option<ChangeHistory>>;

    /// Return the change history of a persisted identity
    async fn get_change_history(&self, identifier: &Identifier) -> Result<ChangeHistory> {
        match self.get_change_history_optional(identifier).await? {
            Some(change_history) => Ok(change_history),
            None => Err(Error::new(
                Origin::Core,
                Kind::NotFound,
                format!("identity not found for identifier {}", identifier),
            )),
        }
    }
}
