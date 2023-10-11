use crate::Result;
use miette::IntoDiagnostic;
use ockam_api::nodes::models::portal::OutletStatus;
use ockam_core::Address;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::net::SocketAddr;
use std::str::FromStr;

/// The ModelState stores all the data which is not maintained by the NodeManager.
#[derive(Serialize, Deserialize, Clone)]
pub struct ModelState {
    #[serde(default = "Vec::new")]
    pub(crate) tcp_outlets: Vec<OutletStatus>,
}

impl Default for ModelState {
    fn default() -> Self {
        ModelState::new(vec![])
    }
}

impl ModelState {
    pub fn new(tcp_outlets: Vec<OutletStatus>) -> Self {
        Self { tcp_outlets }
    }
}

#[derive(FromRow)]
pub struct TcpOutletRow {
    socket_addr: String,
    worker_addr: String,
    alias: String,
    payload: Option<String>,
}

impl TcpOutletRow {
    pub(crate) fn socket_addr(&self) -> Result<SocketAddr> {
        Ok(SocketAddr::from_str(&self.socket_addr)?)
    }

    pub(crate) fn worker_addr(&self) -> Result<Address> {
        Ok(Address::from_str(&self.worker_addr).into_diagnostic()?)
    }

    pub(crate) fn alias(&self) -> String {
        self.alias.clone()
    }

    pub(crate) fn payload(&self) -> Option<String> {
        self.payload.clone()
    }

    pub(crate) fn tcp_outlet_status(&self) -> Result<OutletStatus> {
        Ok(OutletStatus::new(
            self.socket_addr()?,
            self.worker_addr()?,
            self.alias(),
            self.payload(),
        ))
    }
}
