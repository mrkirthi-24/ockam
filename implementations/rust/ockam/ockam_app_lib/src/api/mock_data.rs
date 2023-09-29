use crate::api::state::{c, convert_to_c, rust, OrchestratorStatus};

/// This function serves to create a mock application state for the UI.
/// The sole purpose is to have a quick preview without requiring an initialized state.
#[no_mangle]
extern "C" fn mock_application_state() -> c::ApplicationState {
    let state = rust::ApplicationState {
        enrolled: true,
        orchestrator_status: OrchestratorStatus::Connected,
        enrollment_name: Some("Davide Baldo".into()),
        enrollment_email: Some("davide@baldo.me".into()),
        enrollment_image: Some("https://avatars.githubusercontent.com/u/408088?v=4".into()),
        enrollment_github_user: Some("davide-baldo".into()),
        local_services: vec![
            rust::LocalService {
                name: "Super Cool Web Demo".into(),
                address: "localhost".into(),
                port: 8080,
                scheme: Some("http".into()),
                shared_with: vec![rust::Invitee {
                    name: Some("Adrian Benavides".into()),
                    email: "adrian@ockam.io".into(),
                }],
                available: true,
            },
            rust::LocalService {
                name: "My Router Admin Page".into(),
                address: "localhost".into(),
                port: 8080,
                scheme: Some("http".into()),
                shared_with: vec![rust::Invitee {
                    name: Some("Adrian Benavides".into()),
                    email: "adrian@ockam.io".into(),
                }],
                available: true,
            },
        ],
        groups: vec![
            rust::ServiceGroup {
                name: Some("Adrian Benavides".into()),
                email: "adrian@ockam.io".into(),
                image_url: Some("https://avatars.githubusercontent.com/u/12375782?v=4".into()),
                invites: vec![rust::Invite {
                    id: "1234".into(),
                    service_name: "Local Web Deployment".into(),
                    service_scheme: Some("http".into()),
                }],
                incoming_services: vec![rust::Service {
                    source_name: "ssh".into(),
                    address: "".into(),
                    port: 22,
                    scheme: Some("ssh".into()),
                    available: false,
                }],
            },
            rust::ServiceGroup {
                name: Some("Eric Torreborre".into()),
                email: "eric.torreborre@ockam.io".into(),
                image_url: Some("https://avatars.githubusercontent.com/u/10988?v=4".into()),
                invites: vec![],
                incoming_services: vec![rust::Service {
                    source_name: "Production Database".into(),
                    address: "localhost".into(),
                    port: 5432,
                    scheme: Some("postgresql".into()),
                    available: true,
                }],
            },
        ],
    };

    convert_to_c(state)
}
