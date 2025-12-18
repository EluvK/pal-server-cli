mod local_storage;
mod psm;
mod server_status;

use clap::Parser;
use local_storage::LocalSaveStorageConfig;
use tencent_cloud_sdk::config::ClientConfig;

#[derive(clap::Parser, Debug)]
struct Args {
    #[clap(short, long, default_value = "config.toml")]
    config: String,

    #[clap(short, long, default_value_t = false)]
    init_config: bool,
}

#[derive(Debug, serde::Deserialize)]
struct Config {
    tcc_config: ClientConfig,
    server_status_file: String,
    local_storage: LocalSaveStorageConfig,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Hello, world!");
    let args = Args::parse();
    let config_content = std::fs::read_to_string(&args.config)?;
    let config: Config = toml::from_str(&config_content)?;
    println!("Config: {:?}", config);

    let psm = {
        let client = tencent_cloud_sdk::client::TencentCloudClient::new(&config.tcc_config);
        let server_manager = server_status::ServerManager::new(&config.server_status_file)?;
        let local_storage = local_storage::LocalStorage::new(config.local_storage);
        psm::PalServerManager::new(client, server_manager, local_storage)?
    };

    Ok(())
}
