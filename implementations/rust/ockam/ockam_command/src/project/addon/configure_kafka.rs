use clap::builder::NonEmptyStringValueParser;
use clap::Args;
use colorful::Colorful;

use ockam::Context;
use ockam_api::cli_state::{StateDirTrait, StateItemTrait};
use ockam_api::cloud::addon::KafkaConfig;
use ockam_api::cloud::operation::CreateOperationResponse;
use ockam_api::cloud::project::Project;
use ockam_api::cloud::CloudRequestWrapper;
use ockam_core::api::Request;

use crate::node::util::delete_embedded_node;
use crate::operation::util::check_for_completion;
use crate::project::addon::configure_addon_endpoint;
use crate::project::util::check_project_readiness;
use crate::util::api::CloudOpts;
use crate::util::{api, node_rpc, Rpc};
use crate::{docs, fmt_ok, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/configure_kafka/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/configure_kafka/after_long_help.txt");

/// Configure the Apache Kafka addon for a project
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct AddonConfigureKafkaSubcommand {
    /// Ockam project name
    #[arg(
        long = "project",
        id = "project",
        value_name = "PROJECT_NAME",
        default_value = "default",
        value_parser(NonEmptyStringValueParser::new())
    )]
    project_name: String,

    /// Bootstrap server address
    #[arg(
        long,
        id = "bootstrap_server",
        value_name = "BOOTSTRAP_SERVER",
        value_parser(NonEmptyStringValueParser::new())
    )]
    bootstrap_server: String,
}

impl AddonConfigureKafkaSubcommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, AddonConfigureKafkaSubcommand),
) -> miette::Result<()> {
    let controller_route = &CloudOpts::route();
    let AddonConfigureKafkaSubcommand {
        project_name,
        bootstrap_server,
    } = cmd;

    let mut rpc = Rpc::embedded(&ctx, &opts).await?;
    let body = KafkaConfig::new(bootstrap_server);
    let addon_id = "confluent";
    let endpoint = format!(
        "{}/{}",
        configure_addon_endpoint(&opts.state, &project_name)?,
        addon_id
    );
    let req = Request::post(endpoint).body(CloudRequestWrapper::new(body, controller_route, None));
    let response: CreateOperationResponse = rpc.ask(req).await?;
    let operation_id = response.operation_id;

    check_for_completion(&ctx, &opts, rpc.node_name(), &operation_id).await?;

    let project_id = opts.state.projects.get(&project_name)?.config().id.clone();
    let mut rpc = rpc.clone();
    let project: Project = rpc
        .ask(api::project::show(&project_id, controller_route))
        .await?;
    check_project_readiness(&ctx, &opts, rpc.node_name(), None, project).await?;

    opts.terminal
        .write_line(&fmt_ok!("Apache Kafka addon configured successfully"))?;

    delete_embedded_node(&opts, rpc.node_name()).await;
    Ok(())
}