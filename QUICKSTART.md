# Apollo Care Protocol - Developer Quickstart

Get up and running with Apollo Care Protocol in under 10 minutes.

## Quick Overview

Apollo Care is a decentralized health coverage protocol built on Solana. It replaces traditional insurance with:
- **Member-owned infrastructure** (no corporate middlemen)
- **AI-driven claims processing** (fast, fair, transparent)
- **90%+ Medical Loss Ratio** (vs traditional ~82%)
- **DAO governance** (community control)

## Prerequisites

- **Node.js 18+**
- **Rust 1.75+** (for smart contract development)
- **Solana CLI**
- **Anchor 0.30+**

## 1. Clone & Install

```bash
git clone https://github.com/ApolloCareIO/apollo-care-protocol.git
cd apollo-care-protocol
npm install
```

## 2. Build Smart Contracts

```bash
# Install Rust dependencies
cargo build

# Build Anchor programs
anchor build
```

## 3. Run Tests

```bash
# Start local validator
solana-test-validator

# In another terminal, run tests
anchor test
```

## 4. Deploy to Devnet

```bash
# Configure Solana CLI for devnet
solana config set --url devnet

# Airdrop SOL for deployment
solana airdrop 5

# Deploy
anchor deploy
```

## SDK Quick Start

### Installation

```bash
npm install @apollocare/sdk
```

### Basic Usage

```typescript
import { ApolloClient, CoverageTier } from '@apollocare/sdk';

// Initialize client
const client = new ApolloClient({
  endpoint: 'https://api.devnet.solana.com',
  wallet: yourWallet,
});

// Check enrollment status
const isOpen = await client.isEnrollmentOpen();

// Calculate contribution
const contribution = await client.calculateContribution({
  age: 35,
  tobaccoUser: false,
  dependentCount: 2,
  coverageTier: CoverageTier.Gold,
});

// Enroll
await client.enroll(enrollmentParams);

// Submit claim
await client.submitClaim({
  amount: 500_000000, // $500 USDC
  claimType: ClaimType.Preventive,
  serviceDate: new Date(),
  providerId: 'NPI123456',
  documentHash: 'QmXyz...',
});
```

## Architecture at a Glance

```
┌────────────────────────────────────────────────────────────────┐
│                     Apollo Care Protocol                       │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐        │
│  │Membership│  │  Claims  │  │ Staking  │  │Governance│        │
│  │  Program │  │  Program │  │  Program │  │  Program │        │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘        │
│       │             │             │             │              │
│  ┌────▼─────────────▼─────────────▼─────────────▼────┐         │
│  │                    Core Program                   │         │
│  │              (State Coordination)                 │         │
│  └────┬─────────────┬─────────────┬─────────────┬────┘         │
│       │             │             │             │              │
│  ┌────▼─────┐  ┌────▼──────┐  ┌────▼──────┐  ┌────▼─────┐      │
│  │ Reserves │  │Risk Engine│  │Reinsurance│  │  Oracle  │      │
│  │  Program │  │  Program  │  │  Program  │  │  System  │      │
│  └──────────┘  └───────────┘  └───────────┘  └──────────┘      │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

## Key Concepts

### Capital Adequacy Ratio (CAR)

Protocol health metric that controls enrollment:

| Zone     | CAR         | Enrollment Status    |
|----------|-------------|----------------------|
| 🟢 Green  | ≥150%    | Unlimited              |
| 🟡 Yellow | 125-150% | Max 500/month          |
| 🟠 Orange | 100-125% | Max 100/month          |
| 🔴 Red    | <100%    | Frozen                 |

### Reserve Tiers

| Tier | Purpose           | Target Coverage |
|------|-------------------|-----------------|
| 0    | Liquidity Buffer  | 30 days         |
| 1    | Operating + IBNR  | 60 days         |
| 2    | Contingent Capital| 180 days        |

### Claims Processing

| Tier      | Amount      | Processing | Decision By   |
|-----------|-------------|------------|---------------|
| Fast-Lane | <$500       | <100ms     | Fast-approval |
| AI Review | $500-$5K    | <24 hours  | AI + Human    |
| Committee | >$5K        | 3-5 days   | DAO Committee |

### Staking Tiers

| Tier         | APY     | Max Loss | Lock Period |
|--------------|---------|----------|-------------|
| Conservative | 3-5%    | 2%       | 30 days     |
| Standard     | 6-8%    | 5%       | 90 days     |
| Aggressive   | 10-15%  | 10%      | 180 days    |

## Project Structure

```
apollo-care-protocol/
├── programs/                    # Anchor smart contracts
│   ├── apollo_membership/       # Enrollment & member management
│   ├── apollo_claims/           # Claims processing & AI oracle
│   ├── apollo_staking/          # APH staking mechanics
│   ├── apollo_reserves/         # Reserve management
│   ├── apollo_risk_engine/      # CAR & pricing
│   ├── apollo_governance/       # DAO voting
│   ├── apollo_reinsurance/      # External risk transfer
│   └── apollo_core/             # State coordination
├── sdk/                         # TypeScript SDK
├── tests/                       # Integration tests
├── docs/                        # Documentation
└── examples/                    # Usage examples
```

## Useful Commands

```bash
# Development
anchor build                  # Build all programs
anchor test                   # Run test suite
npm run lint                  # Lint code
npm run format                # Format code

# Deployment
./scripts/deploy-devnet.sh    # Deploy to devnet
./scripts/deploy-mainnet.sh   # Deploy to mainnet

# SDK
cd sdk && npm run build       # Build SDK
cd sdk && npm run test        # Test SDK
```

## Resources

- **Full Documentation**: [apollocare.io/documentation](https://apollocare.io/documentation)
- **Architecture Guide**: [ARCHITECTURE.md](./ARCHITECTURE.md)
- **AI Integration**: [docs/AI_OPTIMIZED_ARCHITECTURE.md](./docs/AI_OPTIMIZED_ARCHITECTURE.md)
- **SDK Examples**: [examples/](./examples/)

## Community

- **Discord**: [discord.gg/apollocare](https://discord.gg/apollocare.io)
- **Twitter**: [@ApolloCareIO](https://twitter.com/ApolloCareIO)
- **Email**: [contact@apollocare.io]

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines.

## Security

Found a vulnerability? See [SECURITY.md](./SECURITY.md) for responsible disclosure.

---

**Ready to build the future of healthcare?** Start with the [examples](./examples/) or dive into the [architecture docs](./ARCHITECTURE.md).
