//! Devnet Integration Tests for the Vault Program
//!
//! These tests run against the LIVE devnet deployment.
//! Prerequisites:
//!   1. Program deployed: solana program deploy ./target/deploy/vault.so
//!   2. Solana CLI configured for devnet: solana config set --url devnet
//!   3. Keypair funded: solana airdrop 2
//!
//! Run with: cargo test --test devnet_tests -- --nocapture
//!
//! ⚠️ Each test run costs real devnet SOL for transaction fees + rent.

use std::str::FromStr;
use std::thread::sleep;
use std::time::Duration;

use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_message::Message;
use solana_pubkey::Pubkey;
use solana_rpc_client::rpc_client::RpcClient;
use solana_signer::Signer;
use solana_transaction::Transaction;

// ─── Constants ─────────────────────────────────────────────────────────

const DEVNET_RPC: &str = "https://api.devnet.solana.com";
const PROGRAM_ID: &str = "BfJKG9PC4yKEJF1NkUppnSvXUoGjJgPKXEjNgkZthdPF";

/// Seconds to wait between transactions to avoid rate limiting
const TX_DELAY: u64 = 2;

// ─── Helpers ───────────────────────────────────────────────────────────

fn program_id() -> Pubkey {
    Pubkey::from_str(PROGRAM_ID).unwrap()
}

fn rpc() -> RpcClient {
    RpcClient::new(DEVNET_RPC.to_string())
}

/// Load the default Solana CLI keypair (~/.config/solana/id.json)
fn load_payer() -> Keypair {
    let keypair_path = dirs::home_dir()
        .expect("No home dir")
        .join(".config/solana/id.json");
    let keypair_bytes = std::fs::read_to_string(&keypair_path)
        .unwrap_or_else(|_| panic!("Cannot read keypair at {:?}", keypair_path));
    let bytes: Vec<u8> = serde_json::from_str(&keypair_bytes).expect("Invalid keypair JSON");
    // The JSON file has 64 bytes: first 32 are the secret key
    let secret: [u8; 32] = bytes[..32].try_into().expect("Keypair too short");
    Keypair::new_from_array(secret)
}

fn vault_pda(owner: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"vault", owner.as_ref()], &program_id())
}

fn send_tx(client: &RpcClient, ix: Instruction, payer: &Keypair) -> Result<String, String> {
    let blockhash = client.get_latest_blockhash().map_err(|e| e.to_string())?;
    let tx = Transaction::new(
        &[payer],
        Message::new(&[ix], Some(&payer.pubkey())),
        blockhash,
    );
    client
        .send_and_confirm_transaction(&tx)
        .map(|sig| sig.to_string())
        .map_err(|e| e.to_string())
}

fn wait() {
    sleep(Duration::from_secs(TX_DELAY));
}

// ─── Instruction Builders ──────────────────────────────────────────────

fn build_initialize_ix(payer: &Pubkey, vault: &Pubkey, bump: u8) -> Instruction {
    Instruction {
        program_id: program_id(),
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new(*vault, false),
            AccountMeta::new_readonly(solana_system_interface::program::id(), false),
        ],
        data: vec![0x00, bump],
    }
}

fn build_deposit_ix(owner: &Pubkey, vault: &Pubkey, amount: u64) -> Instruction {
    let mut data = vec![0x01];
    data.extend_from_slice(&amount.to_le_bytes());
    Instruction {
        program_id: program_id(),
        accounts: vec![
            AccountMeta::new(*owner, true),
            AccountMeta::new(*vault, false),
            AccountMeta::new_readonly(solana_system_interface::program::id(), false),
        ],
        data,
    }
}

fn build_withdraw_ix(owner: &Pubkey, vault: &Pubkey, amount: u64, bump: u8) -> Instruction {
    let mut data = vec![0x02];
    data.extend_from_slice(&amount.to_le_bytes());
    data.push(bump);
    Instruction {
        program_id: program_id(),
        accounts: vec![
            AccountMeta::new(*owner, true),
            AccountMeta::new(*vault, false),
            AccountMeta::new_readonly(solana_system_interface::program::id(), false),
        ],
        data,
    }
}

/// Read the vault account and return (discriminator, owner, amount)
fn read_vault(client: &RpcClient, vault: &Pubkey) -> Option<([u8; 8], Pubkey, u64)> {
    let account = client.get_account(vault).ok()?;
    if account.data.len() != 48 {
        return None;
    }
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&account.data[0..8]);
    let owner = Pubkey::new_from_array(account.data[8..40].try_into().unwrap());
    let amount = u64::from_le_bytes(account.data[40..48].try_into().unwrap());
    Some((disc, owner, amount))
}

const VAULT_DISCRIMINATOR: [u8; 8] = [0x56, 0x61, 0x75, 0x6c, 0x74, 0x21, 0x21, 0x21];

// ─── Tests ─────────────────────────────────────────────────────────────
//
// Run with: cargo test --test devnet_tests -- --nocapture
//
// The lifecycle test runs all steps sequentially in one function
// to avoid parallel execution issues (all tests share the same
// vault PDA on devnet).

/// Full lifecycle: initialize → deposit → withdraw (all sequential)
#[test]
fn devnet_full_lifecycle() {
    let client = rpc();
    let payer = load_payer();
    let (vault_pda, bump) = vault_pda(&payer.pubkey());

    println!("══════════════════════════════════════════════");
    println!("  DEVNET VAULT LIFECYCLE TEST");
    println!("══════════════════════════════════════════════");
    println!("  Payer:   {}", payer.pubkey());
    println!("  Vault:   {}", vault_pda);
    println!("  Program: {}", program_id());
    println!("  Bump:    {}", bump);
    println!();

    // ── Step 1: Initialize ──────────────────────────────────────────
    println!("─── Step 1: Initialize Vault ───");
    if let Some((disc, owner, amount)) = read_vault(&client, &vault_pda) {
        if disc == VAULT_DISCRIMINATOR {
            println!(
                "  ✅ Already initialized (owner: {}, amount: {} lamports)",
                owner, amount
            );
        }
    } else {
        let ix = build_initialize_ix(&payer.pubkey(), &vault_pda, bump);
        let sig = send_tx(&client, ix, &payer).expect("❌ Initialize failed");
        println!("  ✅ Initialize TX: {}", sig);
        wait();

        let (disc, owner, amount) =
            read_vault(&client, &vault_pda).expect("Vault not found after init");
        assert_eq!(disc, VAULT_DISCRIMINATOR, "Bad discriminator");
        assert_eq!(owner, payer.pubkey(), "Owner mismatch");
        assert_eq!(amount, 0, "Initial amount should be 0");
        println!("  ✅ Vault state: owner={}, amount={}", owner, amount);
    }
    println!();

    // ── Step 2: Deposit ─────────────────────────────────────────────
    println!("─── Step 2: Deposit SOL ───");
    let before = read_vault(&client, &vault_pda).expect("Vault not found");
    let amount_before = before.2;
    println!(
        "  Vault before: {} lamports ({:.9} SOL)",
        amount_before,
        amount_before as f64 / 1e9
    );

    let deposit_amount: u64 = 10_000_000; // 0.01 SOL
    println!(
        "  Depositing:   {} lamports ({:.9} SOL)",
        deposit_amount,
        deposit_amount as f64 / 1e9
    );

    let ix = build_deposit_ix(&payer.pubkey(), &vault_pda, deposit_amount);
    let sig = send_tx(&client, ix, &payer).expect("❌ Deposit failed");
    println!("  ✅ Deposit TX: {}", sig);
    wait();

    let (_, _, amount_after_deposit) =
        read_vault(&client, &vault_pda).expect("Vault not found after deposit");
    println!(
        "  Vault after:  {} lamports ({:.9} SOL)",
        amount_after_deposit,
        amount_after_deposit as f64 / 1e9
    );
    assert_eq!(
        amount_after_deposit,
        amount_before + deposit_amount,
        "Amount mismatch after deposit"
    );
    println!("  ✅ Deposit verified!");
    println!();

    // ── Step 3: Withdraw ────────────────────────────────────────────
    println!("─── Step 3: Withdraw SOL ───");
    let withdraw_amount = deposit_amount / 2; // withdraw half of what we deposited
    println!(
        "  Vault before:  {} lamports ({:.9} SOL)",
        amount_after_deposit,
        amount_after_deposit as f64 / 1e9
    );
    println!(
        "  Withdrawing:   {} lamports ({:.9} SOL)",
        withdraw_amount,
        withdraw_amount as f64 / 1e9
    );

    let payer_balance_before = client.get_balance(&payer.pubkey()).unwrap();

    let ix = build_withdraw_ix(&payer.pubkey(), &vault_pda, withdraw_amount, bump);
    let sig = send_tx(&client, ix, &payer).expect("❌ Withdraw failed");
    println!("  ✅ Withdraw TX: {}", sig);
    wait();

    let (_, _, amount_after_withdraw) =
        read_vault(&client, &vault_pda).expect("Vault not found after withdraw");
    println!(
        "  Vault after:   {} lamports ({:.9} SOL)",
        amount_after_withdraw,
        amount_after_withdraw as f64 / 1e9
    );
    assert_eq!(
        amount_after_withdraw,
        amount_after_deposit - withdraw_amount,
        "Amount mismatch after withdraw"
    );

    let payer_balance_after = client.get_balance(&payer.pubkey()).unwrap();
    println!(
        "  Payer balance: {} → {} lamports",
        payer_balance_before, payer_balance_after
    );
    println!("  ✅ Withdraw verified!");
    println!();

    // ── Summary ─────────────────────────────────────────────────────
    println!("══════════════════════════════════════════════");
    println!("  ✅ ALL DEVNET TESTS PASSED");
    println!(
        "  Final vault amount: {} lamports ({:.9} SOL)",
        amount_after_withdraw,
        amount_after_withdraw as f64 / 1e9
    );
    println!("══════════════════════════════════════════════");
}

/// Read-only: just check current vault state (safe to run anytime)
#[test]
fn devnet_check_vault_state() {
    let client = rpc();
    let payer = load_payer();
    let (vault_pda, _) = vault_pda(&payer.pubkey());

    println!("\n=== DEVNET: Check Vault State ===");
    println!("  Payer:   {}", payer.pubkey());
    println!("  Vault:   {}", vault_pda);

    match read_vault(&client, &vault_pda) {
        Some((disc, owner, amount)) => {
            println!("  Discriminator: {:?}", disc);
            println!("  Valid:         {}", disc == VAULT_DISCRIMINATOR);
            println!("  Owner:         {}", owner);
            println!(
                "  Amount:        {} lamports ({:.9} SOL)",
                amount,
                amount as f64 / 1e9
            );

            let vault_account = client.get_account(&vault_pda).unwrap();
            println!(
                "  Vault lamports (total): {} ({:.9} SOL)",
                vault_account.lamports,
                vault_account.lamports as f64 / 1e9
            );
            println!("  Vault program owner: {}", vault_account.owner);
            println!("  ✅ Vault is live on devnet!");
        }
        None => {
            println!("  ⚠️  Vault not found or has invalid data. Initialize it first.");
        }
    }

    let balance = client.get_balance(&payer.pubkey()).unwrap();
    println!(
        "  Payer balance: {} lamports ({:.9} SOL)",
        balance,
        balance as f64 / 1e9
    );
}
