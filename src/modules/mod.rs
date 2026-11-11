mod inventory_integrity;
pub mod ssh_config;

use crate::core::models::{AppState, Finding};
use anyhow::Result;

pub trait CheckModule {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn run(&self, state: &AppState) -> Result<Vec<Finding>>;
}

pub fn default_modules() -> Vec<Box<dyn CheckModule>> {
    vec![Box::new(inventory_integrity::InventoryIntegrity)]
}
