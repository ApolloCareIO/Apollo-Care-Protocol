# APH Token-2022 Integration Guide

## Overview

Apollo Care Protocol uses the **APH Token** as its native governance and staking token. APH is deployed as a **Token-2022 (Token Extensions)** token on Solana, enabling advanced features like transfer fees.

### Token Details

|----------|-------|
| **Token Name** | Apollo Care Token |
| **Symbol** | APH |
| **Mint Address** | `6S3T6f1mmhxQWKR1gMiZR1SZLpu396jnnRMGqAZUj3Qj` |
| **Program** | Token-2022 (`TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb`) |
| **Decimals** | 9 |
| **Total Supply** | 1,000,000,000 APH |
| **Mint Authority** | `7jMC5SKBDmweVLst8YKTNP77wX7RpUpV6MvYVcfqswyt` |

### Token Extensions

APH was **minted as a Token-2022 token from day one** with the following extensions:

1. **Transfer Fee** (built-in, rate adjustable)
   - Fee Rate: 2% (200 basis points) - target rate
   - Max Fee: 200,000 APH (0.02% of total supply)
   - Fee collection supports protocol sustainability
   - **Current Status**: Fee rate set to 0% during presale, will be activated to 2% post-presale via transfer fee config authority

## Token Allocations

Apollo Care tokenizes the healthcare coverage infrastructure layer, transforming health coverage from for-profit extraction into a community-owned public utility.

| Category | Percentage | Amount | Purpose |
|----------|------------|--------|---------|
| **Community & Ecosystem** | 47% | 470M APH | DAO treasury for ICO, rewards, grants, partnerships, marketing |
| **Core Team** | 22% | 220M APH | Founders & advisors |
| **Seed & Strategic** | 10% | 100M APH | Early investors |
| **Insurance Reserve** | 10% | 100M APH | DAO-controlled emergency fund for claim shortfalls |
| **Liquidity** | 6% | 60M APH | DEX liquidity pools (Raydium, etc.) |
| **Operations** | 5% | 50M APH | Platform maintenance, infrastructure, legal/regulatory |

### APH Token Utility

$APH is NOT primarily a speculative asset. It serves three core functions:

1. **Governance** - Members vote on coverage policies, pricing, committee elections
2. **Capital Staking** - APH provides decentralized Tier-2 backstop capital
3. **Community Incentives** - Rewards for participation, referrals, wellness programs

The Community & Ecosystem allocation (47%) is the largest, underscoring Apollo's community-first ethos where the majority of tokens remain under DAO control for ecosystem growth.

## Integration Architecture

### Dual Token Strategy

The protocol uses two token programs:

1. **Token-2022** (`anchor_spl::token_2022`) - For APH operations
   - Staking
   - Rewards
   - Governance

2. **Standard SPL Token** (`anchor_spl::token`) - For USDC operations
   - Contributions
   - Claims payouts
   - Reserve management

### Key Code Changes

#### 1. Account Types

```rust
// For APH Token-2022 accounts
use anchor_spl::token_interface::{
    Mint as MintInterface,
    TokenAccount as TokenAccountInterface,
    TokenInterface,
    TransferChecked,
};

// Account declarations
pub aph_mint: InterfaceAccount<'info, MintInterface>,
pub aph_token_account: InterfaceAccount<'info, TokenAccountInterface>,
pub token_program: Interface<'info, TokenInterface>,
```

#### 2. Transfer Operations

```rust
// Token-2022 requires transfer_checked with decimals
token_interface::transfer_checked(
    CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        TransferChecked {
            from: ctx.accounts.from_account.to_account_info(),
            mint: ctx.accounts.aph_mint.to_account_info(),
            to: ctx.accounts.to_account.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        },
    ),
    amount,
    decimals, // Must match mint decimals (9 for APH)
)?;
```

#### 3. Vault Creation

```rust
#[account(
    init,
    payer = authority,
    token::mint = aph_mint,
    token::authority = vault_authority,
    token::token_program = token_program, // Specifies Token-2022
    seeds = [b"vault_token", aph_mint.key().as_ref()],
    bump
)]
pub vault_token_account: InterfaceAccount<'info, TokenAccountInterface>,
```

## Transfer Fee Handling

### Pre-Presale (Current State)

Transfer fee is **disabled**. Tokens transfer at face value.

### Post-Presale (After DEX Launch)

Transfer fee will be **enabled** at 2%:

```rust
// Calculate expected fee
let fee = amount * 200 / 10000; // 2%
let fee = fee.min(50_000_000_000); // Max 50 APH

// Net received = amount - fee
let net_received = amount - fee;
```

### Fee-Aware Transfer Helper

```rust
pub fn calculate_transfer_with_fee(
    amount: u64,
    transfer_fee_bps: u16,
    max_fee: u64,
) -> (u64, u64) {
    if transfer_fee_bps == 0 {
        return (amount, 0);
    }

    let fee = amount
        .saturating_mul(transfer_fee_bps as u64)
        .checked_div(10000)
        .unwrap_or(0);

    let actual_fee = fee.min(max_fee);

    (amount, actual_fee)
}
```

## Program Dependencies

### Cargo.toml Updates

```toml
[dependencies]
anchor-lang = "0.30.1"
anchor-spl = "0.30.1"
spl-token-2022 = "5.0"
apollo_core = { path = "../apollo_core", features = ["cpi"] }
```

### Import Pattern

```rust
// For APH operations
use anchor_spl::token_interface::{
    self, Mint as MintInterface, TokenAccount as TokenAccountInterface,
    TokenInterface, TransferChecked,
};

// For USDC operations (standard SPL Token)
use anchor_spl::token::{self, Token, TokenAccount, Mint, Transfer};
```

## Testing with Token-2022

### Local Validator Setup

```bash
# Start validator with Token-2022 program
solana-test-validator \
  --bpf-program TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb \
  /path/to/spl_token_2022.so
```

### Creating Test Token-2022 Mint

```typescript
import { createMint } from "@solana/spl-token";
import { TOKEN_2022_PROGRAM_ID } from "@solana/spl-token";

const aphMint = await createMint(
  connection,
  payer,
  mintAuthority.publicKey,
  freezeAuthority.publicKey,
  9, // decimals
  undefined,
  undefined,
  TOKEN_2022_PROGRAM_ID
);
```

### Creating Token-2022 Accounts

```typescript
import { createAssociatedTokenAccount } from "@solana/spl-token";
import { TOKEN_2022_PROGRAM_ID } from "@solana/spl-token";

const ata = await createAssociatedTokenAccount(
  connection,
  payer,
  aphMint,
  owner.publicKey,
  undefined,
  TOKEN_2022_PROGRAM_ID
);
```

## Security Considerations

### Mint Validation

Always validate the APH mint address:

```rust
use apollo_core::aph_token;

// In account constraints
#[account(
    constraint = aph_mint.key() == staking_config.aph_mint @ StakingError::InvalidAphMint
)]
pub aph_mint: InterfaceAccount<'info, MintInterface>,
```

### Program ID Validation

Ensure using the correct token program:

```rust
// Verify Token-2022 program
let token_2022_id: Pubkey = anchor_spl::token_2022::ID;
require!(
    ctx.accounts.token_program.key() == token_2022_id,
    StakingError::InvalidTokenProgram
);
```

### Fee Awareness

When transfer fees are enabled, ensure proper handling:

```rust
// Don't assume amount sent = amount received
// Always account for potential transfer fee deduction
let expected_fee = calculate_expected_fee(amount, fee_bps);
let expected_received = amount - expected_fee;
```

## Constants Reference

```rust
// From apollo_core/src/lib.rs
pub mod aph_token {
    pub const APH_MINT: &str = "6S3T6f1mmhxQWKR1gMiZR1SZLpu396jnnRMGqAZUj3Qj";
    pub const MINT_AUTHORITY: &str = "7jMC5SKBDmweVLst8YKTNP77wX7RpUpV6MvYVcfqswyt";
    pub const DECIMALS: u8 = 9;
    pub const TOTAL_SUPPLY: u64 = 1_000_000_000_000_000_000; // 1B with 9 decimals
    pub const TRANSFER_FEE_BPS: u16 = 200; // 2%
    pub const MAX_TRANSFER_FEE: u64 = 50_000_000_000; // 50 APH
}
```

## Migration Notes

### From SPL Token to Token-2022

1. Update Cargo.toml dependencies
2. Replace `Token` with `TokenInterface`
3. Replace `TokenAccount` with `InterfaceAccount<TokenAccountInterface>`
4. Replace `Mint` with `InterfaceAccount<MintInterface>`
5. Replace `token::transfer` with `token_interface::transfer_checked`
6. Add `mint` account to transfer contexts
7. Pass `decimals` parameter to transfers

### Account Compatibility

Token-2022 accounts are **not** directly compatible with standard SPL Token operations. Use the appropriate program for each token type:

- APH → Token-2022
- USDC → Standard SPL Token

## Troubleshooting

### Common Errors

1. **"Invalid program id"** - Using wrong token program for account type
2. **"Account not initialized"** - Token account created with wrong program
3. **"Invalid mint"** - Mint doesn't match expected address
4. **"Insufficient funds"** - Not accounting for transfer fee when enabled

### Debugging Tips

```rust
msg!("Token program: {}", ctx.accounts.token_program.key());
msg!("APH Mint: {}", ctx.accounts.aph_mint.key());
msg!("Expected: {}", aph_token::APH_MINT);
```
