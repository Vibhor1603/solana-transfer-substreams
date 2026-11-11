//! Solana transfer extractor Substreams module.
//!
//! Emits native SOL (System Program), SPL Token, and Token-2022
//! `Transfer` / `TransferChecked` instructions from each successful
//! transaction. SPL / Token-2022 `from`/`to` are wallet owners when
//! present in transaction token-balance metadata.

mod pb;

use std::collections::HashMap;

use substreams::errors::Error;
use substreams_solana::block_view::InstructionView;
use substreams_solana::pb::sf::solana::r#type::v1::{
    Block, ConfirmedTransaction, TransactionStatusMeta,
};
use substreams_solana_program_instructions::token_instruction_2022::TokenInstruction;

use pb::solana::transfers::v1::{Transfer, Transfers};

/// System Program — native SOL transfers.
const SYSTEM_PROGRAM: &str = "11111111111111111111111111111111";
/// Classic SPL Token program.
const TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
/// Token-2022 (Token Extensions) program.
const TOKEN_2022_PROGRAM: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";

/// System Program `Transfer` discriminator (u32 little-endian = 2).
const SYSTEM_TRANSFER_DISCRIMINATOR: [u8; 4] = [2, 0, 0, 0];

#[substreams::handlers::map]
fn map_transfers(block: Block) -> Result<Transfers, Error> {
    let mut transfers = Vec::new();

    for trx in &block.transactions {
        // Failed txs did not settle — skip their instructions.
        if !trx.is_successful() {
            continue;
        }

        let owners = token_account_owners(trx);

        // Top-level + inner (CPI) instructions.
        for ix in trx.walk_instructions() {
            if let Some(transfer) = decode_instruction(&ix, &owners) {
                transfers.push(transfer);
            }
        }
    }

    Ok(Transfers { transfers })
}

fn decode_instruction(
    ix: &InstructionView,
    owners: &HashMap<String, String>,
) -> Option<Transfer> {
    let program_id = ix.program_id().to_string();

    match program_id.as_str() {
        SYSTEM_PROGRAM => decode_system_transfer(ix),
        TOKEN_PROGRAM | TOKEN_2022_PROGRAM => decode_token_transfer(ix, owners),
        _ => None,
    }
}

fn decode_system_transfer(ix: &InstructionView) -> Option<Transfer> {
    let data = ix.data();
    if data.len() < 12 || data[0..4] != SYSTEM_TRANSFER_DISCRIMINATOR {
        return None;
    }

    let accounts = ix.accounts();
    if accounts.len() < 2 {
        return None;
    }

    let amount = u64::from_le_bytes(data[4..12].try_into().ok()?);

    // System Transfer accounts are already wallet pubkeys.
    Some(Transfer {
        from: accounts[0].to_string(),
        to: accounts[1].to_string(),
        amount,
    })
}

fn decode_token_transfer(
    ix: &InstructionView,
    owners: &HashMap<String, String>,
) -> Option<Transfer> {
    let instruction = TokenInstruction::unpack(&ix.data()).ok()?;
    let accounts = ix.accounts();

    let (source, destination, amount) = match instruction {
        #[allow(deprecated)]
        TokenInstruction::Transfer { amount } => {
            // accounts: source, destination, authority, ...
            if accounts.len() < 2 {
                return None;
            }
            (accounts[0].to_string(), accounts[1].to_string(), amount)
        }
        TokenInstruction::TransferChecked { amount, .. } => {
            // accounts: source, mint, destination, authority, ...
            if accounts.len() < 3 {
                return None;
            }
            (accounts[0].to_string(), accounts[2].to_string(), amount)
        }
        _ => return None,
    };

    // Option B: prefer wallet owners over token-account addresses.
    let from = owners
        .get(&source)
        .cloned()
        .unwrap_or(source);
    let to = owners
        .get(&destination)
        .cloned()
        .unwrap_or(destination);

    Some(Transfer { from, to, amount })
}

/// Map token-account address → wallet owner using pre/post token balances.
///
/// Single map module only — no foundational store / DB / extra services.
fn token_account_owners(trx: &ConfirmedTransaction) -> HashMap<String, String> {
    let mut owners = HashMap::new();
    let Some(meta) = trx.meta.as_ref() else {
        return owners;
    };

    let accounts: Vec<String> = trx
        .resolved_accounts()
        .into_iter()
        .map(|bytes| bs58::encode(bytes).into_string())
        .collect();

    ingest_balance_owners(&mut owners, &accounts, meta);
    owners
}

fn ingest_balance_owners(
    owners: &mut HashMap<String, String>,
    accounts: &[String],
    meta: &TransactionStatusMeta,
) {
    for balance in meta
        .pre_token_balances
        .iter()
        .chain(meta.post_token_balances.iter())
    {
        if balance.owner.is_empty() {
            continue;
        }
        let idx = balance.account_index as usize;
        if let Some(account) = accounts.get(idx) {
            owners.insert(account.clone(), balance.owner.clone());
        }
    }
}
