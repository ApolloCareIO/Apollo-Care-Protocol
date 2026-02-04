# Solana Development Best Practices for Apollo Care

This document outlines Solana-specific best practices implemented in the Apollo Care Protocol,
based on the official Solana documentation (solana.com/docs/core).

## Account Model

### Key Principles (from solana.com/docs/core/accounts)

1. **All data is stored in accounts** - Solana uses a key-value store model
2. **Maximum account size is 10 MiB** - Plan data structures accordingly
3. **Rent is required** - Accounts must maintain minimum lamport balance
4. **Ownership matters** - Only the owner program can modify account data

### Apollo Implementation

- All state accounts derive from `Account<'info, T>` with `#[account]` macro
- Account sizes calculated with `InitSpace` derive macro
- Seeds defined as constants (`SEED_PREFIX`) for consistency
- Owner verification through Anchor constraints

## Program Derived Addresses (PDAs)

### Key Principles (from solana.com/docs/core/pda)

1. **PDAs fall off the Ed25519 curve** - No private key exists
2. **Use canonical bump** - First valid bump (255 → 0) for security
3. **Deterministic derivation** - Same seeds + program ID = same address
4. **Programs can sign for PDAs** - Via `invoke_signed`

### Apollo Implementation

```rust
// Standard PDA pattern used throughout Apollo
#[account]
pub struct ClaimAccount {
    // ... fields
    pub bump: u8,  // Store canonical bump
}

impl ClaimAccount {
    pub const SEED_PREFIX: &'static [u8] = b"claim";
}

// In instruction accounts:
#[account(
    seeds = [ClaimAccount::SEED_PREFIX, &claim_id.to_le_bytes()],
    bump = claim.bump,  // Verify canonical bump
)]
pub claim: Account<'info, ClaimAccount>,
```

### Security: Always Verify Canonical Bump

⚠️ **CRITICAL**: Always verify the canonical bump to prevent PDA spoofing attacks.

```rust
// ✅ CORRECT - verifies stored bump
#[account(
    seeds = [b"config"],
    bump = config.bump,
)]

// ❌ WRONG - allows any valid bump
#[account(
    seeds = [b"config"],
    bump,  // Dangerous if not verified elsewhere
)]
```

## Cross Program Invocations (CPI)

### Key Principles (from solana.com/docs/core/cpi)

1. **Max depth is 4** - Stack height starts at 1, max is 5
2. **Account privileges extend** - Signer/writable status propagates
3. **Use `invoke_signed` for PDA signers** - Programs sign for their PDAs

### Apollo Implementation

```rust
// CPI to reserves program for claim payment
let signer_seeds: &[&[&[u8]]] = &[&[
    ClaimsConfig::SEED_PREFIX,
    &[ctx.accounts.claims_config.bump],
]];

invoke_signed(
    &transfer_ix,
    &[from.clone(), to.clone(), program.clone()],
    signer_seeds,
)?;
```

### CPI Depth Tracking

Apollo's claim payment flow:
1. Client → apollo_claims (depth 1)
2. apollo_claims → apollo_reserves (depth 2)
3. apollo_reserves → token_program (depth 3)

⚠️ Keep CPI chains under 4 to allow composability.

## Transaction Fees

### Key Principles (from solana.com/docs/core/fees)

1. **Base fee**: 5000 lamports per signature
2. **Priority fee**: CU price × CU limit
3. **Default CU per instruction**: 200,000
4. **Default CU per transaction**: 1.4 million

### Apollo Optimizations

For complex claim processing:
```rust
// Recommend clients set compute budget for attestation
// Typical usage: ~150,000 CU for simple claims
// Complex claims with AI oracle: ~300,000 CU
ComputeBudgetProgram.setComputeUnitLimit({ units: 300_000 })
```

## Transaction Size (1232 bytes)

### Key Principles (from solana.com/docs/core/transactions)

- Max transaction size: 1232 bytes
- Consider Address Lookup Tables for many accounts
- Batch operations when possible

### Apollo Considerations

- Claim submission fits in single transaction
- Multi-attestation may require Address Lookup Tables
- Large data stored off-chain (IPFS hashes stored on-chain)

## Error Handling

### Best Practices

1. Use descriptive error codes with `#[error_code]`
2. Include error messages for debugging
3. Fail early and explicitly

### Apollo Pattern

```rust
#[error_code]
pub enum ClaimsError {
    #[msg("Claims processing not initialized")]
    NotInitialized,
    
    #[msg("Claim amount exceeds benefit limit")]
    ExceedsBenefitLimit,
    // ...
}
```

## Testing Checklist

Before deployment, verify:

- [ ] All PDAs use canonical bump verification
- [ ] CPI depth never exceeds 4
- [ ] Account sizes calculated correctly
- [ ] Error messages are descriptive
- [ ] No uninitialized account vulnerabilities
- [ ] Owner checks on all writable accounts
- [ ] Signer checks for privileged operations

## References

- [Solana Accounts](https://solana.com/docs/core/accounts)
- [Solana Transactions](https://solana.com/docs/core/transactions)
- [Solana Fees](https://solana.com/docs/core/fees)
- [Solana Programs](https://solana.com/docs/core/programs)
- [Solana PDAs](https://solana.com/docs/core/pda)
- [Solana CPI](https://solana.com/docs/core/cpi)

---
*Document generated: 2026-02-04*
*Based on: solana.com/docs/core*
