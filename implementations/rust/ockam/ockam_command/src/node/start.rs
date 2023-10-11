use clap::Args;
use colorful::Colorful;

use ockam_api::nodes::BackgroundNode;
use ockam_node::Context;

use crate::node::show::print_query_status;
use crate::node::util::{check_default, spawn_node};
use crate::node::{get_node_name, initialize_node_if_default};
use crate::util::node_rpc;
use crate::{docs, fmt_err, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/start/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/start/after_long_help.txt");

/// Start a node that was previously stopped
#[derive(Clone, Debug, Args)]
#[command(
arg_required_else_help = true,
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct StartCommand {
    /// Name of the node to be started
    node_name: Option<String>,

    #[arg(long, default_value = "false")]
    aws_kms: bool,
}

impl StartCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_node_if_default(&opts, &self.node_name);
        node_rpc(run_impl, (opts, self))
    }
}

async fn run_impl(
    ctx: Context,
    (mut opts, cmd): (CommandGlobalOpts, StartCommand),
) -> miette::Result<()> {
    let node_name = get_node_name(&opts.state, &cmd.node_name).await;

    let node_info = opts.state.get_node(&node_name).await?;
    // Abort if node is already running
    if node_info.is_running() {
        let n = node_info.name();
        opts.terminal
            .stdout()
            .plain(fmt_err!(
                "The node '{n}' is already running. If you want to restart it you can call `ockam node stop {n}` and then `ockam node start {n}`"
            ))
            .write_line()?;
        return Ok(());
    }
    opts.state.kill_node(&node_name, false).await?;
    let node_address = node_info
        .api_transport_address()
        .map(|a| a.to_string())
        .unwrap_or("no transport address".to_string());
    opts.global_args.verbose = node_info.verbosity();

    // Restart node
    spawn_node(
        &opts,
        &node_name,    // The selected node name
        &node_address, // The selected node api address
        None,          // No project information available
        None,          // No trusted identities
        None,          // "
        None,          // "
        None,          // Launch config
        None,          // Authority Identity
        None,          // Credential
        None,          // Trust Context
        None,          // Project Name
        true,          // Restarted nodes will log to files
    )
    .await?;

    // Print node status
    let mut node = BackgroundNode::create(&ctx, &opts.state, &node_name).await?;
    let is_default = check_default(&opts, &node_name).await?;
    print_query_status(&opts, &ctx, &node_name, &mut node, true, is_default).await?;

    Ok(())
}
