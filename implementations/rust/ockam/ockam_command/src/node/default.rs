use clap::Args;
use colorful::Colorful;
use miette::miette;

use ockam_node::Context;

use crate::node::get_node_name;
use crate::util::node_rpc;
use crate::{docs, fmt_ok, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/default/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/default/after_long_help.txt");

/// Change the default node
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct DefaultCommand {
    /// Name of the node to set as default
    node_name: String,
}

impl DefaultCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self));
    }
}

async fn run_impl(
    _cxt: Context,
    (opts, cmd): (CommandGlobalOpts, DefaultCommand),
) -> miette::Result<()> {
    let name = get_node_name(&opts.state, &Some(cmd.node_name)).await;
    if opts.state.is_default_node(&name).await? {
        Err(miette!("The node '{name}' is already the default"))
    } else {
        opts.state.set_default_node(&name).await?;
        opts.terminal
            .stdout()
            .plain(fmt_ok!("The node '{name}' is now the default"))
            .machine(&name)
            .write_line()?;
        Ok(())
    }
}
