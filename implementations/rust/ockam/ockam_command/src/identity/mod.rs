use clap::{Args, Subcommand};
use colorful::Colorful;

pub use create::CreateCommand;
pub(crate) use delete::DeleteCommand;
pub(crate) use list::ListCommand;
use ockam_api::cli_state::CliState;
pub(crate) use show::ShowCommand;

use crate::identity::default::DefaultCommand;
use crate::{docs, CommandGlobalOpts};

mod create;
mod default;
mod delete;
mod list;
mod show;

const LONG_ABOUT: &str = include_str!("./static/long_about.txt");

/// Manage identities
#[derive(Clone, Debug, Args)]
#[command(
arg_required_else_help = true,
subcommand_required = true,
long_about = docs::about(LONG_ABOUT),
)]
pub struct IdentityCommand {
    #[command(subcommand)]
    subcommand: IdentitySubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum IdentitySubcommand {
    Create(CreateCommand),
    Show(ShowCommand),
    List(ListCommand),
    Default(DefaultCommand),
    Delete(DeleteCommand),
}

impl IdentityCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            IdentitySubcommand::Create(c) => c.run(options),
            IdentitySubcommand::Show(c) => c.run(options),
            IdentitySubcommand::List(c) => c.run(options),
            IdentitySubcommand::Delete(c) => c.run(options),
            IdentitySubcommand::Default(c) => c.run(options),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::GlobalArgs;

    use super::*;

    #[test]
    fn test_initialize() {
        let state = CliState::test().unwrap();
        let opts = CommandGlobalOpts::new_for_test(GlobalArgs::default(), state);

        assert!(false, "todo !!!");
        // // on start-up there is no default identity
        // assert!(opts.state.identities.default().is_err());
        //
        // // if no name is given then the default identity is initialized
        // initialize_identity_if_default(&opts, &None);
        // assert!(opts.state.identities.default().is_ok());
        //
        // // if "default" is given as a name the default identity is initialized
        // opts.state.identities.default().unwrap().delete().unwrap();
        // initialize_identity_if_default(&opts, &Some("default".into()));
        // assert!(opts.state.identities.default().is_ok());
        //
        // // if the name of another identity is given then the default identity is not initialized
        // opts.state.identities.default().unwrap().delete().unwrap();
        // initialize_identity_if_default(&opts, &Some("other".into()));
        // assert!(opts.state.identities.default().is_err());
    }
}
