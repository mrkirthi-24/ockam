#[derive(Clone, Debug)]
#[repr(C)]
pub enum OrchestratorStatus {
    Disconnected = 0,
    Connecting,
    Connected,

    WaitingForToken,
    RetrievingSpace,
    RetrievingProject,
}

impl Default for OrchestratorStatus {
    fn default() -> Self {
        OrchestratorStatus::Disconnected
    }
}

pub mod rust {
    pub use crate::api::state::OrchestratorStatus;

    #[derive(Default, Clone, Debug)]
    pub struct Invitee {
        pub name: Option<String>,
        pub email: String,
    }

    #[derive(Default, Clone, Debug)]
    pub struct Invite {
        pub id: String,
        pub service_name: String,
        pub service_scheme: Option<String>,
    }

    #[derive(Default, Clone, Debug)]
    pub struct LocalService {
        pub name: String,
        pub address: String,
        pub port: u16,
        pub scheme: Option<String>,
        pub shared_with: Vec<Invitee>,
        pub available: bool,
    }

    #[derive(Default, Clone, Debug)]
    pub struct Service {
        pub source_name: String,
        pub address: String,
        pub port: u16,
        pub scheme: Option<String>,
        pub available: bool,
    }

    #[derive(Default, Clone, Debug)]
    pub struct ServiceGroup {
        pub email: String,
        pub name: Option<String>,
        pub image_url: Option<String>,

        pub invites: Vec<Invite>,
        pub incoming_services: Vec<Service>,
    }

    #[derive(Default, Clone, Debug)]
    pub struct ApplicationState {
        pub enrolled: bool,
        pub orchestrator_status: OrchestratorStatus,
        pub enrollment_name: Option<String>,
        pub enrollment_email: Option<String>,
        pub enrollment_image: Option<String>,
        pub enrollment_github_user: Option<String>,

        pub local_services: Vec<LocalService>,
        pub groups: Vec<ServiceGroup>,
    }

    #[derive(Clone)]
    pub struct ApplicationStateCallback(
        unsafe extern "C" fn(state: super::c::ApplicationState) -> (),
    );
    impl ApplicationStateCallback {
        pub fn new(
            callback: unsafe extern "C" fn(state: super::c::ApplicationState) -> (),
        ) -> Self {
            Self(callback)
        }
        pub fn call(&self, state: ApplicationState) -> () {
            unsafe {
                (self.0)(super::convert_to_c(state));
            }
        }
    }
}

pub mod c {
    use crate::api::state::OrchestratorStatus;
    use libc::c_char;

    #[repr(C)]
    pub struct Invitee {
        /// Optional
        pub(super) name: *const c_char,
        pub(super) email: *const c_char,
    }

    #[repr(C)]
    pub struct Invite {
        pub(super) id: *const c_char,
        pub(super) service_name: *const c_char,
        /// Optional
        pub(super) service_scheme: *const c_char,
    }

    #[repr(C)]
    pub struct LocalService {
        pub(super) name: *const c_char,
        pub(super) address: *const c_char,
        pub(super) port: u16,
        /// Optional
        pub(super) scheme: *const c_char,
        pub(super) shared_with: *const *const Invitee,
        pub(super) available: u8,
    }

    #[repr(C)]
    pub struct Service {
        pub(super) source_name: *const c_char,
        pub(super) address: *const c_char,
        pub(super) port: u16,
        /// Optional
        pub(super) scheme: *const c_char,
        pub(super) available: u8,
    }

    #[repr(C)]
    pub struct ServiceGroup {
        pub(super) email: *const c_char,
        /// Optional
        pub(super) name: *const c_char,
        /// Optional
        pub(super) image_url: *const c_char,

        pub(super) invites: *const *const Invite,
        pub(super) incoming_services: *const *const Service,
    }

    #[repr(C)]
    pub struct ApplicationState {
        pub(super) enrolled: u8,
        pub(super) orchestrator_status: OrchestratorStatus,
        /// Optional
        pub(super) enrollment_name: *const c_char,
        /// Optional
        pub(super) enrollment_email: *const c_char,
        /// Optional
        pub(super) enrollment_image: *const c_char,
        /// Optional
        pub(super) enrollment_github_user: *const c_char,

        pub(super) local_services: *const *const LocalService,
        pub(super) groups: *const *const ServiceGroup,
    }
}

use crate::api::{append_c_terminator, to_c_string, to_optional_c_string};

fn invitee_to_c(invitee: rust::Invitee) -> *const c::Invitee {
    let invitee_c = c::Invitee {
        name: to_optional_c_string(invitee.name),
        email: to_c_string(invitee.email),
    };
    Box::into_raw(Box::new(invitee_c))
}

fn invite_to_c(invite: rust::Invite) -> *const c::Invite {
    let invite_c = c::Invite {
        id: to_c_string(invite.id),
        service_name: to_c_string(invite.service_name),
        service_scheme: to_optional_c_string(invite.service_scheme),
    };
    Box::into_raw(Box::new(invite_c))
}

fn local_service_to_c(local_service: rust::LocalService) -> *const c::LocalService {
    let local_service_c = c::LocalService {
        name: to_c_string(local_service.name),
        address: to_c_string(local_service.address),
        port: local_service.port,
        scheme: to_optional_c_string(local_service.scheme),
        shared_with: append_c_terminator(
            local_service
                .shared_with
                .into_iter()
                .map(invitee_to_c)
                .collect::<Vec<_>>(),
        ),
        available: local_service.available as u8,
    };
    Box::into_raw(Box::new(local_service_c))
}
fn service_to_c(service: rust::Service) -> *const c::Service {
    let service_c = c::Service {
        source_name: to_c_string(service.source_name),
        address: to_c_string(service.address),
        port: service.port,
        scheme: to_optional_c_string(service.scheme),
        available: service.available as u8,
    };
    Box::into_raw(Box::new(service_c))
}

fn group_to_c(group: rust::ServiceGroup) -> *const c::ServiceGroup {
    let group_c = c::ServiceGroup {
        name: to_optional_c_string(group.name),
        email: to_c_string(group.email),
        image_url: to_optional_c_string(group.image_url),
        invites: append_c_terminator(
            group
                .invites
                .into_iter()
                .map(invite_to_c)
                .collect::<Vec<_>>(),
        ),

        incoming_services: append_c_terminator(
            group
                .incoming_services
                .into_iter()
                .map(service_to_c)
                .collect::<Vec<_>>(),
        ),
    };
    Box::into_raw(Box::new(group_c))
}

/// Convert the instance into c representation.
/// Manual call to [free] must be performed to reclaim memory.
pub(crate) fn convert_to_c(state: rust::ApplicationState) -> c::ApplicationState {
    c::ApplicationState {
        enrolled: state.enrolled as u8,
        orchestrator_status: state.orchestrator_status,
        enrollment_name: to_optional_c_string(state.enrollment_name),
        enrollment_email: to_optional_c_string(state.enrollment_email),
        enrollment_image: to_optional_c_string(state.enrollment_image),
        enrollment_github_user: to_optional_c_string(state.enrollment_github_user),

        local_services: append_c_terminator(
            state
                .local_services
                .into_iter()
                .map(local_service_to_c)
                .collect::<Vec<_>>(),
        ),

        groups: append_c_terminator(state.groups.into_iter().map(group_to_c).collect::<Vec<_>>()),
    }
}

pub(crate) fn free(_instance: &c::ApplicationState) {
    unimplemented!()
}
