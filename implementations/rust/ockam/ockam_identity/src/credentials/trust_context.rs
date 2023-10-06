use ockam_core::compat::string::String;

use crate::models::Identifier;

/// A trust context defines which authorities are trusted to attest to which attributes, within a context.
/// Our first implementation assumes that there is only one authority and it is trusted to attest to all attributes within this context.
#[derive(Clone)]
pub struct TrustContext {
    /// This is the ID of the trust context; which is primarily used for ABAC policies
    id: String, // FIXME: Is it Checked?
    /// Identifier of the Authority which Credentials we consider trusted
    authority: Identifier,
}

impl TrustContext {
    /// Create a new Trust Context
    pub fn new(id: String, authority: Identifier) -> Self {
        Self { id, authority }
    }

    /// Return the ID of the Trust Context
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Return the Authority of the Trust Context
    pub fn authority(&self) -> &Identifier {
        &self.authority
    }
}
