use async_trait::async_trait;
use loong_contracts::ToolPlaneError;
use loong_kernel::{CoreToolAdapter, ToolCoreOutcome, ToolCoreRequest};

use super::runtime_config::ToolRuntimeConfig;

pub struct KernelToolAdapter {
    config: Option<ToolRuntimeConfig>,
}

impl Default for KernelToolAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl KernelToolAdapter {
    pub fn new() -> Self {
        Self { config: None }
    }

    pub fn with_config(config: ToolRuntimeConfig) -> Self {
        Self {
            config: Some(config),
        }
    }
}

#[async_trait]
impl CoreToolAdapter for KernelToolAdapter {
    fn name(&self) -> &str {
        "mvp-tools"
    }

    async fn execute_core_tool(
        &self,
        request: ToolCoreRequest,
    ) -> Result<ToolCoreOutcome, ToolPlaneError> {
        match &self.config {
            Some(config) => super::execute_tool_core_with_config(request, config),
            None => super::execute_tool_core(request),
        }
        .map_err(ToolPlaneError::Execution)
    }
}

pub type MvpToolAdapter = KernelToolAdapter;
