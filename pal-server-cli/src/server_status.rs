use serde::{Deserialize, Serialize};
use tencent_cloud_sdk::constant::InstanceType;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ServiceInstanceType {
    #[serde(rename = "2c2g")]
    T2C2G, // simple test
    #[serde(rename = "2c16g")]
    T2C16G,
    #[serde(rename = "4c16g")]
    T4C16G,
    #[serde(rename = "4c32g")]
    T4C32G,
    // T8C32G,
}
impl ServiceInstanceType {
    pub fn to_list(&self) -> Vec<InstanceType> {
        match self {
            ServiceInstanceType::T2C2G => vec![InstanceType::SA2Medium2, InstanceType::S5Medium2],
            // ServiceInstanceType::T4C8G => vec![InstanceType::SA2Large8, InstanceType::SA3Large8],
            ServiceInstanceType::T2C16G => vec![InstanceType::MA3Medium16, InstanceType::M5Medium16],
            ServiceInstanceType::T4C16G => vec![
                InstanceType::SA2Large16,
                InstanceType::SA3Large16,
                InstanceType::S5Large16,
                InstanceType::S6Large16,
                InstanceType::SA5Large16,
            ],
            ServiceInstanceType::T4C32G => vec![
                InstanceType::MA3Large32,
                InstanceType::MA2Large32,
                InstanceType::M5Large32,
                InstanceType::MA5Large32,
            ],
            // ServiceInstanceType::T8C32G => vec![InstanceType::SA22Xlarge32],
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Server {
    pub name: String,
    pub status: Status,
    pub service_instance_type: ServiceInstanceType,
    // pub instance_type: InstanceType,
    pub save: Option<String>,
    pub ip: Option<String>,
    pub region: Option<String>,
    pub instance_id: Option<String>,
}

pub struct ServerManager {
    data: ServerManagerData,
    path: String,
}

#[derive(Deserialize, Serialize)]
struct ServerManagerData {
    server: Vec<Server>,
}

impl ServerManager {
    pub fn new(path: &str) -> anyhow::Result<Self> {
        if !std::path::Path::new(path).exists() {
            // create empty file
            std::fs::write(path, toml::to_string(&ServerManagerData { server: vec![] })?)?;
        }
        let content = std::fs::read_to_string(path)?;
        let data: ServerManagerData = toml::from_str(&content)?;
        Ok(Self {
            data,
            path: path.to_string(),
        })
    }

    pub fn get(&self, name: &str) -> anyhow::Result<Server> {
        self.data
            .server
            .iter()
            .find(|server| server.name == name)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Server {} not found", name))
    }

    pub fn add(&mut self, server: &Server) -> anyhow::Result<()> {
        if self.data.server.iter().any(|s| s.name == server.name) {
            anyhow::bail!("Server {} already exists", server.name);
        }
        self.data.server.push(server.clone());
        std::fs::write(&self.path, toml::to_string(&self.data)?)?;
        Ok(())
    }

    pub fn update(&mut self, name: &str, server: &Server) -> anyhow::Result<()> {
        if let Some(existing_server) = self.data.server.iter_mut().find(|server| server.name == name) {
            *existing_server = server.clone();
            std::fs::write(&self.path, toml::to_string(&self.data)?)?;
            Ok(())
        } else {
            anyhow::bail!("Server {} not found", name);
        }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub enum Status {
    Creating,
    Running,
    Stopping,
    Stopped,
}
