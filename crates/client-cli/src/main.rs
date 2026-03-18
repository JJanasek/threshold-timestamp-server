use clap::{Parser, Subcommand};
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::fs;

use common::{TimestampToken, CoordinatorConfig, SignerConfig};
use frost_core::secp256k1::generate_with_dealer;
use frost_core::sha256;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate key shares for the servers (Offline)
    Keygen {
        #[arg(long, default_value_t = 2)]
        k: u16,
        #[arg(long, default_value_t = 3)]
        n: u16,
        #[arg(long, default_value = "./configs")]
        out: PathBuf,
    },
    /// Request a timestamp for a file
    Timestamp {
        file: PathBuf,
        #[arg(long, default_value = "http://localhost:8000")]
        coordinator: String,
    },
    /// Verify a timestamp token against a file (Offline or Online)
    Verify {
        file: PathBuf,
        token: PathBuf,
        /// Optional: Check against coordinator's public key
        #[arg(long)]
        coordinator: Option<String>,
    },
    /// Inspect a token without verifying file hash (Offline)
    Inspect {
        token: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Keygen { k, n, out } => {
            println!("Generating {} shares with threshold {}...", n, k);
            fs::create_dir_all(&out)?;
            
            let (group_pubkey, shares) = generate_with_dealer(n, k);
            
            println!("Group Public Key: {}", group_pubkey);

            // Write coordinator config
            let coord_config = CoordinatorConfig {
                group_public_key: group_pubkey.clone(),
                relay_urls: vec!["ws://localhost:8080".to_string()],
                signers_npubs: vec![], // To be filled manually or by advanced logic
                port: 8000,
            };
            let coord_path = out.join("coordinator.toml");
            fs::write(&coord_path, toml::to_string_pretty(&coord_config)?)
                .context("Failed to write coordinator config")?;
            println!("Wrote coordinator config to {:?}", coord_path);

            // Write signer configs
            for (i, share) in shares.iter().enumerate() {
                // share.secret_share is already a hex string in the wrapper
                let signer_config = SignerConfig {
                    key_package: share.secret_share.clone(), 
                    coordinator_npub: "replace_with_coordinator_npub".to_string(),
                    relay_urls: vec!["ws://localhost:8080".to_string()],
                };
                let path = out.join(format!("signer_{}.toml", i + 1));
                fs::write(&path, toml::to_string_pretty(&signer_config)?)
                    .context("Failed to write signer config")?;
                println!("Wrote signer {} config to {:?}", i + 1, path);
            }
        }

        Commands::Inspect { token } => {
            let content = fs::read_to_string(token).context("Failed to read token file")?;
            let token: TimestampToken = serde_json::from_str(&content).context("Invalid token format")?;
            
            println!("Token Serial: {}", token.serial_number);
            println!("Timestamp:    {}", token.timestamp);
            println!("File Hash:    {}", token.file_hash);
            println!("Signature:    {}", token.signature);
            println!("Group PubKey: {}", token.group_public_key);

            match token.verify() {
                 Ok(true) => println!("\n[PASSED] Crypto Signature is VALID for this message."),
                 Ok(false) => println!("\n[FAILED] Crypto Signature is INVALID."),
                 Err(e) => println!("\n[ERROR] Verification error: {}", e),
            }
        }

        Commands::Verify { file, token, coordinator: _ } => {
            // 1. Hash the file
            let file_bytes = fs::read(&file).context("Failed to read input file")?;
            let calculated_hash = hex::encode(sha256(&file_bytes));

            // 2. Load Token
            let content = fs::read_to_string(token).context("Failed to read token file")?;
            let token_struct: TimestampToken = serde_json::from_str(&content)?;

            // 3. Compare Hashes
            if calculated_hash != token_struct.file_hash {
                println!("[FAILED] File hash mismatch!");
                println!("  Expected: {}", token_struct.file_hash);
                println!("  Actual:   {}", calculated_hash);
                return Ok(());
            }

            // 4. Verify Signature
            match token_struct.verify() {
                Ok(true) => println!("[SUCCESS] The timestamp is valid and matches the file."),
                Ok(false) => println!("[FAILED] Invalid cryptographic signature."),
                Err(e) => println!("[ERROR] Verification error: {}", e),
            }
        }

        Commands::Timestamp { file, coordinator } => {
            // 1. Hash file
            let file_bytes = fs::read(&file).context("Failed to read file")?;
            let file_hash = hex::encode(sha256(&file_bytes));

            println!("Requesting timestamp for hash: {}", file_hash);

            // 2. Send to Coordinator
            let client = reqwest::Client::new();
            let res = client.post(format!("{}/timestamp", coordinator))
                .json(&serde_json::json!({ "hash": file_hash }))
                .send()
                .await?;
            
            // Removed extra parentheses
            if !res.status().is_success() {
                println!("Server error: {}", res.status());
                println!("Body: {}", res.text().await?);
                return Ok(());
            }

            let token: TimestampToken = res.json().await?;

            // 3. Save result
            let out_name = file.with_extension("tst");
            let json = serde_json::to_string_pretty(&token)?;
            fs::write(&out_name, json)?;
            
            println!("Success! Token saved to {:?}", out_name);
        }
    }

    Ok(())
}