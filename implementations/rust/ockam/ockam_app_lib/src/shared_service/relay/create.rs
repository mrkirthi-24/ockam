use crate::api::state::OrchestratorStatus;
use crate::state::{AppState, NODE_NAME};
use crate::Result;
use miette::IntoDiagnostic;
use ockam::Context;
use ockam_api::cli_state::{CliState, StateDirTrait};
use ockam_api::nodes::models::relay::{CreateRelay, RelayInfo};
use ockam_api::nodes::{InMemoryNode, NodeManagerWorker};
use ockam_multiaddr::MultiAddr;
use once_cell::sync::Lazy;
use std::str::FromStr;
use std::sync::Arc;
use tracing::{debug, info, trace, warn};

pub static RELAY_NAME: Lazy<String> = Lazy::new(|| format!("forward_to_{NODE_NAME}"));

impl AppState {
    /// Try to create a relay until it succeeds.
    pub async fn create_relay(
        &self,
        context: Arc<Context>,
        cli_state: CliState,
        node_manager: Arc<InMemoryNode>,
    ) {
        self.update_orchestrator_status(OrchestratorStatus::Connecting);
        self.publish_state().await;

        loop {
            match self
                .create_relay_impl(&context, &cli_state, node_manager.clone())
                .await
            {
                Ok(_) => break,
                Err(e) => {
                    warn!(%e, "Failed to create relay, retrying...");
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        }
    }

    /// Create a relay at the default project if doesn't exist yet
    ///
    /// Once it's created, a `Medic` worker will monitor it and recreate it whenever it's unresponsive
    async fn create_relay_impl(
        &self,
        context: &Context,
        cli_state: &CliState,
        node_manager: Arc<InMemoryNode>,
    ) -> Result<Option<RelayInfo>> {
        trace!("Creating relay");
        if !cli_state.is_enrolled().unwrap_or(false) {
            trace!("Not enrolled, skipping relay creation");
            return Ok(None);
        }
        match cli_state.projects.default() {
            Ok(project) => {
                if let Some(relay) = Self::get_relay(node_manager.clone()).await {
                    debug!(project = %project.name(), "Relay already exists");
                    self.update_orchestrator_status(OrchestratorStatus::Connected);
                    self.publish_state().await;
                    Ok(Some(relay.clone()))
                } else {
                    debug!(project = %project.name(), "Creating relay at project");
                    let project_route = format!("/project/{}", project.name());
                    let project_address = MultiAddr::from_str(&project_route).into_diagnostic()?;
                    let relay = node_manager
                        .create_relay(
                            context,
                            &project_address,
                            Some(NODE_NAME.to_string()),
                            false,
                            None,
                        )
                        .await
                        .into_diagnostic()?;
                    info!(forwarding_route = %relay.forwarding_route(), "Relay created at project");
                    self.update_orchestrator_status(OrchestratorStatus::Connected);
                    self.publish_state().await;
                    Ok(Some(relay))
                }
            }
            Err(err) => {
                warn!(%err, "No default project has ben set");
                Ok(None)
            }
        }
    }

    pub(crate) async fn get_relay(node_manager: Arc<InMemoryNode>) -> Option<RelayInfo> {
        node_manager
            .get_relays()
            .await
            .into_iter()
            .find(|r| r.remote_address() == *RELAY_NAME)
    }
}
