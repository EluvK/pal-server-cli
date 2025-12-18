use serde::{Deserialize, Serialize};
use tencent_cloud_sdk::constant::InstanceType;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Server {
    pub name: String,
    pub status: Status,
    pub instance_type: String,
    pub save: Option<String>,
    pub ip_port: Option<String>,
    pub region: Option<String>,
    pub instance_id: Option<String>,
}

pub struct ServerManager {
    server: Vec<Server>,
    path: String,
}

impl ServerManager {
    pub fn new(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let server: Vec<Server> = toml::from_str(&content)?;
        Ok(Self {
            server,
            path: path.to_string(),
        })
    }

    pub fn get(&self, name: &str) -> anyhow::Result<Server> {
        self.server
            .iter()
            .find(|server| server.name == name)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Server {} not found", name))
    }

    pub fn init_save(&mut self, name: &str) -> anyhow::Result<()> {
        let server = Server {
            name: name.to_string(),
            status: Status::Stopped,
            instance_type: InstanceType::SA9Large16.to_string(),
            save: None,
            ip_port: None,
            region: None,
            instance_id: None,
        };
        self.server.push(server);
        // self.save()?;
        Ok(())
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub enum Status {
    Creating,
    Running,
    Stopping,
    Stopped,
}
