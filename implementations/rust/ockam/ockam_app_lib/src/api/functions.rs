use crate::api::state::OrchestratorStatus;
use crate::state::AppState;
use tracing::error;

/// Global application state.
static mut APPLICATION_STATE: Option<AppState> = None;

const ERROR_NOT_INITIALIZED: &str =
    "initialize_application must be called before any other function";

/// This functions initializes the application state.
/// It must be called before any other function.
#[no_mangle]
extern "C" fn initialize_application(
    // we can't use any type alias because cbindgen doesn't support them
    application_state_callback: unsafe extern "C" fn(
        state: super::state::c::ApplicationState,
    ) -> (),
    notification_callback: unsafe extern "C" fn(
        notification: super::notification::c::Notification,
    ) -> (),
) -> () {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_ansi(false)
        .init();

    let app_state = AppState::new(
        super::state::rust::ApplicationStateCallback::new(application_state_callback),
        super::notification::rust::NotificationCallback::new(notification_callback),
    );
    unsafe {
        APPLICATION_STATE.replace(app_state);
    }

    // avoid waiting for the load to return for a quicker initialization
    let app_state = unsafe { APPLICATION_STATE.as_ref().expect(ERROR_NOT_INITIALIZED) };
    app_state.context().runtime().spawn(async {
        app_state.publish_state().await;
        app_state.load_model_state().await;
    });
}

/// Initiate and wait for graceful shutdown of the application.
#[no_mangle]
extern "C" fn shutdown_application() {
    let app_state = unsafe { APPLICATION_STATE.as_ref() };
    if let Some(app_state) = app_state {
        app_state.shutdown();
    }
}

/// Starts user enrollment
#[no_mangle]
extern "C" fn enroll_user() {
    let app_state = unsafe { APPLICATION_STATE.as_ref().expect(ERROR_NOT_INITIALIZED) };

    let _ = app_state
        .context()
        .runtime()
        .spawn(async move { app_state.enroll_user().await });
}

/// This function retrieve the current version of the application state, for polling purposes.
#[no_mangle]
extern "C" fn application_state_snapshot() -> super::state::c::ApplicationState {
    let app_state = unsafe { APPLICATION_STATE.as_ref().expect(ERROR_NOT_INITIALIZED) };

    let result = app_state
        .context()
        .runtime()
        .block_on(async { app_state.snapshot().await });

    let public_rust_state = match result {
        Ok(state) => state,
        Err(err) => {
            error!("Error refreshing application state: {}", err);
            empty_state()
        }
    };

    super::state::convert_to_c(public_rust_state)
}

fn empty_state() -> super::state::rust::ApplicationState {
    super::state::rust::ApplicationState {
        enrolled: false,
        orchestrator_status: OrchestratorStatus::Disconnected,
        enrollment_name: None,
        enrollment_email: None,
        enrollment_image: None,
        enrollment_github_user: None,
        local_services: vec![],
        groups: vec![],
    }
}
