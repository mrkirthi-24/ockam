use std::path::PathBuf;

use ockam::identity::{Identifier, Vault};
use ockam_multiaddr::MultiAddr;

use crate::cli_state::{CliState, CliStateError};
use crate::cli_state::{ProjectConfig, Result};
use crate::config::lookup::{InternetAddress, ProjectLookup};
use crate::nodes::models::transport::{CreateTransportJson, TransportMode, TransportType};

impl CliState {
    pub async fn get_nodes(&self) -> Result<Vec<NodeInfo>> {
        todo!("implement get_node_identifier")
    }

    pub async fn get_node_vault(&self, node_name: &str) -> Result<Vault> {
        todo!("get_node_vault")
    }

    pub async fn get_node_identifier(&self, node_name: &str) -> Result<Identifier> {
        todo!("implement get_node_identifier")
    }

    pub async fn get_node_identifier_name(&self, node_name: &str) -> Result<Option<String>> {
        todo!("implement get_node_identifier_name")
    }

    pub async fn create_node(&self, node_name: &str) -> Result<NodeInfo> {
        todo!("create_node")
    }

    pub async fn get_node(&self, node_name: &str) -> Result<NodeInfo> {
        todo!("get_node_by_name")
    }

    pub async fn is_node_running(&self, node_name: &str) -> Result<bool> {
        todo!("is_node_running")
    }

    pub fn stdout_logs(&self, node_name: &str) -> PathBuf {
        todo!("stdout_logs")
    }

    pub async fn get_node_project(&self, node_name: &str) -> Result<Option<ProjectConfig>> {
        todo!("get_node_project")
    }

    pub async fn kill_node(&self, node_name: &str, force: bool) -> Result<()> {
        todo!("kill_node")
    }

    pub async fn delete_node_sigkill(&self, node_name: &str, force: bool) -> Result<()> {
        todo!("delete_sigkill")
    }

    pub async fn delete_node(&self, node_name: &str) -> Result<()> {
        todo!("get_node_by_name")
    }

    pub async fn delete_default_node(&self) -> Result<()> {
        todo!("get_node_by_name")
    }

    pub async fn get_default_node(&self) -> Result<NodeInfo> {
        todo!("get_default_node")
    }

    pub async fn is_default_node(&self, name: &str) -> Result<bool> {
        todo!("is_default_node")
    }

    pub async fn set_default_node(&self, name: &str) -> Result<bool> {
        todo!("set_default_node")
    }

    pub async fn set_node_transport(
        &self,
        node_name: &str,
        transport_type: TransportType,
        transport_mode: TransportMode,
        address: String,
    ) -> Result<()> {
        todo!("set_node_transport")
    }

    pub async fn set_node_pid(&self, node_name: &str, pid: u32) -> Result<()> {
        todo!("set_node_pid")
    }

    pub async fn is_node_api_transport_set(&self, node_name: &str) -> Result<bool> {
        todo!("is_node_api_transport_set")
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct NodeInfo {
    name: String,
    identifier: Identifier,
    verbosity: u8,
    is_authority_node: bool,
    project: Option<ProjectLookup>,
    api_transport: Option<CreateTransportJson>,
    default_vault_name: String,
    pid: Option<u32>,
    is_default: bool,
    stdout_log: PathBuf,
    stderr_log: PathBuf,
}

impl NodeInfo {
    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn identifier(&self) -> Identifier {
        self.identifier.clone()
    }

    pub fn is_default(&self) -> bool {
        self.is_default
    }

    pub fn pid(&self) -> Option<u32> {
        self.pid.clone()
    }

    pub fn is_running(&self) -> bool {
        self.pid.is_some()
    }

    pub fn verbosity(&self) -> u8 {
        self.verbosity
    }

    pub fn api_transport_port(&self) -> Option<u16> {
        self.api_transport.as_ref().map(|t| t.addr.port())
    }

    pub fn api_transport_address(&self) -> Option<InternetAddress> {
        self.api_transport.as_ref().map(|t| t.addr.clone())
    }

    pub fn api_transport_multiaddr(&self) -> Result<MultiAddr> {
        self.api_transport
            .as_ref()
            .ok_or(CliStateError::InvalidData(
                "no transport has been set on the node".to_string(),
            ))
            .and_then(|t| t.maddr().map_err(|e| CliStateError::Ockam(e)))
    }

    pub fn is_authority_node(&self) -> bool {
        self.is_authority_node
    }

    pub fn stdout_log(&self) -> PathBuf {
        self.stdout_log.clone()
    }

    pub fn stderr_log(&self) -> PathBuf {
        self.stderr_log.clone()
    }
}

// use super::Result;
// use crate::cli_state::{
//     CliState, CliStateError, ProjectConfig, ProjectConfigCompact, StateDirTrait, StateItemTrait,
//     VaultState,
// };
// use crate::config::lookup::ProjectLookup;
// use crate::nodes::models::transport::CreateTransportJson;
// use miette::{IntoDiagnostic, WrapErr};
// use nix::errno::Errno;
// use ockam::identity::Identifier;
// use ockam::identity::Vault;
// use ockam_core::compat::collections::HashSet;
// use serde::{Deserialize, Serialize};
// use std::fmt::{Display, Formatter};
// use std::path::{Path, PathBuf};
// use std::str::FromStr;
// use sysinfo::{Pid, ProcessExt, ProcessStatus, System, SystemExt};
//
// #[derive(Debug, Clone, Eq, PartialEq)]
// pub struct NodesState {
//     dir: PathBuf,
// }
//
// impl NodesState {
//     pub fn stdout_logs(&self, name: &str) -> Result<PathBuf> {
//         let dir = self.path(name);
//         std::fs::create_dir_all(&dir)?;
//         Ok(NodePaths::new(&dir).stdout())
//     }
//
//     pub fn delete_sigkill(&self, name: &str, sigkill: bool) -> Result<()> {
//         self._delete(name, sigkill)
//     }
//
//     fn _delete(&self, name: impl AsRef<str>, sigkill: bool) -> Result<()> {
//         // If doesn't exist do nothing
//         if !self.exists(&name) {
//             return Ok(());
//         }
//         let node = self.get(&name)?;
//         // Set default to another node if it's the default
//         if self.is_default(&name)? {
//             // Remove link if it exists
//             let _ = std::fs::remove_file(self.default_path()?);
//             for node in self.list()? {
//                 if node.name() != name.as_ref() && self.set_default(node.name()).is_ok() {
//                     debug!(name=%node.name(), "set default node");
//                     break;
//                 }
//             }
//         }
//         // Remove node directory
//         node.delete_sigkill(sigkill)?;
//         Ok(())
//     }
// }
//
// #[derive(Debug, Clone, Eq, PartialEq)]
// pub struct NodeState {
//     name: String,
//     path: PathBuf,
//     paths: NodePaths,
// }
//
// impl NodeState {
//     fn _delete(&self, sikgill: bool) -> Result<()> {
//         //self.kill_process(sikgill)?;
//         std::fs::remove_dir_all(&self.path)?;
//         let _ = std::fs::remove_dir(&self.path); // Make sure the dir is gone
//         info!(name=%self.name, "node deleted");
//         Ok(())
//     }
//
//     pub fn delete_sigkill(&self, sigkill: bool) -> Result<()> {
//         self._delete(sigkill)
//     }
//
//     // pub fn kill_process(&self, sigkill: bool) -> Result<()> {
//     //     if let Some(pid) = self.pid()? {
//     //         nix::sys::signal::kill(
//     //             nix::unistd::Pid::from_raw(pid),
//     //             if sigkill {
//     //                 nix::sys::signal::Signal::SIGKILL
//     //             } else {
//     //                 nix::sys::signal::Signal::SIGTERM
//     //             },
//     //         )
//     //         .or_else(|e| {
//     //             if e == Errno::ESRCH {
//     //                 tracing::warn!(node = %self.name(), %pid, "No such process");
//     //                 Ok(())
//     //             } else {
//     //                 Err(e)
//     //             }
//     //         })
//     //         .map_err(|e| {
//     //             CliStateError::Io(std::io::Error::new(
//     //                 std::io::ErrorKind::Other,
//     //                 format!("failed to stop PID `{pid}` with error `{e}`"),
//     //             ))
//     //         })?;
//     //         std::fs::remove_file(self.paths.pid())?;
//     //     }
//     //     info!(name = %self.name(), "node process killed");
//     //     Ok(())
//     // }
//
//     // fn pid(&self) -> Result<Option<i32>> {
//     //     let path = self.paths.pid();
//     //     if path.exists() {
//     //         let pid = std::fs::read_to_string(path)?
//     //             .parse::<i32>()
//     //             .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
//     //         Ok(Some(pid))
//     //     } else {
//     //         Ok(None)
//     //     }
//     // }
//
//     pub fn is_running(&self) -> bool {
//         if let Ok(Some(pid)) = self.pid() {
//             let mut sys = System::new();
//             sys.refresh_processes();
//             if let Some(p) = sys.process(Pid::from(pid as usize)) {
//                 // Under certain circumstances the process can be in a state where it's not running
//                 // and we are unable to kill it. For example, `kill -9` a process created by
//                 // `node create` in a Docker environment will result in a zombie process.
//                 !matches!(p.status(), ProcessStatus::Dead | ProcessStatus::Zombie)
//             } else {
//                 false
//             }
//         } else {
//             false
//         }
//     }
//
//     pub fn stdout_log(&self) -> PathBuf {
//         self.paths.stdout()
//     }
//
//     pub fn stderr_log(&self) -> PathBuf {
//         self.paths.stderr()
//     }
//
//     pub fn name(&self) -> &str {
//         &self.name
//     }
// }
//
// #[derive(Debug, Clone, Eq, PartialEq)]
// struct NodePaths {
//     path: PathBuf,
// }
//
// impl NodePaths {
//     fn new(path: &Path) -> Self {
//         Self {
//             path: path.to_path_buf(),
//         }
//     }
//
//     fn stdout(&self) -> PathBuf {
//         self.path.join("stdout.log")
//     }
//
//     fn stderr(&self) -> PathBuf {
//         self.path.join("stderr.log")
//     }
// }
//
// mod traits {
//     use super::*;
//     use crate::cli_state::file_stem;
//     use crate::cli_state::traits::*;
//     use crate::nodes::models::transport::{TransportMode, TransportType};
//     use ockam_core::async_trait;
//
//     #[async_trait]
//     impl StateDirTrait for NodesState {
//         type Item = NodeState;
//         const DEFAULT_FILENAME: &'static str = "node";
//         const DIR_NAME: &'static str = "nodes";
//         const HAS_DATA_DIR: bool = false;
//
//         fn new(root_path: &Path) -> Self {
//             Self {
//                 dir: Self::build_dir(root_path),
//             }
//         }
//
//         fn dir(&self) -> &PathBuf {
//             &self.dir
//         }
//
//         fn path(&self, name: impl AsRef<str>) -> PathBuf {
//             self.dir().join(name.as_ref())
//         }
//
//         /// A node contains several files, and the existence of the main directory is not not enough
//         /// to determine if a node exists as it could be created but empty.
//         fn exists(&self, name: impl AsRef<str>) -> bool {
//             let paths = NodePaths::new(&self.path(&name));
//             paths.setup().exists()
//         }
//
//         fn delete(&self, name: impl AsRef<str>) -> Result<()> {
//             self._delete(&name, false)
//         }
//     }
//
//     #[async_trait]
//     impl StateItemTrait for NodeState {
//         type Config = ();
//
//         fn new(path: PathBuf) -> Result<Self> {
//             std::fs::create_dir_all(&path)?;
//             let paths = NodePaths::new(&path);
//             let name = file_stem(&path)?;
//             let _ = std::fs::remove_file(paths.vault());
//             let _ = std::fs::remove_file(paths.identity());
//             Ok(Self { name, path, paths })
//         }
//
//         fn load(path: PathBuf) -> Result<Self> {
//             let paths = NodePaths::new(&path);
//             let name = file_stem(&path)?;
//             let setup = {
//                 let contents = std::fs::read_to_string(paths.setup())?;
//                 serde_json::from_str(&contents)?
//             };
//             let version = {
//                 let contents = std::fs::read_to_string(paths.version())?;
//                 contents.parse::<ConfigVersion>()?
//             };
//             let config = NodeConfig {
//                 setup,
//                 version,
//                 default_vault: paths.vault(),
//             };
//             Ok(Self {
//                 name,
//                 path,
//                 paths,
//                 config,
//             })
//         }
//
//         fn delete(&self) -> Result<()> {
//             self._delete(false)
//         }
//
//         fn path(&self) -> &PathBuf {
//             &self.path
//         }
//
//         fn config(&self) -> &Self::Config {
//             &self.config
//         }
//     }
// }
//
// pub async fn init_node_state(
//     cli_state: &CliState,
//     node_name: &str,
//     vault_name: Option<&str>,
//     identity_name: Option<&str>,
// ) -> miette::Result<()> {
//     debug!(name=%node_name, "initializing node state");
//     // Get vault specified in the argument, or get the default
//     let vault_state = cli_state.create_vault_state(vault_name).await?;
//
//     // // create an identity for the node
//     // let identity = cli_state
//     //     .get_identities(vault_state.get().await?)
//     //     .await?
//     //     .identities_creation()
//     //     .create_identity()
//     //     .await
//     //     .into_diagnostic()
//     //     .wrap_err("Failed to create identity")?;
//     //
//     // let identity_state = cli_state
//     //     .create_identity_state(identity.identifier(), identity_name)
//     //     .await?;
//
//     // Create the node with the given vault and identity
//     // let node_config = NodeConfigBuilder::default()
//     //     .vault(vault_state.path().clone())
//     //     .build(cli_state)?;
//     // cli_state.nodes.overwrite(node_name, node_config)?;
//
//     info!(name=%node_name, "node state initialized");
//     Ok(())
// }
//
// pub async fn add_project_info_to_node_state(
//     node_name: &str,
//     cli_state: &CliState,
//     project_path: Option<&PathBuf>,
// ) -> Result<Option<String>> {
//     debug!(name=%node_name, "Adding project info to state");
//     let proj_path = if let Some(path) = project_path {
//         Some(path.clone())
//     } else if let Ok(proj) = cli_state.projects.default() {
//         Some(proj.path().clone())
//     } else {
//         None
//     };
//
//     // match proj_path {
//     //     Some(path) => {
//     //         debug!(path=%path.display(), "Reading project info from path");
//     //         let s = std::fs::read_to_string(path)?;
//     //         let proj_info: ProjectConfigCompact = serde_json::from_str(&s)?;
//     //         let proj_lookup = ProjectLookup::from_project(&(&proj_info).into())
//     //             .await
//     //             .map_err(|e| {
//     //                 CliStateError::InvalidData(format!("Failed to read project: {}", e))
//     //             })?;
//     //         let proj_config = ProjectConfig::from(&proj_info);
//     //         let state = cli_state.nodes.get(node_name)?;
//     //         state.set_setup(state.config().setup_mut().set_project(proj_lookup.clone()))?;
//     //         cli_state
//     //             .projects
//     //             .overwrite(proj_lookup.name, proj_config)?;
//     //         Ok(Some(proj_lookup.id))
//     //     }
//     //     None => {
//     //         debug!("No project info used");
//     //         Ok(None)
//     //     }
//     // }
// }
//
// pub async fn update_enrolled_identity(cli_state: &CliState, node_name: &str) -> Result<Identifier> {
//     todo!("enroll an identity")
//     // let identities = cli_state.identities.list()?;
//     //
//     // let node_state = cli_state.nodes.get(node_name)?;
//     // let node_identifier = node_state.config().identifier()?;
//     //
//     // for mut identity in identities {
//     //     if node_identifier == identity.config().identifier() {
//     //         identity.set_enrollment_status()?;
//     //     }
//     // }
//     //
//     // Ok(node_identifier)
// }
//
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::config::lookup::InternetAddress;
//     use crate::nodes::models::transport::{TransportMode, TransportType};
//
//     #[test]
//     fn node_config_setup_transports_no_duplicates() {
//         let mut config = NodeSetupConfigV1 {
//             verbose: 0,
//             authority_node: None,
//             project: None,
//             transports: HashSet::new(),
//         };
//         let transport = CreateTransportJson {
//             tt: TransportType::Tcp,
//             tm: TransportMode::Listen,
//             addr: InternetAddress::V4("127.0.0.1:1020".parse().unwrap()),
//         };
//         config = config.add_transport(transport.clone());
//         assert_eq!(config.transports.len(), 1);
//         assert_eq!(config.transports.iter().next(), Some(&transport));
//
//         config = config.add_transport(transport);
//         assert_eq!(config.transports.len(), 1);
//     }
//
//     #[test]
//     fn node_config_setup_transports_parses_a_json_with_duplicate_entries() {
//         // This test is to ensure backwards compatibility, for versions where transports where stored as a Vec<>
//         let config_json = r#"{
//             "verbose": 0,
//             "authority_node": null,
//             "project": null,
//             "transports": [
//                 {"tt":"Tcp","tm":"Listen","addr":{"V4":"127.0.0.1:1020"}},
//                 {"tt":"Tcp","tm":"Listen","addr":{"V4":"127.0.0.1:1020"}}
//             ]
//         }"#;
//         let config = serde_json::from_str::<NodeSetupConfigV1>(config_json).unwrap();
//         assert_eq!(config.transports.len(), 1);
//     }
// }
