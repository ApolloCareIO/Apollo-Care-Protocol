# @apollocare/sdk

TypeScript SDK for interacting with Apollo Care Protocol on Solana.

## Installation

```bash
npm install @apollocare/sdk
# or
yarn add @apollocare/sdk
# or
pnpm add @apollocare/sdk
```

## Quick Start

```typescript
import { ApolloClient, createDevnetClient, CoverageTier } from '@apollocare/sdk';
import { Keypair } from '@solana/web3.js';

// Create a client
const wallet = Keypair.generate(); // Use your actual wallet
const client = createDevnetClient({
  publicKey: wallet.publicKey,
  signTransaction: async (tx) => {
    tx.sign(wallet);
    return tx;
  },
  signAllTransactions: async (txs) => {
    txs.forEach(tx => tx.sign(wallet));
    return txs;
  },
});

// Check enrollment status
const isEnrolled = await client.isEnrolled(wallet.publicKey);
console.log('Enrolled:', isEnrolled);

// Calculate contribution
const contribution = await client.calculateContribution({
  coverageTier: CoverageTier.Silver,
  age: 35,
  regionCode: 1,
  tobaccoUser: false,
  dependentCount: 2,
});
console.log('Monthly contribution:', formatUsdc(contribution));

// Enroll (if not already enrolled)
if (!isEnrolled) {
  const result = await client.enroll({
    coverageTier: CoverageTier.Silver,
    age: 35,
    regionCode: 1,
    tobaccoUser: false,
    dependentCount: 2,
  });
  console.log('Enrolled:', result.signature);
}
```

## Features

### Membership

```typescript
// Get member account
const member = await client.getMember(walletAddress);

// Calculate contribution
const contribution = await client.calculateContribution(params);

// Enroll new member
const result = await client.enroll(params);

// Pay monthly contribution
await client.payContribution();
```

### Claims

```typescript
// Submit a claim
const result = await client.submitClaim({
  claimType: ClaimType.Medical,
  amountRequested: parseUsdc('1500.00'),
  serviceDate: Math.floor(Date.now() / 1000),
  procedureCodes: ['99213'],
  diagnosisCodes: ['J06.9'],
});

// Get claim status
const claim = await client.getClaim(claimId);

// Appeal denied claim
await client.appealClaim(claimId, 'Reason for appeal');
```

### Staking

```typescript
// Stake APH tokens
await client.stake({
  amount: parseAph('10000'),
  tier: StakingTier.Standard,
});

// Get stake info
const stake = await client.getStake(walletAddress);

// Claim rewards
await client.claimRewards();

// Unstake (if unlocked)
await client.unstake(parseAph('5000'));
```

### Protocol State

```typescript
// Get current CAR zone
const zone = await client.getCurrentCarZone();

// Check if enrollment is open
const isOpen = await client.isEnrollmentOpen();

// Get reserve state
const reserves = await client.getReserveState();

// Get risk engine state
const risk = await client.getRiskEngineState();
```

## Utilities

```typescript
import {
  formatAph,
  formatUsdc,
  parseAph,
  parseUsdc,
  bpsToPercent,
  getCarZone,
  calculateContribution,
  calculateIbnr,
} from '@apollocare/sdk';

// Format token amounts
console.log(formatAph(new BN('1000000000000'))); // "1000.0000"
console.log(formatUsdc(new BN('1500000000'))); // "$1500.00"

// Parse token amounts
const aphAmount = parseAph('1000');
const usdcAmount = parseUsdc('500.00');

// Basis points
console.log(bpsToPercent(9000)); // "90.00%"

// CAR zone
const zone = getCarZone(15500); // CarZone.Green

// Premium calculation
const contribution = calculateContribution(
  parseUsdc('400'), // base rate
  35,               // age
  false,            // tobacco
  2,                // dependents
  CoverageTier.Silver,
  10000             // shock factor (1.0x)
);
```

## Constants

```typescript
import {
  APH_MINT,
  TOTAL_SUPPLY,
  TOKEN_ALLOCATIONS,
  CAR_ZONES,
  STAKING_TIERS,
  COVERAGE_TIERS,
  TARGET_MLR_BPS,
} from '@apollocare/sdk';

// Token info
console.log(APH_MINT.toBase58()); // "6S3T6f1mmhxQWKR1gMiZR1SZLpu396jnnRMGqAZUj3Qj"

// Allocations
console.log(TOKEN_ALLOCATIONS.COMMUNITY_ECOSYSTEM); // 4700 (47%)

// CAR thresholds
console.log(CAR_ZONES.GREEN_THRESHOLD); // 15000 (150%)

// Coverage tiers
console.log(COVERAGE_TIERS.GOLD.DEDUCTIBLE_USDC); // $1,500
```

## Types

Full TypeScript support with comprehensive type definitions:

```typescript
import type {
  MemberAccount,
  ClaimAccount,
  StakeAccount,
  ReserveState,
  RiskEngineState,
  EnrollmentParams,
  ClaimSubmissionParams,
  StakeParams,
  CoverageTier,
  ClaimStatus,
  StakingTier,
  CarZone,
} from '@apollocare/sdk';
```

## Networks

```typescript
import { NETWORKS, createDevnetClient, createMainnetClient } from '@apollocare/sdk';

// Devnet (for testing)
const devnetClient = createDevnetClient(wallet);

// Mainnet (for production)
const mainnetClient = createMainnetClient(wallet);

// Custom endpoint
const client = new ApolloClient({
  endpoint: 'https://my-rpc.example.com',
  wallet,
  commitment: 'confirmed',
});
```

## Error Handling

```typescript
import { ApolloErrorCode, ERROR_MESSAGES } from '@apollocare/sdk';

try {
  await client.submitClaim(params);
} catch (error) {
  if (error.code === ApolloErrorCode.MemberNotActive) {
    console.log(ERROR_MESSAGES.MEMBER_NOT_ACTIVE);
  }
}
```

## Documentation

- [Apollo Care Protocol Documentation](https://docs.apollocare.io)
- [API Reference](https://docs.apollocare.io/sdk)
- [GitHub Repository](https://github.com/ApolloCareIO/apollo-care-protocol)

## License

Apache-2.0
