//! Jenkins Slaves Informations

use serde::{Deserialize, Serialize};

use crate::client_internals::{Name, Path, Result};
use crate::Jenkins;

pub mod computer;
pub mod monitor;

/// List of `Computer` associated to the `Jenkins` instance
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputerSet {
    /// Display name of the set
    pub display_name: String,
    /// Number of busy executors
    pub busy_executors: u32,
    /// Number of executors
    pub total_executors: u32,
    /// List of computers
    #[serde(rename = "computer")]
    pub computers: Vec<computer::CommonComputer>,
}

impl Jenkins {
    /// Get a `ComputerSet`
    pub async fn get_nodes(&self) -> Result<ComputerSet> {
        let response = self.get(&Path::Computers).await?.json().await?;
        Ok(response)
    }

    /// Get a `Computer`
    pub async fn get_node<'a, C>(&self, computer_name: C) -> Result<computer::CommonComputer>
    where
        C: Into<computer::ComputerName<'a>>,
    {
        let response = self
            .get(&Path::Computer {
                name: Name::Name(computer_name.into().0),
            })
            .await?
            .json()
            .await?;
        Ok(response)
    }

    /// Get the master `Computer`
    pub async fn get_master_node(&self) -> Result<computer::MasterComputer> {
        let response = self
            .get(&Path::Computer {
                name: Name::Name("(master)"),
            })
            .await?
            .json()
            .await?;
        Ok(response)
    }
}
