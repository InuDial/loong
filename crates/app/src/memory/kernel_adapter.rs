use async_trait::async_trait;
use loong_contracts::MemoryPlaneError;
use loong_kernel::{CoreMemoryAdapter, MemoryCoreOutcome, MemoryCoreRequest};

use super::runtime_config::MemoryRuntimeConfig;

pub struct KernelMemoryAdapter {
    config: MemoryRuntimeConfig,
}

impl KernelMemoryAdapter {
    pub fn new() -> Self {
        Self {
            config: super::runtime_config::get_memory_runtime_config().clone(),
        }
    }

    pub fn with_config(config: MemoryRuntimeConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl CoreMemoryAdapter for KernelMemoryAdapter {
    fn name(&self) -> &str {
        "mvp-memory"
    }

    async fn execute_core_memory(
        &self,
        request: MemoryCoreRequest,
    ) -> Result<MemoryCoreOutcome, MemoryPlaneError> {
        super::execute_memory_core_with_config(request, &self.config)
            .map_err(MemoryPlaneError::Execution)
    }
}

pub type MvpMemoryAdapter = KernelMemoryAdapter;
