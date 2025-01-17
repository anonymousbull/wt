use ssh2::Session;
use anyhow::{Result, Context};
use std::path::Path;
use std::net::TcpStream;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::io::{BufReader, Cursor, Read};
use std::sync::Arc;
use std::io::Write;
use std::str;

pub struct SshExecutor {
    session: Arc<Session>,
}

impl SshExecutor {
    pub async fn new(host: &str, user: &str, private_key: Vec<u8>) -> Result<Self> {
        let tcp = TcpStream::connect(format!("{}:22", host))
            .with_context(|| format!("Failed to connect to {}:22", host))?;
        let mut session = Session::new().context("Failed to create SSH session")?;
        session.set_tcp_stream(tcp);
        session.handshake().context("Failed to perform SSH handshake")?;

        // Convert the private key from Vec<u8> to &str
        let private_key_str = str::from_utf8(&private_key)
            .context("Failed to convert private key to string")?;

        // Use the private key as a &str
        session.userauth_pubkey_memory(user, None, private_key_str, None)
            .with_context(|| format!("Failed to authenticate user {}", user))?;

        Ok(SshExecutor { session: Arc::new(session) })
    }

    pub async fn execute_command(&self, command: &str) -> Result<String> {
        let session = Arc::clone(&self.session);
        let command = command.to_string();
        tokio::task::spawn_blocking(move || {
            let mut channel = session.channel_session().context("Failed to open SSH channel")?;
            channel.exec(command.as_str()).with_context(|| format!("Failed to execute command: {}", command))?;
            let mut output = String::new();
            channel.read_to_string(&mut output).context("Failed to read command output")?;
            channel.wait_close().context("Failed to close SSH channel")?;
            Ok(output)
        }).await.context("Failed to run blocking task")?
    }

    pub async fn copy_file_to_remote(&self, local_path: &str, remote_path: &str) -> Result<()> {
        let session = Arc::clone(&self.session);
        let local_path = local_path.to_string();
        let remote_path = remote_path.to_string();

        tokio::task::spawn_blocking(move || {
            let mut local_file = std::fs::File::open(&local_path)
                .with_context(|| format!("Failed to open local file: {}", local_path))?;
            let file_size = local_file.metadata()?.len();
            if file_size == 0 {
                return Err(anyhow::anyhow!("Local file is empty: {}", local_path));
            }

            let mut buffer = Vec::with_capacity(file_size as usize);
            local_file.read_to_end(&mut buffer)
                .with_context(|| format!("Failed to read from local file: {}", local_path))?;

            let mut remote_file = session.scp_send(Path::new(&remote_path), 0o644, file_size, None)
                .with_context(|| format!("Failed to open remote file: {}", remote_path))?;

            remote_file.write_all(&buffer)
                .with_context(|| format!("Failed to write to remote file: {}", remote_path))?;

            remote_file.send_eof().context("Failed to send EOF to remote file")?;
            remote_file.wait_eof().context("Failed to wait for EOF from remote file")?;
            remote_file.wait_close().context("Failed to close remote file")?;

            Ok(())
        }).await.context("Failed to run blocking task")?
    }

    pub async fn certbot(&self, domain: &str, email: &str) -> Result<(String, String)> {
        // Run Certbot to obtain the certificates
        let certbot_command = format!(
            "certbot certonly --standalone -d {} --non-interactive --agree-tos -m {}",
            domain, email
        );
        self.execute_command(&certbot_command).await?;

        // Read the fullchain.pem and privkey.pem files
        let fullchain_path = format!("/etc/letsencrypt/live/{}/fullchain.pem", domain);
        let privkey_path = format!("/etc/letsencrypt/live/{}/privkey.pem", domain);

        let fullchain = self.execute_command(&format!("cat {}", fullchain_path)).await?;
        let privkey = self.execute_command(&format!("cat {}", privkey_path)).await?;

        let dir_path = Path::new("./ssl");
        if !dir_path.exists() {
            std::fs::create_dir_all(dir_path).context("Failed to create ssl directory")?;
        }

        // Write the fullchain.pem
        let fullchain_file_path = dir_path.join(format!("fullchain.pem"));
        std::fs::write(&fullchain_file_path, &fullchain).context("Failed to write fullchain.pem")?;

        // Write the privkey.pem
        let privkey_file_path = dir_path.join(format!("privkey.pem"));
        std::fs::write(&privkey_file_path, &privkey).context("Failed to write privkey.pem")?;

        println!("Saved ssl certificates to ./ssl folder");

        Ok((fullchain, privkey))
    }


}

#[cfg(test)]
mod tests {
    use super::SshExecutor;
    use tokio;

    #[tokio::test]
    async fn test_execute_command() {
        let host = "localhost"; // Use your local SSH server
        let user = "your_username";
        let private_key_path = "/path/to/your/private_key";

        // let executor = SshExecutor::new(host, user, private_key_path).await.expect("Failed to create SSH executor");
        //
        // let command = "echo Hello, world!";
        // let result = executor.execute_command(command).await;
        //
        // assert!(result.is_ok());
        // assert_eq!(result.unwrap().trim(), "Hello, world!");
    }

    #[tokio::test]
    async fn test_copy_file_to_remote() {
        let host = "localhost"; // Use your local SSH server
        let user = "your_username";
        let private_key_path = "/path/to/your/private_key";
        let local_path = "/path/to/local/file";
        let remote_path = "/path/to/remote/destination";

        // let executor = SshExecutor::new(host, user, private_key_path).await.expect("Failed to create SSH executor");
        //
        // let result = executor.copy_file_to_remote(local_path, remote_path).await;
        //
        // assert!(result.is_ok());
    }
} 