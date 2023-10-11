use std::env::current_exe;
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use miette::Context as _;
use miette::{miette, IntoDiagnostic};
use rand::random;

use ockam_core::env::get_env_with_default;

use crate::util::api::TrustContextOpts;
use crate::CommandGlobalOpts;

pub struct NodeManagerDefaults {
    pub node_name: String,
    pub tcp_listener_address: String,
    pub trust_context_opts: TrustContextOpts,
}

impl Default for NodeManagerDefaults {
    fn default() -> Self {
        Self {
            node_name: hex::encode(random::<[u8; 4]>()),
            tcp_listener_address: "127.0.0.1:0".to_string(),
            trust_context_opts: TrustContextOpts::default(),
        }
    }
}

pub async fn delete_node(opts: &CommandGlobalOpts, name: &str, force: bool) -> miette::Result<()> {
    Ok(opts.state.delete_node_sigkill(name, force).await?)
}

pub async fn delete_all_nodes(opts: &CommandGlobalOpts, force: bool) -> miette::Result<()> {
    let node_infos = opts.state.get_nodes().await?;
    let mut deletion_errors = Vec::new();
    for node_info in node_infos {
        if let Err(e) = opts
            .state
            .delete_node_sigkill(&node_info.name(), force)
            .await
        {
            deletion_errors.push((node_info.name(), e));
        }
    }
    if !deletion_errors.is_empty() {
        return Err(miette!(
            "errors while deleting nodes: {:?}",
            deletion_errors
        ));
    }
    Ok(())
}

pub async fn check_default(opts: &CommandGlobalOpts, name: &str) -> miette::Result<bool> {
    Ok(opts.state.get_node(name).await?.is_default())
}

/// A utility function to spawn a new node into foreground mode
#[allow(clippy::too_many_arguments)]
pub async fn spawn_node(
    opts: &CommandGlobalOpts,
    name: &str,
    address: &str,
    project: Option<&PathBuf>,
    trusted_identities: Option<&String>,
    trusted_identities_file: Option<&PathBuf>,
    reload_from_trusted_identities_file: Option<&PathBuf>,
    launch_config: Option<String>,
    authority_identity: Option<&String>,
    credential: Option<&String>,
    trust_context: Option<&PathBuf>,
    project_name: Option<&String>,
    logging_to_file: bool,
) -> miette::Result<()> {
    let mut args = vec![
        match opts.global_args.verbose {
            0 => "-vv".to_string(),
            v => format!("-{}", "v".repeat(v as usize)),
        },
        "node".to_string(),
        "create".to_string(),
        "--tcp-listener-address".to_string(),
        address.to_string(),
        "--foreground".to_string(),
        "--child-process".to_string(),
    ];

    if logging_to_file || !opts.terminal.is_tty() {
        args.push("--no-color".to_string());
    }

    if let Some(path) = project {
        args.push("--project-path".to_string());
        let p = path
            .to_str()
            .unwrap_or_else(|| panic!("unsupported path {path:?}"));
        args.push(p.to_string())
    }

    if let Some(l) = launch_config {
        args.push("--launch-config".to_string());
        args.push(l);
    }

    if let Some(t) = trusted_identities {
        args.push("--trusted-identities".to_string());
        args.push(t.to_string())
    } else if let Some(t) = trusted_identities_file {
        args.push("--trusted-identities-file".to_string());
        args.push(
            t.to_str()
                .unwrap_or_else(|| panic!("unsupported path {t:?}"))
                .to_string(),
        );
    } else if let Some(t) = reload_from_trusted_identities_file {
        args.push("--reload-from-trusted-identities-file".to_string());
        args.push(
            t.to_str()
                .unwrap_or_else(|| panic!("unsupported path {t:?}"))
                .to_string(),
        );
    }

    if let Some(ai) = authority_identity {
        args.push("--authority-identity".to_string());
        args.push(ai.to_string());
    }

    if let Some(credential) = credential {
        args.push("--credential".to_string());
        args.push(credential.to_string());
    }

    if let Some(trust_context) = trust_context {
        args.push("--trust-context".to_string());
        args.push(
            trust_context
                .to_str()
                .unwrap_or_else(|| panic!("unsupported path {trust_context:?}"))
                .to_string(),
        );
    }

    if let Some(project_name) = project_name {
        args.push("--project".to_string());
        args.push(project_name.to_string());
    }

    args.push(name.to_owned());

    run_ockam(opts, name, args, logging_to_file).await
}

/// Run the ockam command line with specific arguments
pub async fn run_ockam(
    opts: &CommandGlobalOpts,
    node_name: &str,
    args: Vec<String>,
    logging_to_file: bool,
) -> miette::Result<()> {
    // On systems with non-obvious path setups (or during
    // development) re-executing the current binary is a more
    // deterministic way of starting a node.
    let ockam_exe = get_env_with_default("OCKAM", current_exe().unwrap_or_else(|_| "ockam".into()))
        .into_diagnostic()?;
    let node_info = opts.state.get_node(node_name).await?;

    let mut cmd = Command::new(ockam_exe);

    if logging_to_file {
        let (mlog, elog) = { (node_info.stdout_log(), node_info.stderr_log()) };
        let main_log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(mlog)
            .into_diagnostic()
            .context("failed to open log path")?;
        let stderr_log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(elog)
            .into_diagnostic()
            .context("failed to open stderr log path")?;
        cmd.stdout(main_log_file).stderr(stderr_log_file);
    }

    cmd.args(args)
        .stdin(Stdio::null())
        .spawn()
        .into_diagnostic()
        .context("failed to spawn node")?;
    Ok(())
}
