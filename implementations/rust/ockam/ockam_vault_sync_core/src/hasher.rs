use crate::{
    VaultRequestMessage, VaultResponseMessage, VaultSync, VaultSyncCoreError, VaultSyncState,
};
use ockam_core::Result;
use ockam_node::block_future;
use ockam_vault_core::{Hasher, Secret, SecretAttributes, SmallBuffer};

impl Hasher for VaultSync {
    fn sha256(&mut self, data: &[u8]) -> Result<[u8; 32]> {
        match &mut self.0 {
            VaultSyncState::Worker { state } => block_future(&state.ctx().runtime(), async move {
                state
                    .send_message(VaultRequestMessage::Sha256 { data: data.into() })
                    .await?;

                let resp = state.receive_message().await?;

                if let VaultResponseMessage::Sha256(s) = resp {
                    Ok(s)
                } else {
                    Err(VaultSyncCoreError::InvalidResponseType.into())
                }
            }),
            VaultSyncState::Mutex { mutex } => mutex.lock().unwrap().sha256(data),
        }
    }

    fn hkdf_sha256(
        &mut self,
        salt: &Secret,
        info: &[u8],
        ikm: Option<&Secret>,
        output_attributes: SmallBuffer<SecretAttributes>,
    ) -> Result<SmallBuffer<Secret>> {
        match &mut self.0 {
            VaultSyncState::Worker { state } => block_future(&state.ctx().runtime(), async move {
                state
                    .send_message(VaultRequestMessage::HkdfSha256 {
                        salt: salt.clone(),
                        info: info.into(),
                        ikm: ikm.map(|v| v.clone()),
                        output_attributes,
                    })
                    .await?;

                let resp = state.receive_message().await?;

                if let VaultResponseMessage::HkdfSha256(s) = resp {
                    Ok(s)
                } else {
                    Err(VaultSyncCoreError::InvalidResponseType.into())
                }
            }),
            VaultSyncState::Mutex { mutex } => {
                mutex
                    .lock()
                    .unwrap()
                    .hkdf_sha256(salt, info, ikm, output_attributes)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use ockam_vault::SoftwareVault;
    use ockam_vault_test_attribute::*;

    fn new_vault() -> SoftwareVault {
        SoftwareVault::default()
    }

    #[vault_test_sync]
    fn sha256() {}

    #[vault_test_sync]
    fn hkdf() {}
}
