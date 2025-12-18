use tencent_cloud_sdk::client::TencentCloudClient;

use crate::{local_storage::LocalStorage, server_status::ServerManager};

pub struct PalServerManager {
    pub client: TencentCloudClient,
    pub server_status: ServerManager,
    pub local_storage: LocalStorage,
}

impl PalServerManager {
    pub fn new(
        client: TencentCloudClient,
        server_status: ServerManager,
        local_storage: LocalStorage,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            client,
            server_status,
            local_storage,
        })
    }

    pub fn new_save(&self, name: &str) -> anyhow::Result<()> {
        println!("Creating new save: {}", name);
        if self.server_status.get(name).is_ok() {
            anyhow::bail!("Save with name {} already exists", name);
        }

        Ok(())
    }
}
