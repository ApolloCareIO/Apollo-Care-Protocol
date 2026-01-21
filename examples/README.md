# Apollo Care Protocol Examples

This directory contains example scripts demonstrating how to interact with the Apollo Care Protocol.

## Prerequisites

1. **Node.js** (v18+)
2. **Solana CLI** (optional, for key management)
3. **USDC tokens** (for enrollment/claims on mainnet)
4. **APH tokens** (for staking/governance)

## Setup

```bash
# Install dependencies
npm install @apollocare/sdk @solana/web3.js @coral-xyz/anchor

# Or with yarn
yarn add @apollocare/sdk @solana/web3.js @coral-xyz/anchor
```

## Examples

### 01 - Enrollment

Demonstrates the complete member enrollment flow:
- Connect to Solana network
- Check enrollment eligibility (CAR zone)
- Calculate monthly contribution
- Complete enrollment transaction

```bash
npx ts-node 01-enrollment.ts
```

### 02 - Claims

Shows how to submit and track claims across all processing tiers:
- Fast-lane claims (<$500, auto-approval)
- AI review claims ($500-$5,000)
- Committee review claims (>$5,000)
- Appeal process for denied claims

```bash
npx ts-node 02-claims.ts
```

### 03 - Staking

Explains APH token staking mechanics:
- Three-tier risk/reward profiles
- Staking and unstaking operations
- Reward claiming
- Voting power calculation

```bash
npx ts-node 03-staking.ts
```

### 04 - Protocol Monitor

Build a real-time protocol health dashboard:
- Capital Adequacy Ratio (CAR) monitoring
- Reserve tier balances
- Risk metrics and shock factor
- Event subscriptions

```bash
npx ts-node 04-protocol-monitor.ts
```

## Network Configuration

By default, examples connect to **Devnet**. For mainnet:

```typescript
import { NETWORKS, createMainnetClient } from '@apollocare/sdk';

const client = createMainnetClient(wallet);
```

## Environment Variables

For production use, set these environment variables:

```bash
# Solana RPC endpoint (optional, uses default if not set)
SOLANA_RPC_URL=https://your-rpc-endpoint.com

# Wallet keypair path (for signing transactions)
WALLET_KEYPAIR=/path/to/keypair.json
```

## Common Patterns

### Wallet Integration

```typescript
import { useWallet } from '@solana/wallet-adapter-react';
import { ApolloClient } from '@apollocare/sdk';

function MyComponent() {
  const wallet = useWallet();
  
  const client = new ApolloClient({
    endpoint: 'https://api.mainnet-beta.solana.com',
    wallet: wallet,
  });
  
  // Use client...
}
```

### Error Handling

```typescript
try {
  const result = await client.submitClaim(claimParams);
} catch (error) {
  if (error.code === 6001) {
    console.log('Not an active member');
  } else if (error.code === 6100) {
    console.log('Claim amount exceeds coverage');
  }
  // Handle other errors...
}
```

### Event Subscriptions

```typescript
import { Connection } from '@solana/web3.js';
import { PROGRAM_IDS } from '@apollocare/sdk';

const connection = new Connection('https://api.devnet.solana.com');

// Subscribe to claim events
const subscriptionId = connection.onLogs(
  new PublicKey(PROGRAM_IDS.APOLLO_CLAIMS),
  (logs) => {
    console.log('Claim event:', logs);
  }
);

// Clean up
connection.removeOnLogsListener(subscriptionId);
```

## Support

- **Documentation**: [docs.apollocare.io](https://docs.apollocare.io)
- **Discord**: [discord.gg/apollocare](https://discord.gg/apollocare)
- **Issues**: [GitHub Issues](https://github.com/ApolloCareIO/apollo-care-protocol/issues)
