use miette::IntoDiagnostic;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use tracing::{debug, info, trace, warn};

use ockam_api::address::get_free_address;
use ockam_api::cli_state::{CliState, StateDirTrait};
use ockam_api::cloud::project::Project;
use ockam_api::cloud::share::{
    AcceptInvitation, CreateServiceInvitation, InvitationWithAccess, Invitations,
};
use ockam_api::cloud::share::{InvitationListKind, ListInvitations};

use crate::background_node::BackgroundNodeClient;
use crate::invitations::state::{Inlet, ReceivedInvitationStatus};
use crate::shared_service::relay::RELAY_NAME;
use crate::state::{AppState, PROJECT_NAME};

impl AppState {
    pub async fn accept_invitation(&self, id: String) -> Result<(), String> {
        self.accept_invitation_impl(id)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn accept_invitation_impl(&self, id: String) -> crate::Result<()> {
        debug!(?id, "Accepting invitation");
        if !self.is_enrolled().await? {
            debug!(?id, "Not enrolled, invitation can't be accepted");
            return Ok(());
        }

        // Update the invitation status to Accepting if it's not already being processed.
        // Otherwise, return early.
        {
            let invitations = self.invitations();
            let mut writer = invitations.write().await;
            match writer.received.status.iter_mut().find(|x| x.0 == id) {
                None => {
                    writer
                        .received
                        .status
                        .push((id.clone(), ReceivedInvitationStatus::Accepting));
                    self.publish_state().await;
                }
                Some((i, s)) => {
                    return match s {
                        ReceivedInvitationStatus::Accepting => {
                            debug!(?i, "Invitation is being processed");
                            Ok(())
                        }
                        ReceivedInvitationStatus::Accepted => {
                            debug!(?i, "Invitation was already accepted");
                            Ok(())
                        }
                    }
                }
            }
        }

        let controller = self.controller().await?;
        let res = controller
            .accept_invitation(&self.context(), id.clone())
            .await?;

        debug!(?res);
        self.publish_state().await;
        info!(?id, "Invitation accepted");
        Ok(())
    }

    pub async fn create_service_invitation(
        &self,
        recipient_email: String,
        outlet_socket_addr: String,
    ) -> Result<(), String> {
        info!(
            ?recipient_email,
            ?outlet_socket_addr,
            "creating service invitation"
        );
        let projects = self.projects();
        let projects_guard = projects.read().await;
        let project_id = projects_guard
            .iter()
            .find(|p| p.name == *PROJECT_NAME)
            .map(|p| p.id.to_owned())
            .ok_or_else(|| "could not find default project".to_string())?;
        let enrollment_ticket = self
            .create_enrollment_ticket(project_id)
            .await
            .map_err(|e| e.to_string())?;

        let socket_addr = SocketAddr::from_str(outlet_socket_addr.as_str())
            .into_diagnostic()
            .map_err(|e| format!("Cannot parse the outlet address as a socket address: {e}"))?;
        let invite_args = self
            .build_args_for_create_service_invitation(
                &socket_addr,
                &recipient_email,
                enrollment_ticket,
            )
            .await
            .map_err(|e| e.to_string())?;

        let this = self.clone();
        tokio::spawn(async move {
            let result = this.send_invitation(invite_args).await;
            if let Err(e) = result {
                warn!(%e, "Failed to send invitation");
            }
        });
        Ok(())
    }

    async fn send_invitation(&self, invite_args: CreateServiceInvitation) -> crate::Result<()> {
        let controller = self.controller().await.into_diagnostic()?;
        let CreateServiceInvitation {
            expires_at,
            project_id,
            recipient_email,
            project_identity,
            project_route,
            project_authority_identity,
            project_authority_route,
            shared_node_identity,
            shared_node_route,
            enrollment_ticket,
        } = invite_args;
        let res = controller
            .create_service_invitation(
                &self.context(),
                expires_at,
                project_id,
                recipient_email,
                project_identity,
                project_route,
                project_authority_identity,
                project_authority_route,
                shared_node_identity,
                shared_node_route,
                enrollment_ticket,
            )
            .await
            .map_err(|e| e.to_string())?;
        debug!(?res, "invitation sent");
        Ok(())
    }

    pub async fn refresh_invitations(&self) -> Result<(), String> {
        debug!("Refreshing invitations");
        let invitations = {
            if !self.is_enrolled().await.unwrap_or(false) {
                debug!("not enrolled, skipping invitations refresh");
                return Ok(());
            }
            let controller = self.controller().await.map_err(|e| e.to_string())?;
            let invitations = controller
                .list_invitations(&self.context(), InvitationListKind::All)
                .await
                .map_err(|e| e.to_string())?;
            debug!("Invitations fetched");
            trace!(?invitations);
            invitations
        };

        self.invitations().write().await.replace_by(invitations);
        self.publish_state().await;
        Ok(())
    }

    pub(crate) async fn refresh_inlets(&self) -> crate::Result<()> {
        debug!("Refreshing inlets");

        let mut running_inlets = vec![];
        let invitations = self.invitations();
        let invitation_guard = invitations.read().await;
        {
            if invitation_guard.accepted.invitations.is_empty() {
                debug!("No accepted invitations, skipping inlets refresh");
                return Ok(());
            }

            let cli_state = self.state().await;
            let background_node_client = self.background_node_client().await;
            for invitation in &invitation_guard.accepted.invitations {
                match InletDataFromInvitation::new(
                    &cli_state,
                    invitation,
                    &invitation_guard.accepted.inlets,
                ) {
                    Ok(i) => match i {
                        Some(mut i) => {
                            if !i.enabled {
                                debug!(node = %i.local_node_name, "TCP inlet is disabled by the user, skipping");
                                continue;
                            }

                            debug!(node = %i.local_node_name, "Checking node status");
                            if let Ok(node) = cli_state.nodes.get(&i.local_node_name) {
                                if node.is_running() {
                                    debug!(node = %i.local_node_name, "Node already running");
                                    if let Ok(inlet) = background_node_client
                                        .inlets()
                                        .show(&i.local_node_name, &i.service_name)
                                        .await
                                    {
                                        i.socket_addr = Some(inlet.bind_addr.parse()?);
                                        running_inlets.push((invitation.invitation.id.clone(), i));
                                        continue;
                                    }
                                }
                            }
                            background_node_client
                                .nodes()
                                .delete(&i.local_node_name)
                                .await?;
                            match self.create_inlet(background_node_client.clone(), &i).await {
                                Ok(socket_addr) => {
                                    i.socket_addr = Some(socket_addr);
                                    running_inlets.push((invitation.invitation.id.clone(), i));
                                }
                                Err(err) => {
                                    warn!(%err, node = %i.local_node_name, "Failed to create TCP inlet for accepted invitation");
                                }
                            }
                        }
                        None => {
                            warn!("Invalid invitation data");
                        }
                    },
                    Err(err) => {
                        warn!(%err, "Failed to parse invitation data");
                    }
                }
            }
        }

        {
            let mut invitation_guard = invitations.write().await;
            for (invitation_id, i) in running_inlets {
                invitation_guard
                    .accepted
                    .inlets
                    .insert(invitation_id, Inlet::new(i)?);
            }
        }

        self.publish_state().await;
        info!("Inlets refreshed");
        Ok(())
    }

    /// Create the tcp-inlet for the accepted invitation
    /// Returns the inlet SocketAddr
    async fn create_inlet(
        &self,
        background_node_client: Arc<dyn BackgroundNodeClient>,
        inlet_data: &InletDataFromInvitation,
    ) -> crate::Result<SocketAddr> {
        debug!(service_name = ?inlet_data.service_name, "Creating TCP inlet for accepted invitation");
        let InletDataFromInvitation {
            enabled,
            local_node_name,
            service_name,
            service_route,
            enrollment_ticket_hex,
            socket_addr,
        } = inlet_data;
        if !enabled {
            return Err("TCP inlet is disabled by the user".into());
        }
        let from = match socket_addr {
            Some(socket_addr) => *socket_addr,
            None => get_free_address()?,
        };
        if let Some(enrollment_ticket_hex) = enrollment_ticket_hex {
            background_node_client
                .projects()
                .enroll(local_node_name, enrollment_ticket_hex)
                .await?;
        }
        background_node_client
            .nodes()
            .create(local_node_name)
            .await?;
        background_node_client
            .inlets()
            .create(local_node_name, &from, service_route, service_name)
            .await?;
        Ok(from)
    }

    pub(crate) async fn disconnect_tcp_inlet(&self, invitation_id: &str) -> crate::Result<()> {
        let background_node_client = self.background_node_client().await;
        let invitations = self.invitations();
        let mut writer = invitations.write().await;
        if let Some(inlet) = writer.accepted.inlets.get_mut(invitation_id) {
            if !inlet.enabled {
                debug!(node = %inlet.node_name, alias = %inlet.alias, "TCP inlet was already disconnected");
                return Ok(());
            }
            inlet.disable();
            background_node_client
                .inlets()
                .delete(&inlet.node_name, &inlet.alias)
                .await?;
            self.publish_state().await;
        }
        Ok(())
    }

    pub(crate) async fn enable_tcp_inlet(&self, invitation_id: &str) -> crate::Result<()> {
        let invitations = self.invitations();
        let mut writer = invitations.write().await;
        if let Some(inlet) = writer.accepted.inlets.get_mut(invitation_id) {
            if inlet.enabled {
                debug!(node = %inlet.node_name, alias = %inlet.alias, "TCP inlet was already enabled");
                return Ok(());
            }
            inlet.enable();
            self.publish_state().await;
            info!(node = %inlet.node_name, alias = %inlet.alias, "Enabled TCP inlet");
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct InletDataFromInvitation {
    pub enabled: bool,
    pub local_node_name: String,
    pub service_name: String,
    pub service_route: String,
    pub enrollment_ticket_hex: Option<String>,
    pub socket_addr: Option<SocketAddr>,
}

impl InletDataFromInvitation {
    pub fn new(
        cli_state: &CliState,
        invitation: &InvitationWithAccess,
        inlets: &HashMap<String, Inlet>,
    ) -> crate::Result<Option<Self>> {
        match &invitation.service_access_details {
            Some(d) => {
                let service_name = d.service_name()?;
                let mut enrollment_ticket = d.enrollment_ticket()?;
                // The enrollment ticket contains the project data.
                // We need to replace the project name on the enrollment ticket with the project id,
                // so that, when using the enrollment ticket, there are no conflicts with the default project.
                // The node created when setting up the TCP inlet is meant to only serve that TCP inlet and
                // only has to resolve the `/project/{id}` project to create the needed secure-channel.
                if let Some(project) = enrollment_ticket.project.as_mut() {
                    project.name = project.id.clone();
                }
                let enrollment_ticket_hex = if invitation.invitation.is_expired()? {
                    None
                } else {
                    Some(enrollment_ticket.hex_encoded()?)
                };

                if let Some(project) = enrollment_ticket.project {
                    // At this point, the project name will be the project id.
                    let project = cli_state
                        .projects
                        .overwrite(project.name.clone(), Project::from(project.clone()))?;
                    assert_eq!(
                        project.name(),
                        project.id(),
                        "Project name should be the project id"
                    );

                    let project_id = project.id();
                    let local_node_name = format!("ockam_app_{project_id}_{service_name}");
                    let service_route = format!(
                        "/project/{project_id}/service/{}/secure/api/service/{service_name}",
                        *RELAY_NAME
                    );

                    let inlet = inlets.get(&invitation.invitation.id);
                    let enabled = inlet.map(|i| i.enabled).unwrap_or(true);
                    let socket_addr = inlet.map(|i| i.socket_addr);

                    Ok(Some(Self {
                        enabled,
                        local_node_name,
                        service_name,
                        service_route,
                        enrollment_ticket_hex,
                        socket_addr,
                    }))
                } else {
                    warn!(?invitation, "No project data found in enrollment ticket");
                    Ok(None)
                }
            }
            None => {
                warn!(
                    ?invitation,
                    "No service details found in accepted invitation"
                );
                Ok(None)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ockam::identity::OneTimeCode;
    use ockam_api::cloud::share::{
        ReceivedInvitation, RoleInShare, ServiceAccessDetails, ShareScope,
    };
    use ockam_api::config::lookup::ProjectLookup;
    use ockam_api::identity::EnrollmentTicket;

    #[test]
    fn test_inlet_data_from_invitation() {
        let cli_state = CliState::test().unwrap();
        let mut inlets = HashMap::new();
        let mut invitation = InvitationWithAccess {
            invitation: ReceivedInvitation {
                id: "invitation_id".to_string(),
                expires_at: "2020-09-12T15:07:14.00".to_string(),
                grant_role: RoleInShare::Admin,
                owner_email: "owner_email".to_string(),
                scope: ShareScope::Project,
                target_id: "target_id".to_string(),
            },
            service_access_details: None,
        };

        // InletDataFromInvitation will be none because `service_access_details` is none
        assert!(
            InletDataFromInvitation::new(&cli_state, &invitation, &inlets)
                .unwrap()
                .is_none()
        );

        invitation.service_access_details = Some(ServiceAccessDetails {
            project_identity: "I1234561234561234561234561234561234561234"
                .try_into()
                .unwrap(),
            project_route: "project_route".to_string(),
            project_authority_identity: "Iabcdefabcdefabcdefabcdefabcdefabcdefabcd"
                .try_into()
                .unwrap(),
            project_authority_route: "project_authority_route".to_string(),
            shared_node_identity: "I12ab34cd56ef12ab34cd56ef12ab34cd56ef12ab"
                .try_into()
                .unwrap(),
            shared_node_route: "shared_node_route".to_string(),
            enrollment_ticket: EnrollmentTicket::new(
                OneTimeCode::new(),
                Some(ProjectLookup {
                    node_route: None,
                    id: "project_identity".to_string(),
                    name: "project_name".to_string(),
                    identity_id: None,
                    authority: None,
                    okta: None,
                }),
                None,
            )
            .hex_encoded()
            .unwrap(),
        });

        // Validate the inlet data, with no prior inlet data
        let inlet_data = InletDataFromInvitation::new(&cli_state, &invitation, &inlets)
            .unwrap()
            .unwrap();
        assert!(inlet_data.socket_addr.is_none());

        // Validate the inlet data, with prior inlet data
        inlets.insert(
            "invitation_id".to_string(),
            Inlet {
                node_name: "local_node_name".to_string(),
                alias: "alias".to_string(),
                socket_addr: "127.0.0.1:1000".parse().unwrap(),
                enabled: true,
            },
        );
        let inlet_data = InletDataFromInvitation::new(&cli_state, &invitation, &inlets)
            .unwrap()
            .unwrap();
        assert!(inlet_data.socket_addr.is_some());
    }
}
