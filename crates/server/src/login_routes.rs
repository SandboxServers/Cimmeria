//! The auth routes mounted at startup: the launcher telemetry routes on the
//! public SOAP login port, and the identity routes of `/auth/v1` (contract
//! C4, C12). Both go on the same `AuthService`, in one guard.

use cimmeria_services::orchestrator::Orchestrator;

/// Mounts the public login-port routes and the identity routes on the auth
/// service. A providers file the identity crate refuses stops the server:
/// the process exits with code 1 after logging the reason.
pub(crate) async fn install(orch: &Orchestrator, developer_mode: bool) {
    let state = orch.state();
    let mut guard = state.write().await;
    guard
        .auth
        .set_public_routes(cimmeria_admin_api::login_port_telemetry_router());
    if let Err(e) = cimmeria_identity::wiring::install(&mut guard.auth, developer_mode) {
        tracing::error!("auth provider config refused: {e}");
        std::process::exit(1);
    }
}
