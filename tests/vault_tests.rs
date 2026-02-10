use std::str::FromStr;

use litesvm::LiteSVM;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_message::Message;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::Transaction;

/// Program ID — a deterministic, valid pubkey for local testing
fn program_id() -> Pubkey {
    // 32 bytes: "vault_program___________________" padded
    // Pubkey::from([
    //     0x76, 0x61, 0x75, 0x6c, 0x74, 0x5f, 0x70, 0x72, // vault_pr
    //     0x6f, 0x67, 0x72, 0x61, 0x6d, 0x5f, 0x5f, 0x5f, // ogram___
    //     0x5f, 0x5f, 0x5f, 0x5f, 0x5f, 0x5f, 0x5f, 0x5f, // ________
    //     0x5f, 0x5f, 0x5f, 0x5f, 0x5f, 0x5f, 0x5f, 0x5f, // ________
    // ])

    // for devnet testing
    Pubkey::from_str("BfJKG9PC4yKEJF1NkUppnSvXUoGjJgPKXEjNgkZthdPF").unwrap()
}

/// Load the compiled SBF program into a LiteSVM instance
fn setup() -> LiteSVM {
    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!("../target/deploy/vault.so");
    svm.add_program(program_id(), program_bytes)
        .expect("Failed to load vault program");
    svm
}

/// Derive the vault PDA for a given owner
fn vault_pda(owner: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"vault", owner.as_ref()], &program_id())
}

// ─── Instruction Builders ──────────────────────────────────────────────

/// Build the Initialize instruction
/// Data layout: [0x00, bump]
fn build_initialize_ix(payer: &Pubkey, vault: &Pubkey, bump: u8) -> Instruction {
    Instruction {
        program_id: program_id(),
        accounts: vec![
            AccountMeta::new(*payer, true),  // signer, writable
            AccountMeta::new(*vault, false), // writable (PDA, not signer from client side)
            AccountMeta::new_readonly(solana_system_interface::program::id(), false),
        ],
        data: vec![0x00, bump],
    }
}

/// Build the Deposit instruction
/// Data layout: [0x01, amount_le_bytes(8)]
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

/// Build the Withdraw instruction
/// Data layout: [0x02, amount_le_bytes(8), bump]
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

// ─── Helpers ───────────────────────────────────────────────────────────

/// Read the vault account data and return (discriminator, owner, amount)
fn read_vault_state(svm: &LiteSVM, vault: &Pubkey) -> ([u8; 8], Pubkey, u64) {
    let account = svm.get_account(vault).expect("Vault account not found");
    let data = &account.data;
    assert_eq!(data.len(), 48, "Vault data should be 48 bytes");

    let mut discriminator = [0u8; 8];
    discriminator.copy_from_slice(&data[0..8]);

    let owner = Pubkey::new_from_array(data[8..40].try_into().unwrap());

    let amount = u64::from_le_bytes(data[40..48].try_into().unwrap());

    (discriminator, owner, amount)
}

const VAULT_DISCRIMINATOR: [u8; 8] = [0x56, 0x61, 0x75, 0x6c, 0x74, 0x21, 0x21, 0x21]; // "Vault!!!"

// ─── Tests ─────────────────────────────────────────────────────────────

#[test]
fn test_initialize_vault() {
    let mut svm = setup();
    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    let (vault_pda, bump) = vault_pda(&payer.pubkey());

    let ix = build_initialize_ix(&payer.pubkey(), &vault_pda, bump);
    let tx = Transaction::new(
        &[&payer],
        Message::new(&[ix], Some(&payer.pubkey())),
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    // Verify vault state
    let (disc, owner, amount) = read_vault_state(&svm, &vault_pda);
    assert_eq!(disc, VAULT_DISCRIMINATOR, "Discriminator mismatch");
    assert_eq!(owner, payer.pubkey(), "Owner mismatch");
    assert_eq!(amount, 0, "Initial amount should be 0");

    // Verify vault is owned by our program
    let vault_account = svm.get_account(&vault_pda).unwrap();
    assert_eq!(
        vault_account.owner,
        program_id(),
        "Vault should be owned by the program"
    );
}

#[test]
fn test_deposit_sol() {
    let mut svm = setup();
    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    let (vault_pda, bump) = vault_pda(&payer.pubkey());

    // Initialize first
    let init_ix = build_initialize_ix(&payer.pubkey(), &vault_pda, bump);
    let tx = Transaction::new(
        &[&payer],
        Message::new(&[init_ix], Some(&payer.pubkey())),
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    // Deposit 1 SOL
    let deposit_amount: u64 = 1_000_000_000;
    let deposit_ix = build_deposit_ix(&payer.pubkey(), &vault_pda, deposit_amount);
    let tx2 = Transaction::new(
        &[&payer],
        Message::new(&[deposit_ix], Some(&payer.pubkey())),
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx2).unwrap();

    // Check vault state amount updated
    let (_, _, amount) = read_vault_state(&svm, &vault_pda);
    assert_eq!(
        amount, deposit_amount,
        "Vault amount should be 1 SOL after deposit"
    );

    // Check vault lamports increased
    let vault_account = svm.get_account(&vault_pda).unwrap();
    assert!(
        vault_account.lamports >= deposit_amount,
        "Vault lamports should include the deposited amount"
    );
}

#[test]
fn test_multiple_deposits() {
    let mut svm = setup();
    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    let (vault_pda, bump) = vault_pda(&payer.pubkey());

    // Initialize
    let init_ix = build_initialize_ix(&payer.pubkey(), &vault_pda, bump);
    let tx = Transaction::new(
        &[&payer],
        Message::new(&[init_ix], Some(&payer.pubkey())),
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    // Deposit 1 SOL
    let deposit1: u64 = 1_000_000_000;
    let ix1 = build_deposit_ix(&payer.pubkey(), &vault_pda, deposit1);
    let tx1 = Transaction::new(
        &[&payer],
        Message::new(&[ix1], Some(&payer.pubkey())),
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx1).unwrap();

    // Deposit 2 SOL
    let deposit2: u64 = 2_000_000_000;
    let ix2 = build_deposit_ix(&payer.pubkey(), &vault_pda, deposit2);
    let tx2 = Transaction::new(
        &[&payer],
        Message::new(&[ix2], Some(&payer.pubkey())),
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx2).unwrap();

    // Total should be 3 SOL
    let (_, _, amount) = read_vault_state(&svm, &vault_pda);
    assert_eq!(
        amount,
        deposit1 + deposit2,
        "Vault should hold 3 SOL after two deposits"
    );
}

#[test]
fn test_withdraw_sol() {
    let mut svm = setup();
    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    let (vault_pda, bump) = vault_pda(&payer.pubkey());

    // Initialize
    let init_ix = build_initialize_ix(&payer.pubkey(), &vault_pda, bump);
    let tx = Transaction::new(
        &[&payer],
        Message::new(&[init_ix], Some(&payer.pubkey())),
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    // Deposit 3 SOL
    let deposit_amount: u64 = 3_000_000_000;
    let dep_ix = build_deposit_ix(&payer.pubkey(), &vault_pda, deposit_amount);
    let tx2 = Transaction::new(
        &[&payer],
        Message::new(&[dep_ix], Some(&payer.pubkey())),
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx2).unwrap();

    // Withdraw 1 SOL
    let withdraw_amount: u64 = 1_000_000_000;
    let wd_ix = build_withdraw_ix(&payer.pubkey(), &vault_pda, withdraw_amount, bump);
    let tx3 = Transaction::new(
        &[&payer],
        Message::new(&[wd_ix], Some(&payer.pubkey())),
        svm.latest_blockhash(),
    );

    let payer_lamports_before = svm.get_account(&payer.pubkey()).unwrap().lamports;
    svm.send_transaction(tx3).unwrap();
    let payer_lamports_after = svm.get_account(&payer.pubkey()).unwrap().lamports;

    // Check vault amount decreased
    let (_, _, amount) = read_vault_state(&svm, &vault_pda);
    assert_eq!(
        amount,
        deposit_amount - withdraw_amount,
        "Vault should hold 2 SOL after withdrawal"
    );

    // Check payer got lamports back (minus TX fee)
    assert!(
        payer_lamports_after > payer_lamports_before - 10_000, // some tolerance for tx fee
        "Payer should have received the withdrawn SOL"
    );
}

#[test]
fn test_full_lifecycle_initialize_deposit_withdraw() {
    let mut svm = setup();
    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    let (vault_pda, bump) = vault_pda(&payer.pubkey());

    // 1. Initialize
    let init_ix = build_initialize_ix(&payer.pubkey(), &vault_pda, bump);
    let tx = Transaction::new(
        &[&payer],
        Message::new(&[init_ix], Some(&payer.pubkey())),
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    let (disc, owner, amount) = read_vault_state(&svm, &vault_pda);
    assert_eq!(disc, VAULT_DISCRIMINATOR);
    assert_eq!(owner, payer.pubkey());
    assert_eq!(amount, 0);

    // 2. Deposit 5 SOL
    let dep_ix = build_deposit_ix(&payer.pubkey(), &vault_pda, 5_000_000_000);
    let tx2 = Transaction::new(
        &[&payer],
        Message::new(&[dep_ix], Some(&payer.pubkey())),
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx2).unwrap();

    let (_, _, amount) = read_vault_state(&svm, &vault_pda);
    assert_eq!(amount, 5_000_000_000);

    // 3. Withdraw 2 SOL
    let wd_ix = build_withdraw_ix(&payer.pubkey(), &vault_pda, 2_000_000_000, bump);
    let tx3 = Transaction::new(
        &[&payer],
        Message::new(&[wd_ix], Some(&payer.pubkey())),
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx3).unwrap();

    let (_, _, amount) = read_vault_state(&svm, &vault_pda);
    assert_eq!(amount, 3_000_000_000);

    // 4. Withdraw remaining 3 SOL
    let wd_ix2 = build_withdraw_ix(&payer.pubkey(), &vault_pda, 3_000_000_000, bump);
    let tx4 = Transaction::new(
        &[&payer],
        Message::new(&[wd_ix2], Some(&payer.pubkey())),
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx4).unwrap();

    let (_, _, amount) = read_vault_state(&svm, &vault_pda);
    assert_eq!(amount, 0, "Vault should be empty after withdrawing all");
}

#[test]
fn test_withdraw_insufficient_balance_fails() {
    let mut svm = setup();
    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    let (vault_pda, bump) = vault_pda(&payer.pubkey());

    // Initialize
    let init_ix = build_initialize_ix(&payer.pubkey(), &vault_pda, bump);
    let tx = Transaction::new(
        &[&payer],
        Message::new(&[init_ix], Some(&payer.pubkey())),
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    // Deposit 1 SOL
    let dep_ix = build_deposit_ix(&payer.pubkey(), &vault_pda, 1_000_000_000);
    let tx2 = Transaction::new(
        &[&payer],
        Message::new(&[dep_ix], Some(&payer.pubkey())),
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx2).unwrap();

    // Try to withdraw 5 SOL (more than balance) — should fail
    let wd_ix = build_withdraw_ix(&payer.pubkey(), &vault_pda, 5_000_000_000, bump);
    let tx3 = Transaction::new(
        &[&payer],
        Message::new(&[wd_ix], Some(&payer.pubkey())),
        svm.latest_blockhash(),
    );
    let result = svm.send_transaction(tx3);
    assert!(result.is_err(), "Withdraw more than balance should fail");
}

#[test]
fn test_wrong_owner_cannot_deposit() {
    let mut svm = setup();
    let owner = Keypair::new();
    let attacker = Keypair::new();
    svm.airdrop(&owner.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&attacker.pubkey(), 10_000_000_000).unwrap();

    let (vault_pda, bump) = vault_pda(&owner.pubkey());

    // Owner initializes the vault
    let init_ix = build_initialize_ix(&owner.pubkey(), &vault_pda, bump);
    let tx = Transaction::new(
        &[&owner],
        Message::new(&[init_ix], Some(&owner.pubkey())),
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    // Attacker tries to deposit into owner's vault — should fail (owner mismatch)
    let dep_ix = build_deposit_ix(&attacker.pubkey(), &vault_pda, 1_000_000_000);
    let tx2 = Transaction::new(
        &[&attacker],
        Message::new(&[dep_ix], Some(&attacker.pubkey())),
        svm.latest_blockhash(),
    );
    let result = svm.send_transaction(tx2);
    assert!(
        result.is_err(),
        "Attacker should not be able to deposit into another user's vault"
    );
}

#[test]
fn test_wrong_owner_cannot_withdraw() {
    let mut svm = setup();
    let owner = Keypair::new();
    let attacker = Keypair::new();
    svm.airdrop(&owner.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&attacker.pubkey(), 10_000_000_000).unwrap();

    let (vault_pda, bump) = vault_pda(&owner.pubkey());

    // Owner initializes & deposits
    let init_ix = build_initialize_ix(&owner.pubkey(), &vault_pda, bump);
    let tx = Transaction::new(
        &[&owner],
        Message::new(&[init_ix], Some(&owner.pubkey())),
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    let dep_ix = build_deposit_ix(&owner.pubkey(), &vault_pda, 2_000_000_000);
    let tx2 = Transaction::new(
        &[&owner],
        Message::new(&[dep_ix], Some(&owner.pubkey())),
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx2).unwrap();

    // Attacker tries to withdraw from owner's vault — should fail
    let wd_ix = build_withdraw_ix(&attacker.pubkey(), &vault_pda, 1_000_000_000, bump);
    let tx3 = Transaction::new(
        &[&attacker],
        Message::new(&[wd_ix], Some(&attacker.pubkey())),
        svm.latest_blockhash(),
    );
    let result = svm.send_transaction(tx3);
    assert!(
        result.is_err(),
        "Attacker should not be able to withdraw from another user's vault"
    );
}

#[test]
fn test_invalid_instruction_discriminator_fails() {
    let mut svm = setup();
    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    let (vault_pda, _bump) = vault_pda(&payer.pubkey());

    // Send invalid discriminator byte (0xFF)
    let ix = Instruction {
        program_id: program_id(),
        accounts: vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(vault_pda, false),
            AccountMeta::new_readonly(solana_system_interface::program::id(), false),
        ],
        data: vec![0xFF],
    };
    let tx = Transaction::new(
        &[&payer],
        Message::new(&[ix], Some(&payer.pubkey())),
        svm.latest_blockhash(),
    );
    let result = svm.send_transaction(tx);
    assert!(result.is_err(), "Invalid discriminator should fail");
}

#[test]
fn test_empty_instruction_data_fails() {
    let mut svm = setup();
    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    let (vault_pda, _bump) = vault_pda(&payer.pubkey());

    // Send empty data
    let ix = Instruction {
        program_id: program_id(),
        accounts: vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(vault_pda, false),
            AccountMeta::new_readonly(solana_system_interface::program::id(), false),
        ],
        data: vec![],
    };
    let tx = Transaction::new(
        &[&payer],
        Message::new(&[ix], Some(&payer.pubkey())),
        svm.latest_blockhash(),
    );
    let result = svm.send_transaction(tx);
    assert!(result.is_err(), "Empty instruction data should fail");
}

#[test]
fn test_two_users_independent_vaults() {
    let mut svm = setup();
    let user_a = Keypair::new();
    let user_b = Keypair::new();
    svm.airdrop(&user_a.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&user_b.pubkey(), 10_000_000_000).unwrap();

    let (vault_a, bump_a) = vault_pda(&user_a.pubkey());
    let (vault_b, bump_b) = vault_pda(&user_b.pubkey());

    // Both vaults should be at different addresses
    assert_ne!(
        vault_a, vault_b,
        "Different users should have different vault PDAs"
    );

    // User A initializes & deposits 2 SOL
    let init_a = build_initialize_ix(&user_a.pubkey(), &vault_a, bump_a);
    let tx = Transaction::new(
        &[&user_a],
        Message::new(&[init_a], Some(&user_a.pubkey())),
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    let dep_a = build_deposit_ix(&user_a.pubkey(), &vault_a, 2_000_000_000);
    let tx2 = Transaction::new(
        &[&user_a],
        Message::new(&[dep_a], Some(&user_a.pubkey())),
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx2).unwrap();

    // User B initializes & deposits 4 SOL
    let init_b = build_initialize_ix(&user_b.pubkey(), &vault_b, bump_b);
    let tx3 = Transaction::new(
        &[&user_b],
        Message::new(&[init_b], Some(&user_b.pubkey())),
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx3).unwrap();

    let dep_b = build_deposit_ix(&user_b.pubkey(), &vault_b, 4_000_000_000);
    let tx4 = Transaction::new(
        &[&user_b],
        Message::new(&[dep_b], Some(&user_b.pubkey())),
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx4).unwrap();

    // Verify independent state
    let (_, owner_a, amount_a) = read_vault_state(&svm, &vault_a);
    let (_, owner_b, amount_b) = read_vault_state(&svm, &vault_b);

    assert_eq!(owner_a, user_a.pubkey());
    assert_eq!(amount_a, 2_000_000_000);
    assert_eq!(owner_b, user_b.pubkey());
    assert_eq!(amount_b, 4_000_000_000);
}
