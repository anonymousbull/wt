use anyhow::{Result, Context};
use std::env;
use base64::{Engine};
use tokio::process::Command;
use tokio::time::{sleep, Duration};
use wolf_trader::constant::BASE_64_SSH_PRIVATE_KEY;
use wolf_trader::digitalocean_droplet::{create_digitalocean_droplet, DropletSize};
use wolf_trader::dns_manager::{manage_dns_records, DnsRecord};
use wolf_trader::ssh_executor::SshExecutor;

#[tokio::main]
async fn main() -> Result<()> {

    let host = "femi.market";
    let bin_name = env!("BIN_NAME");
    let ssh_public_keys = vec![
        env!("SSH_PUBLIC_KEY").to_string(),
    ];

    // Create the droplet and get its IP address
    let droplet_ip = create_digitalocean_droplet(bin_name, DropletSize::Small, ssh_public_keys).await?;
    println!("Droplet IP: {}", droplet_ip);


    println!("Waiting for 30 seconds to ensure the droplet is ready...");
    sleep(Duration::from_secs(30)).await;

    // Create a DNS record for the droplet
    manage_dns_records(host, &DnsRecord{
        r#type:"A".to_string(),
        name: bin_name.to_string(),
        data: droplet_ip.clone(),
        ttl: 30,
    }).await?;
    println!("DNS record created for {}.{}", bin_name, host);

    // Add a delay to ensure the droplet is ready for SSH connections
    println!("Waiting for 30 seconds to ensure the droplet is ready...");
    sleep(Duration::from_secs(30)).await;

    // Read and decode the private key from an environment variable
    let private_key = base64::engine::general_purpose::STANDARD
        .decode(BASE_64_SSH_PRIVATE_KEY)
        .context("Failed to decode private key")?;

    let user = "root";

    let mut executor = SshExecutor::new(&droplet_ip, user, private_key).await?;
    println!("SSH connection established.");

    let domain = format!("{}.{}", bin_name, host); // Replace with your domain

    let commands = vec![
        "apt update",
        "sudo apt install snapd",
        "apt install -y software-properties-common",
        "sudo snap install --classic certbot",
        "sudo ln -s /snap/bin/certbot /usr/bin/certbot",
        // Install Node.js 20 and npm
        "curl -fsSL https://deb.nodesource.com/setup_20.x | bash -",
        "apt install -y nodejs", // This should install both Node.js 20 and npm
        // Verify npm installation
        "npm -v",
        // Install PM2
        "npm install -g pm2",
    ];

    for command in commands {
        match executor.execute_command(command).await {
            Ok(output) => println!("Command executed successfully: {}\nOutput: {}", command, output),
            Err(e) => eprintln!("Error executing command '{}': {}", command, e),
        }
    }

    executor.certbot(&domain,"near@sent.com").await?;


    // Run local cargo build --release
    let output = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .envs(env::vars())
        .env("PORT","443")
        .output()
        .await
        .context("Failed to execute cargo build")?;
    if output.status.success() {
        println!("Cargo build succeeded:\n{}", String::from_utf8_lossy(&output.stdout));
    } else {
        panic!("Cargo build failed:\n{}", String::from_utf8_lossy(&output.stderr));
    }

    // Path to the compiled binary
    let local_binary_path = format!("{}/target/release/{bin_name}", env!("CARGO_MANIFEST_DIR"));

    // Copy the binary to the remote server
    executor.copy_file_to_remote(&local_binary_path, bin_name).await?;
    println!("Binary copied to remote server at {}", bin_name);

    // Make the binary executable
    let chmod = format!("chmod +x {}",bin_name);
    executor.execute_command(&chmod).await?;
    println!("Binary made executable");

    // Start the binary using PM2
    let pm2 = format!("pm2 start {bin_name} --name {bin_name}");
    executor.execute_command(&pm2).await?;
    println!("Binary started with PM2");

    Ok(())
}
