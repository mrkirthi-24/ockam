use clap::Args;
use colorful::Colorful;
use miette::miette;

use ockam_node::Context;

use crate::util::node_rpc;
use crate::{docs, fmt_ok, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/default/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/default/after_long_help.txt");

/// Change the default identity
#[derive(Clone, Debug, Args)]
#[command(
arg_required_else_help = true,
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct DefaultCommand {
    /// Name of the identity to be set as default
    name: String,
}

impl DefaultCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self));
    }
}

async fn run_impl(
    _ctx: Context,
    (opts, cmd): (CommandGlobalOpts, DefaultCommand),
) -> miette::Result<()> {
    let is_default = opts.state.is_default_identity_by_name(&cmd.name).await?;
    // If it exists, warn the user and exit
    if is_default {
        Err(miette!(
            "The identity '{}' is already the default",
            &cmd.name
        ))
    }
    // Otherwise, set it as default
    else {
        opts.state.set_as_default_identity(&cmd.name).await?;
        opts.terminal
            .stdout()
            .plain(fmt_ok!("The identity '{}' is now the default", &cmd.name))
            .machine(&cmd.name)
            .write_line()?;
        Ok(())
    }
}
