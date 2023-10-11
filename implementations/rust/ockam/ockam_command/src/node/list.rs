use clap::Args;
use colorful::Colorful;
use indoc::formatdoc;
use miette::IntoDiagnostic;
use serde::Serialize;

use ockam::Context;

use crate::output::Output;
use crate::terminal::OckamColor;
use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts, Result};

const LONG_ABOUT: &str = include_str!("./static/list/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/list/after_long_help.txt");

/// List nodes
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ListCommand {}

impl ListCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self))
    }
}

async fn run_impl(
    ctx: Context,
    (opts, _cmd): (CommandGlobalOpts, ListCommand),
) -> miette::Result<()> {
    let node_infos = opts.state.get_nodes().await?;

    let mut nodes: Vec<NodeListOutput> = Vec::new();
    for node_info in node_infos {
        nodes.push(NodeListOutput::new(
            node_info.name(),
            node_info.pid(),
            node_info.is_default(),
        ));
    }

    let plain = opts
        .terminal
        .build_list(&nodes, "Nodes", "No nodes found on this system.")?;

    let json = serde_json::to_string_pretty(&nodes).into_diagnostic()?;

    opts.terminal
        .stdout()
        .plain(plain)
        .json(json)
        .write_line()?;

    Ok(())
}

#[derive(Serialize)]
pub struct NodeListOutput {
    pub node_name: String,
    pub pid: Option<u32>,
    pub is_default: bool,
}

impl NodeListOutput {
    pub fn new(node_name: String, pid: Option<u32>, is_default: bool) -> Self {
        Self {
            node_name,
            pid,
            is_default,
        }
    }
}

impl Output for NodeListOutput {
    fn output(&self) -> Result<String> {
        let (status, pid) = match self.pid {
            Some(pid) => (
                "UP".color(OckamColor::Success.color()),
                format!(
                    "Process id {}",
                    pid.to_string().color(OckamColor::PrimaryResource.color())
                ),
            ),
            _ => (
                "DOWN".color(OckamColor::Failure.color()),
                "No process running".to_string(),
            ),
        };
        let default = match self.is_default {
            true => " (default)".to_string(),
            false => "".to_string(),
        };

        let output = formatdoc! {"
        Node {node_name}{default} {status}
        {pid}",
        node_name = self
            .node_name
            .to_string()
            .color(OckamColor::PrimaryResource.color()),
        };

        Ok(output)
    }
}
