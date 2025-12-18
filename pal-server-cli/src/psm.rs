use std::str::FromStr;

use tencent_cloud_sdk::{client::TencentCloudClient, constant::Region};

use crate::{
    cvm_utils::{query_cvm_ip, query_spot_paid_price},
    local_storage::{LocalStorage, Script},
    server_status::{Server, ServerManager, ServiceInstanceType, Status},
};

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

    pub async fn test(&mut self) -> anyhow::Result<()> {
        let server = self.server_status.get("test")?;
        self.restore_save(&server).await?;

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        self.start_server(&server).await?;

        println!("Server status: {:?}", server);
        Ok(())
    }

    pub async fn new_save(&mut self, name: &str) -> anyhow::Result<()> {
        println!("Creating new save: {}", name);
        if self.server_status.get(name).is_ok() {
            anyhow::bail!("Save with name {} already exists", name);
        }
        // let server = self.q_and_c(name, ServiceInstanceType::T4C16G).await?;
        let server = self.q_and_c(name, ServiceInstanceType::T2C2G).await?;
        self.server_status.add(&server)?;

        // sleep 10s to wait for instance ready
        println!("Waiting for instance to be ready... sleep 10s");
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;

        self.init_server(&server).await?;
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        self.restore_save(&server).await?;
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        self.start_server(&server).await?;

        Ok(())
    }

    pub async fn restart_save(&mut self, name: &str) -> anyhow::Result<()> {
        println!("Restarting save: {}", name);
        let mut cur_server = self.server_status.get(name)?;

        if let Some(_ip) = &cur_server.ip {
            self.check_status(&mut cur_server).await?;
            if cur_server.status == Status::Running {
                anyhow::bail!("Server {} is already running", name);
            }
            return Ok(());
        }
        let service_instance_type = cur_server.service_instance_type.clone();

        let server = self.q_and_c(name, service_instance_type).await?;
        cur_server.instance_id = server.instance_id;
        cur_server.ip = server.ip;
        cur_server.status = Status::Running;
        cur_server.region = server.region;
        self.server_status.update(name, &cur_server)?;

        // sleep 10s to wait for instance ready
        println!("Waiting for instance to be ready... sleep 10s");
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;

        self.init_server(&cur_server).await?;
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        self.restore_save(&cur_server).await?;
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        self.start_server(&cur_server).await?;

        Ok(())
    }

    pub async fn save_backup(&mut self, name: &str) -> anyhow::Result<()> {
        println!("Backing up save: {}", name);
        let mut server = self.server_status.get(name)?;

        if server.status != Status::Running {
            anyhow::bail!("Server {} is not running", name);
        }

        self.backup_save(&mut server).await?;

        Ok(())
    }

    pub async fn stop_server(&mut self, name: &str) -> anyhow::Result<()> {
        println!("Stopping server: {}", name);
        let mut server = self.server_status.get(name)?;

        if server.status != Status::Running {
            anyhow::bail!("Server {} is not running", name);
        }
        self.backup_save(&mut server).await?;
        let region = Region::from_str(&server.region.as_ref().unwrap()).unwrap();
        let instance_id = server.instance_id.as_ref().unwrap();
        self.client
            .cvm()
            .instances()
            .terminate_instance(&region, instance_id)
            .await?;
        server.status = Status::Stopped;
        server.ip = None;
        server.instance_id = None;
        self.server_status.update(&server.name, &server)?;

        Ok(())
    }

    // easy for test
    async fn q_and_c(&self, name: &str, service_instance_type: ServiceInstanceType) -> anyhow::Result<Server> {
        self.query_and_create(
            name,
            &[Region::Nanjing, Region::Shanghai, Region::Guangzhou],
            &service_instance_type,
        )
        .await
        // self.query_and_create(name, &[Region::Nanjing], &ServiceInstanceType::T2C2G)
        //     .await
    }

    // step 1 query cheapest spot price and create instance
    async fn query_and_create(
        &self,
        name: &str,
        region: &[Region],
        service_instance_type: &ServiceInstanceType,
    ) -> anyhow::Result<Server> {
        let prices = query_spot_paid_price(&self.client, region, service_instance_type).await?;
        // println!(
        //     "[1] Cheapest spot price info: {:?}",
        //     (price, &region, &zone, &instance_type)
        // );

        let key_ids = self
            .client
            .cvm()
            .keys()
            .describe_key_pairs(&Region::Hongkong) // whatever here
            .await
            .map(|vk| vk.into_iter().map(|k| k.key_id).collect::<Vec<_>>())?;

        let mut final_service_id = None;
        let mut final_region = None;

        for (price, (region, zone, instance_type)) in prices {
            println!(
                "[1] Trying to create instance at region: {}, zone: {}, type: {}, price: {:?}",
                region, zone, instance_type, price
            );
            let security_group_id = self
                .client
                .cvm()
                .security_group()
                .describe_security_groups(&region)
                .await?
                .into_iter()
                .filter_map(|sg| {
                    sg.security_group_name
                        .to_ascii_lowercase()
                        .contains("palworld")
                        .then_some(sg.security_group_id)
                })
                .collect::<Vec<_>>();

            if let Ok(server_id) = self
                .client
                .cvm()
                .instances()
                .run_instance(&region, &zone, &instance_type, &key_ids, security_group_id)
                .await
            {
                println!(
                    "[1] Successfully created instance at region: {}, zone: {}, type: {}, price: {:?}, id: {}",
                    region, zone, instance_type, price, server_id
                );
                final_service_id = Some(server_id);
                final_region = Some(region);
                break;
            } else {
                println!(
                    "[1] Failed to create instance at region: {}, zone: {}, type: {}, price: {:?}",
                    region, zone, instance_type, price
                );
            }
        }
        let server_id = final_service_id.ok_or(anyhow::anyhow!("Failed to create instance"))?;
        let region = final_region.ok_or(anyhow::anyhow!("Region not found"))?;

        let ip = query_cvm_ip(&self.client, &region, &server_id).await?;
        println!("[1] New instance created: {}, ip: {}", server_id, ip);
        Ok(Server {
            name: name.to_string(),
            status: Status::Running,
            service_instance_type: service_instance_type.clone(),
            // instance_type,
            save: None,
            ip: Some(ip),
            region: Some(region.to_string()),
            instance_id: Some(server_id),
        })
    }

    // step 2 init server install necessaries
    async fn init_server(&self, server: &Server) -> anyhow::Result<()> {
        let ip = server.ip.as_ref().expect("No IP found for server");
        println!("[2] Start Initializing server: {} , ip: {}", server.name, ip);
        self.local_storage.upload_scripts(ip).await?;

        let res = self.local_storage.exec_shell(ip, Script::InstallServer).await?;
        println!("[2] Init server done, logs: {}", res);
        Ok(())
    }

    // step 3 restore save
    async fn restore_save(&self, server: &Server) -> anyhow::Result<()> {
        let ip = server.ip.as_ref().expect("No IP found for server");
        let Some(save_name) = &server.save else {
            // anyhow::bail!("No save found for server {}", server.name);
            println!("[3] No save found for server {}, skip restore save", server.name);
            return Ok(());
        };
        println!(
            "[3] Start Restoring save: {} to server: {} , ip: {}",
            save_name, server.name, ip
        );
        self.local_storage.upload_saves(save_name, ip).await?;
        self.local_storage.exec_shell(ip, Script::RestoreSave).await?;
        println!("[3] Restore save done");
        Ok(())
    }

    // step 4 start server
    async fn start_server(&self, server: &Server) -> anyhow::Result<()> {
        let ip = server.ip.as_ref().expect("No IP found for server");
        println!("[4] Start starting server: {} , ip: {}", server.name, ip);
        let res = self.local_storage.exec_shell(ip, Script::StartServer).await?;
        println!("[4] Start server done, logs: {}", res);
        Ok(())
    }

    // step 5 backup save
    async fn backup_save(&mut self, server: &mut Server) -> anyhow::Result<()> {
        let ip = server.ip.as_ref().expect("No IP found for server");
        println!("[5] Start backing up save from server: {} , ip: {}", server.name, ip);
        let save_name = self.local_storage.exec_shell(ip, Script::BackupSave).await?;
        self.local_storage.download_saves(&save_name, ip).await?;
        server.save = Some(save_name);
        self.server_status.update(&server.name, server)?;
        println!("[5] Backup save done");
        Ok(())
    }

    // helper functions ...
    async fn check_status(&mut self, server: &mut Server) -> anyhow::Result<()> {
        let ip = server.ip.as_ref().expect("No IP found for server");
        if let Ok(true) = self.local_storage.get_heartbeat(ip).await {
            server.status = Status::Running;
        } else {
            server.status = Status::Stopped;
            server.ip = None;
            server.instance_id = None;
        }
        self.server_status.update(&server.name, server)?;
        Ok(())
    }
}
