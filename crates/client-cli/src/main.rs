use clap::{Parser, Subcommand};
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::fs;

use common::{TimestampToken, SignerConfig};
use frost_core::secp256k1::generate_with_dealer;
use frost_core::sha256;
use nostr_sdk::prelude::*;

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

            let (group_pubkey, pubkey_package_hex, shares) = generate_with_dealer(n, k);

            println!("Group Public Key: {}", group_pubkey);

            // Generate Nostr keys for coordinator
            let coord_keys = Keys::generate();
            let coord_npub = coord_keys.public_key().to_bech32().expect("bech32 encode");
            let coord_nsec = coord_keys.secret_key().expect("secret key").to_bech32().expect("bech32 encode");
            println!("Coordinator npub: {}", coord_npub);
            println!("Coordinator nsec: {}", coord_nsec);

            // Generate Nostr keys for each signer and collect npubs
            let mut signer_npubs = Vec::new();
            let mut signer_nsecs = Vec::new();
            for i in 0..n {
                let keys = Keys::generate();
                let npub = keys.public_key().to_bech32().expect("bech32 encode");
                let nsec = keys.secret_key().expect("secret key").to_bech32().expect("bech32 encode");
                println!("Signer {} npub: {}", i + 1, npub);
                signer_npubs.push(npub);
                signer_nsecs.push(nsec);
            }

            // Write coordinator config (sectioned format matching CoordinatorAppConfig)
            let mut signers_toml = String::new();
            for (i, npub) in signer_npubs.iter().enumerate() {
                signers_toml.push_str(&format!(
                    "\n[[signers]]\nnpub = \"{}\"\nsigner_id = {}\n",
                    npub,
                    i + 1
                ));
            }

            let coord_toml = format!(
                "[coordinator]\nnsec = \"{}\"\nhttp_host = \"0.0.0.0\"\nhttp_port = 8000\n\n\
                 [frost]\nk = {}\nn = {}\npublic_key_package = \"{}\"\n\
                 {}\n\
                 [relays]\nurls = [\"ws://localhost:8080\"]\n",
                coord_nsec, k, n, pubkey_package_hex, signers_toml
            );

            let coord_path = out.join("coordinator.toml");
            fs::write(&coord_path, &coord_toml)?;
            println!("Wrote coordinator config to {:?}", coord_path);

            // Write signer configs
            for (i, share) in shares.iter().enumerate() {
                let signer_config = SignerConfig {
                    key_package: Some(serde_json::to_string(share)?),
                    signer_id: Some((i + 1) as u16),
                    coordinator_npub: coord_npub.clone(),
                    relay_urls: vec!["ws://localhost:8080".to_string()],
                    nsec: Some(signer_nsecs[i].clone()),
                    collector_url: None,
                };
                let path = out.join(format!("signer_{}.toml", i + 1));
                fs::write(&path, toml::to_string_pretty(&signer_config)?)?;
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

            if token.verify().unwrap_or(false) {
                 println!("\n[PASSED] Crypto Signature is VALID for this message.");
            } else {
                 println!("\n[FAILED] Crypto Signature is INVALID.");
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
            let res = client.post(format!("{}/api/v1/timestamp", coordinator))
                .json(&serde_json::json!({ "hash": file_hash }))
                .send()
                .await?;
            
            if (!res.status().is_success()) {
                println!("Server error: {}", res.status());
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