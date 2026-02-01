# Stack Overflow Issues in apollo_reserves

## Summary
Several functions exceed Solana's 4KB stack limit. These are warnings, not errors - the program compiles but may fail at runtime.

## Affected Functions

### Critical: CreateVaults (3344 bytes over)
- **Cause:** 12 accounts in single instruction, each with Anchor validation
- **Fix:** Split into smaller instructions:
  1. `create_vault_authority()` - create VaultAuthority PDA only
  2. `create_vault(tier: u8)` - create one vault at a time
- **Impact:** Cannot create all vaults in single transaction

### Moderate: InitializeReserves (208 bytes over)
- **Fix:** Use `Box<Account<>>` for large account fields or split initialization

### Minor: RouteContribution (8 bytes over)
- **Fix:** Minor refactoring of the struct

### External: spl_token_2022 (152 bytes over)
- **Cause:** Confidential transfer verification in SPL Token 2022
- **Fix:** This is in the SPL library, not our code. May need to avoid confidential transfers or wait for library fix.

## Recommendation
Refactor `CreateVaults` first as it's the most severe. Consider:
1. Create vaults individually via separate calls
2. Use a client-side batching approach
3. Or use `UncheckedAccount` with manual validation (less safe)

## Status
- [ ] CreateVaults split
- [ ] InitializeReserves optimization
- [ ] RouteContribution cleanup
