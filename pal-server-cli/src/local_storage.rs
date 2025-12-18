use std::{io::Read, net::TcpStream, path::Path};

use opendal::{
    Operator,
    services::{Fs, Sftp},
};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct LocalSaveStorageConfig {
    local_dir: String,
    remote_dir: String,
    ssh: SshConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SshConfig {
    pub prikey: String,
    pub user: String,
}

#[derive(Debug)]
pub struct LocalStorage {
    config: LocalSaveStorageConfig,
}

impl LocalStorage {
    pub fn new(config: LocalSaveStorageConfig) -> Self {
        Self { config }
    }
    fn build_local_op(&self) -> anyhow::Result<Operator> {
        let fs = Fs::default().root(&self.config.local_dir);
        Ok(Operator::new(fs)?.finish())
    }
    fn build_remote_sftp(&self, ip: &str) -> anyhow::Result<Operator> {
        let endpoint = format!("ssh://{}@{}:22", self.config.ssh.user, ip);
        let sftp = Sftp::default()
            .root(&self.config.remote_dir)
            .endpoint(&endpoint)
            .key(&self.config.ssh.prikey)
            .user(&self.config.ssh.user)
            .known_hosts_strategy("Accept");
        Ok(Operator::new(sftp)?.finish())
    }

    pub async fn upload_scripts(&self, ip: &str) -> anyhow::Result<()> {
        let local_op = self.build_local_op()?;
        let remote_op = self.build_remote_sftp(ip)?;

        let files = &[
            "install_server.sh",
            "restore_save.sh",
            "start_server.sh",
            "backup_save.sh",
        ];
        for file in files {
            let content = local_op.read(&format!("/scripts/{}", file)).await?;
            remote_op.write(&format!("/scripts/{}", file), content).await?;
        }
        Ok(())
    }

    pub async fn upload_saves(&self, save_name: &str, ip: &str) -> anyhow::Result<()> {
        let local_op = self.build_local_op()?;
        let remote_op = self.build_remote_sftp(ip)?;

        let content = local_op.read(&format!("/saves/{}", save_name)).await?;
        remote_op.write(&format!("/saves/{}", save_name), content).await?;
        Ok(())
    }

    pub async fn download_saves(&self, save_name: &str, ip: &str) -> anyhow::Result<()> {
        let local_op = self.build_local_op()?;
        let remote_op = self.build_remote_sftp(ip)?;
        let content = remote_op.read(&format!("/saves/{}", save_name)).await?;
        local_op.write(&format!("/saves/{}", save_name), content).await?;

        Ok(())
    }

    pub async fn get_heartbeat(&self, ip: &str) -> anyhow::Result<bool> {
        let user = &self.config.ssh.user;
        let prikey_path = &self.config.ssh.prikey;
        let tcp = TcpStream::connect_timeout(&format!("{ip}:22").parse()?, std::time::Duration::from_secs(1))?;
        let mut sess = ssh2::Session::new()?;
        sess.set_tcp_stream(tcp);
        sess.set_timeout(1000);
        sess.handshake()?;
        sess.userauth_pubkey_file(user, None, Path::new(prikey_path), None)?;
        sess.authenticated()
            .then(|| println!("ssh2 authed"))
            .ok_or(anyhow::anyhow!("ssh2 auth failed"))?;

        Ok(true)
    }

    pub async fn exec_shell(&self, ip: &str, script: Script) -> anyhow::Result<String> {
        let user = &self.config.ssh.user;
        let prikey_path = &self.config.ssh.prikey;

        let tcp = TcpStream::connect(format!("{ip}:22"))?;
        let mut sess = ssh2::Session::new()?;
        sess.set_tcp_stream(tcp);
        sess.handshake()?;
        sess.userauth_pubkey_file(user, None, Path::new(prikey_path), None)?;
        sess.authenticated()
            .then(|| println!("ssh2 authed"))
            .ok_or(anyhow::anyhow!("ssh2 auth failed"))?;

        let script_name = match script {
            Script::InstallServer => "install_server.sh",
            Script::RestoreSave => "restore_save.sh",
            Script::StartServer => "start_server.sh",
            Script::BackupSave => "backup_save.sh",
        };

        let mut channel = sess.channel_session()?;
        channel.exec(&format!(
            "(sh /home/{user}/psm/scripts/{script_name} >> /tmp/shell_log.log 2>&1 &)"
        ))?;

        const CHECK_INTERVAL: u64 = 5;
        loop {
            let mut channel = sess.channel_session()?;
            channel.exec(&format!("ps -ef | grep {script_name} | grep -v grep | wc -l",))?;
            let mut process_cnt = String::new();
            channel.read_to_string(&mut process_cnt)?;
            if process_cnt.trim() == "0" {
                break;
            }
            println!(" - running...");
            tokio::time::sleep(tokio::time::Duration::from_secs(CHECK_INTERVAL)).await;
        }

        let res = {
            let mut channel = sess.channel_session()?;
            channel.exec("tail -n 1 /tmp/shell_log.log")?;
            let mut logs = String::new();
            channel.read_to_string(&mut logs)?;
            println!(" -logs: {}", logs);
            channel.close()?;
            println!(" -status: {}", channel.exit_status()?);
            logs
        };
        Ok(res)
    }
}

#[derive(Debug)]
pub enum Script {
    /// install_server.sh
    InstallServer,
    /// restore_save.sh
    RestoreSave,
    /// start_server.sh
    StartServer,
    /// backup_save.sh
    BackupSave,
}
