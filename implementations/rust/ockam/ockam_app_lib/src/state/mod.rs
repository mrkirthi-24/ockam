mod model;
mod repository;

use std::collections::HashSet;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread::spawn;
use std::time::Duration;

use miette::IntoDiagnostic;
use ockam_multiaddr::MultiAddr;
use tokio::sync::RwLock;
use tracing::{error, info, trace, warn};

use crate::api::notification::rust::{Notification, NotificationCallback};
use crate::api::state::rust::{ApplicationStateCallback, Invite, Invitee, Service, ServiceGroup};
use crate::background_node::{BackgroundNodeClient, Cli};
use crate::invitations::state::InvitationState;
pub(crate) use crate::state::model::ModelState;
pub(crate) use crate::state::repository::{LmdbModelStateRepository, ModelStateRepository};
use ockam::Context;
use ockam::{NodeBuilder, TcpListenerOptions, TcpTransport};
use ockam_api::cli_state::{
    add_project_info_to_node_state, init_node_state, CliState, StateDirTrait, StateItemTrait,
};
use ockam_api::cloud::enroll::auth0::UserInfo;
use ockam_api::cloud::project::Project;
use ockam_api::cloud::Controller;
use ockam_api::nodes::models::portal::OutletStatus;
use ockam_api::nodes::models::transport::{CreateTransportJson, TransportMode, TransportType};
use ockam_api::nodes::service::{
    NodeManagerGeneralOptions, NodeManagerTransportOptions, NodeManagerTrustOptions,
};
use ockam_api::nodes::{InMemoryNode, NodeManager, NodeManagerWorker, NODEMANAGER_ADDR};
use ockam_api::trust_context::TrustContextConfigBuilder;

use crate::api::state::OrchestratorStatus;
use crate::{api, Result};

pub const NODE_NAME: &str = "ockam_app";
// TODO: static project name of "default" is an unsafe default behavior due to backend uniqueness requirements
pub const PROJECT_NAME: &str = "default";

/// The AppState struct contains all the state managed by `tauri`.
/// It can be retrieved with the `AppHandle<Wry>` parameter and the `AppHandle::state()` method.
///
/// Note that it contains a `NodeManagerWorker`. This makes the desktop app a full-fledged node
/// with its own set of secure channels, outlets, transports etc...
///
/// However there is no associated persistence yet so outlets created with this `NodeManager` will
/// have to be recreated when the application restarts.
#[derive(Clone)]
pub struct AppState {
    context: Arc<Context>,
    state: Arc<RwLock<CliState>>,
    orchestrator_status: Arc<Mutex<OrchestratorStatus>>,
    model_state: Arc<RwLock<ModelState>>,
    model_state_repository: Arc<RwLock<Arc<dyn ModelStateRepository>>>,
    background_node_client: Arc<RwLock<Arc<dyn BackgroundNodeClient>>>,
    projects: Arc<RwLock<Vec<Project>>>,
    invitations: Arc<RwLock<InvitationState>>,
    application_state_callback: ApplicationStateCallback,
    notification_callback: NotificationCallback,
    node_manager: Arc<RwLock<Arc<InMemoryNode>>>,
}

impl AppState {
    /// Create a new AppState
    pub fn new(
        application_state_callback: ApplicationStateCallback,
        notification_callback: NotificationCallback,
    ) -> AppState {
        let cli_state =
            CliState::initialize().expect("Failed to load the local Ockam configuration");
        let (context, mut executor) = NodeBuilder::new().no_logging().build();
        let context = Arc::new(context);
        let runtime = context.runtime().clone();

        let future = {
            let runtime = runtime.clone();
            async move {
                // start the router, it is needed for the node manager creation
                runtime.spawn(async move { executor.start_router().await });

                // create the application state and its dependencies
                let node_manager = Arc::new(create_node_manager(context.clone(), &cli_state).await);
                let model_state_repository = create_model_state_repository(&cli_state).await;

                info!("AppState initialized");
                let state = AppState {
                    context,
                    application_state_callback,
                    notification_callback,
                    state: Arc::new(RwLock::new(cli_state)),
                    orchestrator_status: Arc::new(Mutex::new(Default::default())),
                    node_manager: Arc::new(RwLock::new(node_manager)),
                    model_state: Arc::new(RwLock::new(ModelState::default())),
                    model_state_repository: Arc::new(RwLock::new(model_state_repository)),
                    // event_manager: std::sync::RwLock::new(EventManager::new()),
                    background_node_client: Arc::new(RwLock::new(Arc::new(Cli::new()))),
                    projects: Arc::new(Default::default()),
                    invitations: Arc::new(RwLock::new(InvitationState::default())),
                };

                state
            }
        };

        runtime.block_on(future)
    }

    /// Load a previously persisted ModelState and start refreshing schedule
    pub async fn load_model_state(&'static self) -> ModelState {
        if self.is_enrolled().await.unwrap_or(false) {
            // no point in trying to connect without being enrolled
            self.load_relay_model_state().await;
        }
        let cli_state = self.state().await;

        match self.model_state_repository.read().await.load().await {
            Ok(model_state) => {
                let model_state = model_state.unwrap_or(ModelState::default());
                self.load_outlet_model_state(&model_state, &cli_state).await;
                self.publish_state().await;

                self.context.runtime().spawn(async move {
                    loop {
                        let result = self.refresh_projects().await;
                        if let Err(e) = result {
                            warn!(%e, "Failed to refresh projects");
                        }
                        tokio::time::sleep(Duration::from_secs(30)).await;
                    }
                });

                self.context.runtime().spawn(async move {
                    loop {
                        let result = self.refresh_invitations().await;
                        if let Err(e) = result {
                            warn!(%e, "Failed to refresh invitations");
                        }
                        tokio::time::sleep(Duration::from_secs(30)).await;
                    }
                });

                self.context.runtime().spawn(async move {
                    loop {
                        let result = self.refresh_inlets().await;
                        if let Err(e) = result {
                            warn!(%e, "Failed to refresh inlets");
                        }
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                });

                model_state
            }
            Err(e) => {
                error!(?e, "Cannot load the model state");
                panic!("Cannot load the model state: {e:?}")
            }
        }
    }

    pub fn shutdown(&self) -> () {
        let mut context = self.context();
        self.context.runtime().spawn(async move {
            let result = context.stop().await;
            match result {
                Ok(_) => info!("Application shutdown"),
                Err(e) => error!(?e, "Failed to shutdown the application"),
            }
        });
    }

    pub async fn reset(&self) -> miette::Result<()> {
        self.reset_state().await?;
        self.reset_node_manager().await?;

        // {
        //     let mut writer = self.event_manager.write().unwrap();
        //     writer.events.clear();
        // }

        // recreate the model state repository since the cli state has changed
        {
            let mut writer = self.model_state.write().await;
            *writer = ModelState::default();
        }
        let identity_path = self
            .state()
            .await
            .identities
            .identities_repository_path()
            .expect("Failed to get the identities repository path");
        let new_state_repository = LmdbModelStateRepository::new(identity_path).await?;
        {
            let mut writer = self.model_state_repository.write().await;
            *writer = Arc::new(new_state_repository);
        }

        Ok(())
    }

    async fn reset_state(&self) -> miette::Result<()> {
        let mut state = self.state.write().await;
        match state.reset().await {
            Ok(s) => {
                *state = s;
                info!("reset the cli state");
            }
            Err(e) => error!("Failed to reset the state {e:?}"),
        }
        Ok(())
    }

    /// Recreate a new NodeManagerWorker instance, which will restart the necessary
    /// child workers as described in its Worker trait implementation.
    pub async fn reset_node_manager(&self) -> miette::Result<()> {
        let mut node_manager = self.node_manager.write().await;
        let _ = node_manager.stop(&self.context).await;
        info!("stopped the old node manager");

        for w in self.context.list_workers().await.into_diagnostic()? {
            let _ = self.context.stop_worker(w.address()).await;
        }
        info!("stopped all the ctx workers");

        let new_node_manager = make_node_manager(self.context.clone(), &self.state().await).await?;
        *node_manager = Arc::new(new_node_manager);
        info!("set a new node manager");
        Ok(())
    }

    /// Return the application Context
    /// This can be used to run async actions involving the Router
    pub fn context(&self) -> Arc<Context> {
        self.context.clone()
    }

    /// Returns the list of projects
    pub fn projects(&self) -> Arc<RwLock<Vec<Project>>> {
        self.projects.clone()
    }

    /// Returns the status of invitations
    pub fn invitations(&self) -> Arc<RwLock<InvitationState>> {
        self.invitations.clone()
    }

    /// Returns the address being used to contact Orchestrator
    pub fn controller_address(&self) -> MultiAddr {
        NodeManager::controller_multiaddr()
    }

    /// Return the application cli state
    /// This can be used to manage the on-disk state for projects, identities, vaults, etc...
    pub async fn state(&self) -> CliState {
        let state = self.state.read().await;
        state.clone()
    }

    /// Return the node manager
    pub async fn node_manager(&self) -> Arc<InMemoryNode> {
        let node_manager = self.node_manager.read().await;
        node_manager.clone()
    }

    /// Return a client to access the Controller
    pub async fn controller(&self) -> Result<Controller> {
        let node_manager = self.node_manager.read().await;
        Ok(node_manager.controller().await?)
    }

    pub async fn is_enrolled(&self) -> Result<bool> {
        self.state().await.is_enrolled().map_err(|e| {
            warn!(%e, "Failed to check if user is enrolled");
            e.into()
        })
    }

    /// Return the list of currently running outlets
    pub async fn tcp_outlet_list(&self) -> Vec<OutletStatus> {
        let node_manager = self.node_manager.read().await;
        node_manager.list_outlets().await.list
    }

    pub async fn user_info(&self) -> Result<UserInfo> {
        Ok(self
            .state
            .read()
            .await
            .users_info
            .default()?
            .config()
            .clone())
    }

    pub async fn user_email(&self) -> Result<String> {
        self.user_info().await.map(|u| u.email)
    }

    pub async fn model_mut(&self, f: impl FnOnce(&mut ModelState)) -> Result<()> {
        let mut model_state = self.model_state.write().await;
        f(&mut model_state);
        self.model_state_repository
            .read()
            .await
            .store(&model_state)
            .await?;
        Ok(())
    }

    pub async fn model<T>(&self, f: impl FnOnce(&ModelState) -> T) -> T {
        let mut model_state = self.model_state.read().await;
        f(&mut model_state)
    }

    pub async fn background_node_client(&self) -> Arc<dyn BackgroundNodeClient> {
        self.background_node_client.read().await.clone()
    }
    pub fn orchestrator_status(&self) -> OrchestratorStatus {
        self.orchestrator_status.lock().unwrap().clone()
    }

    pub fn update_orchestrator_status(&self, status: OrchestratorStatus) {
        *self.orchestrator_status.lock().unwrap() = status;
    }
    pub fn notify(&self, notification: Notification) -> () {
        self.notification_callback.call(notification);
    }

    /// Sends the new application state to the UI
    pub async fn publish_state(&self) {
        let result = self.snapshot().await;
        if let Ok(app_state) = result {
            self.application_state_callback.call(app_state);
        } else {
            warn!("Failed to publish the application state");
        }
    }

    /// Creates a snapshot of the application state without any side-effects
    pub async fn snapshot(&self) -> Result<api::state::rust::ApplicationState> {
        let enrolled = self.is_enrolled().await.unwrap_or(false);
        let orchestrator_status = self.orchestrator_status();
        let enrollment_name;
        let enrollment_email;
        let enrollment_image;
        let enrollment_github_user;
        let local_services: Vec<api::state::rust::LocalService>;
        let groups: Vec<ServiceGroup>;
        let invitations = self.invitations().read().await.clone();

        if enrolled {
            local_services = self
                .tcp_outlet_list()
                .await
                .into_iter()
                .map(|outlet| api::state::rust::LocalService {
                    name: outlet.worker_addr.address().to_string(),
                    address: outlet.socket_addr.ip().to_string(),
                    port: outlet.socket_addr.port(),
                    scheme: None,
                    shared_with: invitations
                        .sent
                        .iter()
                        .filter(|invitation| {
                            invitation.target_id == outlet.worker_addr.address().to_string()
                        })
                        .map(|invitation| Invitee {
                            name: Some(invitation.recipient_email.clone()),
                            email: invitation.recipient_email.clone(),
                        })
                        .collect(),
                    available: true,
                })
                .collect();

            let mut group_names = HashSet::new();

            invitations
                .accepted
                .invitations
                .iter()
                .for_each(|invitation| {
                    group_names.insert(invitation.invitation.owner_email.clone());
                });

            invitations
                .received
                .invitations
                .iter()
                .for_each(|invitation| {
                    group_names.insert(invitation.owner_email.clone());
                });

            groups = group_names
                .into_iter()
                .map(|email| ServiceGroup {
                    email: email.clone(),
                    name: None,
                    image_url: None,
                    invites: invitations
                        .received
                        .invitations
                        .iter()
                        .filter(|invitation| invitation.owner_email == email)
                        .map(|invitation| Invite {
                            id: invitation.id.clone(),
                            service_name: invitation.target_id.clone(),
                            service_scheme: None,
                        })
                        .collect(),
                    incoming_services: invitations
                        .accepted
                        .invitations
                        .iter()
                        .filter(|invitation| {
                            invitation.invitation.owner_email == email
                                && invitation.service_access_details.is_some()
                                && invitations
                                    .accepted
                                    .inlets
                                    .get(&invitation.invitation.id)
                                    .is_some()
                        })
                        .map(|invitation| {
                            let access_details =
                                invitation.service_access_details.as_ref().unwrap();
                            let inlet = invitations
                                .accepted
                                .inlets
                                .get(&invitation.invitation.id)
                                .unwrap();

                            Service {
                                source_name: access_details
                                    .service_name()
                                    .unwrap_or("unknown".to_string()),
                                address: inlet.socket_addr.ip().to_string(),
                                port: inlet.socket_addr.port(),
                                scheme: None,
                                available: inlet.enabled,
                            }
                        })
                        .collect(),
                })
                .collect();

            let user_info = self.user_info().await?;
            enrollment_name = Some(user_info.name);
            enrollment_email = Some(user_info.email);
            enrollment_image = Some(user_info.picture);
            enrollment_github_user = Some(user_info.nickname);
        } else {
            enrollment_name = None;
            enrollment_email = None;
            enrollment_image = None;
            enrollment_github_user = None;
            local_services = vec![];
            groups = vec![];
        }

        Ok(api::state::rust::ApplicationState {
            enrolled,
            orchestrator_status,
            enrollment_name,
            enrollment_email,
            enrollment_image,
            enrollment_github_user,
            local_services,
            groups,
        })
    }
}

async fn create_node_manager(ctx: Arc<Context>, cli_state: &CliState) -> InMemoryNode {
    match make_node_manager(ctx.clone(), cli_state).await {
        Ok(w) => w,
        Err(e) => {
            error!(%e, "Cannot load the model state");
            panic!("Cannot load the model state: {e:?}")
        }
    }
}

/// Make a node manager with a default node called "default"
pub(crate) async fn make_node_manager(
    ctx: Arc<Context>,
    cli_state: &CliState,
) -> miette::Result<InMemoryNode> {
    init_node_state(cli_state, NODE_NAME, None, None).await?;

    let tcp = TcpTransport::create(&ctx).await.into_diagnostic()?;
    let options = TcpListenerOptions::new();
    let listener = tcp
        .listen(&"127.0.0.1:0", options)
        .await
        .into_diagnostic()?;

    add_project_info_to_node_state(NODE_NAME, cli_state, None).await?;

    let node_state = cli_state.nodes.get(NODE_NAME)?;
    node_state.set_setup(
        &node_state.config().setup_mut().set_api_transport(
            CreateTransportJson::new(
                TransportType::Tcp,
                TransportMode::Listen,
                &listener.socket_address().to_string(),
            )
            .into_diagnostic()?,
        ),
    )?;
    let trust_context_config = TrustContextConfigBuilder::new(cli_state).build();

    let node_manager = InMemoryNode::new(
        &ctx,
        NodeManagerGeneralOptions::new(cli_state.clone(), NODE_NAME.to_string(), None, true),
        NodeManagerTransportOptions::new(listener.flow_control_id().clone(), tcp),
        NodeManagerTrustOptions::new(trust_context_config),
    )
    .await
    .into_diagnostic()?;

    ctx.flow_controls()
        .add_consumer(NODEMANAGER_ADDR, listener.flow_control_id());
    Ok(node_manager)
}

/// Create the repository containing the model state
async fn create_model_state_repository(state: &CliState) -> Arc<dyn ModelStateRepository> {
    let identity_path = state
        .identities
        .identities_repository_path()
        .expect("Failed to get the identities repository path");

    match LmdbModelStateRepository::new(identity_path).await {
        Ok(model_state_repository) => Arc::new(model_state_repository),
        Err(e) => {
            error!(%e, "Cannot create a model state repository manager");
            panic!("Cannot create a model state repository manager: {e:?}");
        }
    }
}

pub type EventName = String;
type IsProcessing = AtomicBool;
struct Event {
    name: EventName,
    is_processing: IsProcessing,
}

struct EventManager {
    events: Vec<Event>,
}

impl EventManager {
    fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Add a new event if it doesn't exist
    fn add(&mut self, event_name: &str) {
        if self.events.iter().any(|e| e.name == event_name) {
            return;
        }
        let event = Event {
            name: event_name.to_string(),
            is_processing: AtomicBool::new(true),
        };
        self.events.push(event);
        trace!(%event_name, "New event registered");
    }

    /// Check if it's being processed
    fn is_processing(&mut self, event_name: &str) -> bool {
        match self.events.iter().find(|e| e.name == event_name) {
            Some(e) => {
                let is_processing = e.is_processing.load(Ordering::SeqCst);
                if !is_processing {
                    e.is_processing.store(true, Ordering::SeqCst);
                }
                trace!(%event_name, is_processing, "Event status");
                is_processing
            }
            None => {
                self.add(event_name);
                false
            }
        }
    }

    /// Reset an event after it's been dropped
    fn reset(&self, event_name: &str, processed: bool) {
        if let Some(e) = self.events.iter().find(|e| e.name == event_name) {
            if processed {
                trace!(%event_name, "Event reset");
                e.is_processing.store(false, Ordering::SeqCst);
            }
        }
    }
}
//
// pub struct EventDebouncer<R: Runtime> {
//     app: AppHandle<R>,
//     event_name: EventName,
//     is_processing: bool,
// }
//
// impl<R: Runtime> EventDebouncer<R> {
//     pub fn is_processing(&self) -> bool {
//         self.is_processing
//     }
// }
//
// impl<R: Runtime> Drop for EventDebouncer<R> {
//     fn drop(&mut self) {
//         let state = self.app.state::<AppState>();
//         let reader = state.event_manager.read().unwrap();
//         reader.reset(&self.event_name, !self.is_processing);
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_manager() {
        let mut event_manager = EventManager::new();
        let event_name = "e1";

        // The first call using an unregistered event will register it and return false
        assert!(!event_manager.is_processing(event_name));

        // The second call will return true, as the event has not been marked as processed
        assert!(event_manager.is_processing(event_name));

        // Resetting the event marking it as unprocessed will leave the event as processing
        event_manager.reset(event_name, false);
        assert!(event_manager.is_processing(event_name));

        // Resetting the event marking it as processed will leave the event as processed
        event_manager.reset(event_name, true);

        // The event is now ready to get processed again
        assert!(!event_manager.is_processing(event_name));
    }
}
