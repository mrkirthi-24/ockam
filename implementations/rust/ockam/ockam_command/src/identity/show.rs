use crate::output::{EncodeFormat, IdentifierDisplay, IdentityDisplay};
use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts};
use clap::Args;
use miette::IntoDiagnostic;
use ockam::identity::{Identity, Vault};
use ockam_api::cli_state::traits::{StateDirTrait, StateItemTrait};
use ockam_node::Context;

const LONG_ABOUT: &str = include_str!("./static/show/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Show the details of an identity
#[derive(Clone, Debug, Args)]
#[command(
    long_about = docs::about(LONG_ABOUT),
    before_help = docs::before_help(PREVIEW_TAG),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ShowCommand {
    #[arg()]
    name: Option<String>,

    /// Show the full identity history, and not just the identifier or the nameq
    #[arg(short, long)]
    full: bool,

    //TODO: see if it make sense to have a --encoding argument shared across commands.
    //      note the only reason this is here right now is that project.json expect the
    //      authority' identity change history to be in hex format.  This only applies
    //      for `full` (change history) identity.
    #[arg(long, value_enum, requires = "full")]
    encoding: Option<EncodeFormat>,
}

impl ShowCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(Self::run_impl, (opts, self))
    }

    async fn run_impl(
        _ctx: Context,
        options: (CommandGlobalOpts, ShowCommand),
    ) -> miette::Result<()> {
        let (opts, cmd) = options;
        let identity = opts
            .state
            .get_identity_by_optional_name(&cmd.name)
            .await
            .into_diagnostic()?;
        if cmd.full {
            if Some(EncodeFormat::Hex) == cmd.encoding {
                opts.println(&hex::encode(
                    identity.change_history().export().into_diagnostic()?,
                ))?;
            } else {
                let identity_display = IdentityDisplay(identity);
                opts.println(&identity_display)?;
            }
        } else {
            let identifier_display = IdentifierDisplay(identity.identifier().clone());
            opts.println(&identifier_display)?;
        }
        Ok(())
    }
}
