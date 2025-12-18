mod cvm_utils;
mod local_storage;
mod psm;
mod server_status;

use std::path::Path;

use clap::Parser;
use local_storage::LocalSaveStorageConfig;
use tencent_cloud_sdk::config::ClientConfig;

#[derive(clap::Parser, Debug)]
struct Args {
    #[clap(default_value = "config.toml")]
    config: String,

    #[clap(long)]
    new: Option<String>,

    #[clap(long)]
    start: Option<String>,

    #[clap(long)]
    save: Option<String>,

    #[clap(long)]
    stop: Option<String>,

    /// debug mode
    #[clap(long)]
    test: bool,
}

#[derive(Debug, serde::Deserialize)]
struct Config {
    tcc_config: ClientConfig,
    server_status_filepath: String,
    local_storage: LocalSaveStorageConfig,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let log_path = Path::new("./");
    let _guard = file_log(log_path, true)?;

    let args = Args::parse();
    let config_content = std::fs::read_to_string(&args.config)?;
    let config: Config = toml::from_str(&config_content)?;
    println!("Config: {:?}", config);

    let mut psm = {
        let client = tencent_cloud_sdk::client::TencentCloudClient::new(&config.tcc_config);
        let server_manager = server_status::ServerManager::new(&config.server_status_filepath)?;
        let local_storage = local_storage::LocalStorage::new(config.local_storage);
        psm::PalServerManager::new(client, server_manager, local_storage)?
    };

    if let Some(name) = args.new {
        psm.new_save(&name).await?;
    } else if let Some(name) = args.start {
        psm.restart_save(&name).await?;
    } else if let Some(name) = args.save {
        psm.save_backup(&name).await?;
    } else if let Some(name) = args.stop {
        psm.stop_server(&name).await?;
    } else if args.test {
        psm.test().await?;
    }

    Ok(())
}

fn file_log(path: &Path, enable_debug: bool) -> anyhow::Result<impl Drop> {
    let file_path = path.join("logs");
    println!("logs file to: {file_path:?}");
    let file_appender = tracing_appender::rolling::RollingFileAppender::builder()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("psm")
        .filename_suffix("log")
        .build(file_path)?;
    let (non_blocking_appender, guard) = tracing_appender::non_blocking(file_appender);
    let mut subscriber = tracing_subscriber::fmt()
        .with_writer(non_blocking_appender)
        .with_timer(tracing_subscriber::fmt::time::UtcTime::rfc_3339())
        .with_ansi(false);
    if enable_debug {
        subscriber = subscriber.with_max_level(tracing::Level::DEBUG);
    }
    tracing::subscriber::set_global_default(subscriber.finish()).unwrap();
    tracing::info!("start");

    Ok(guard)
}
