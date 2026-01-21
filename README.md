<p align="center">
  <img src="https://apollocare.io/logo.png" alt="Apollo Care Logo" width="200"/>
</p>

<h1 align="center">Apollo Care Protocol</h1>

<p align="center">
  <strong>Community-Owned Healthcare Coverage on Solana</strong><br/>
  <em>Transparent. Democratic. Affordable.</em>
</p>

<p align="center">
  <a href="https://github.com/ApolloCareIO/apollo-care-protocol/actions/workflows/ci.yml">
    <img src="https://github.com/ApolloCareIO/apollo-care-protocol/actions/workflows/ci.yml/badge.svg" alt="CI Status"/>
  </a>
  <a href="https://opensource.org/licenses/Apache-2.0">
    <img src="https://img.shields.io/badge/License-Apache%202.0-blue.svg" alt="License"/>
  </a>
  <a href="https://solana.com/">
    <img src="https://img.shields.io/badge/Solana-Mainnet-9945FF?logo=solana&logoColor=white" alt="Solana"/>
  </a>
  <a href="https://www.anchor-lang.com/">
    <img src="https://img.shields.io/badge/Anchor-0.30.1-ff69b4" alt="Anchor"/>
  </a>
  <a href="https://github.com/ApolloCareIO/apollo-care-protocol/stargazers">
    <img src="https://img.shields.io/github/stars/ApolloCareIO/apollo-care-protocol?style=social" alt="Stars"/>
  </a>
</p>

<p align="center">
  <a href="https://apollocare.io">🌐 Website</a> •
  <a href="https://docs.apollocare.io">📖 Docs</a> •
  <a href="https://x.com/apollocareio">𝕏 Twitter</a> •
  <a href="https://www.instagram.com/apollocare.io">📸 Instagram</a> •
  <a href="https://www.facebook.com/apollocare.io">📘 Facebook</a>
</p>

---

## The Problem

Healthcare in America is broken.

- **$4.9 trillion** spent annually on healthcare
- **40% wasted** on administrative overhead
- Insurance companies profit when you **don't** get care
- Average family pays **$24,000/year** for coverage they can't use

Traditional insurers have a fundamental conflict of interest: shareholders profit when claims are denied.

## The Solution

Apollo Care inverts the model entirely.

**Member-owned. Transparent. On-chain.**

We're building healthcare coverage as it should be—where the community owns the infrastructure, governance is democratic, and every dollar is tracked on an immutable ledger.

### Key Differentiators

| Traditional Insurance | Apollo Care |
|-----------------------|-------------|
| 80-85% Medical Loss Ratio (ACA minimum) | **90-95% MLR Target** |
| 15-20% to admin & profits | **<10% administrative costs** |
| Opaque claim decisions | **On-chain transparency** |
| Shareholder profits | **Member ownership (DAO)** |
| Days/weeks for claims | **Instant settlement** |
| Profit from denials | **Aligned incentives** |

---

## Architecture

Apollo Care Protocol consists of 7 interconnected Solana programs:

```
┌─────────────────────────────────────────────────────────────────┐
│                     APOLLO CARE PROTOCOL                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│                      ┌─────────────┐                            │
│                      │    CORE     │ ◀── APH Token-2022 Config  │
│                      │ (Shared)    │                            │
│                      └──────┬──────┘                            │
│                             │                                   │
│   ┌─────────────┐     ┌─────────────┐     ┌─────────────┐      │
│   │  GOVERNANCE │────▶│   STAKING   │────▶│  RESERVES   │      │
│   │    (DAO)    │     │  (3-Tier)   │     │  (3-Tier)   │      │
│   └──────┬──────┘     └─────────────┘     └──────┬──────┘      │
│          │                                       │              │
│          ▼                                       ▼              │
│   ┌─────────────┐     ┌─────────────┐     ┌─────────────┐      │
│   │    RISK     │────▶│ MEMBERSHIP  │────▶│   CLAIMS    │      │
│   │   ENGINE    │     │             │     │             │      │
│   └─────────────┘     └─────────────┘     └─────────────┘      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Programs

| Program | Description |
|---------|-------------|
| **apollo_core** | Shared constants, APH Token-2022 configuration, and utilities |
| **apollo_governance** | Wyoming DAO LLC structure with multi-committee governance |
| **apollo_staking** | Three-tier APH staking with risk-adjusted returns |
| **apollo_reserves** | USDC reserve management with IBNR and run-off provisions |
| **apollo_risk_engine** | CMS-compliant actuarial pricing and CAR calculations |
| **apollo_membership** | Member enrollment, contributions, and coverage lifecycle |
| **apollo_claims** | Claims processing with committee attestation |

---

## Tokenomics

**$APH Token** — 1 Billion Fixed Supply

| Allocation | Percentage | Tokens |
|------------|------------|--------|
| Community & Ecosystem Fund | 47% | 470,000,000 $APH |
| Core Team & Advisors | 22% | 220,000,000 $APH |
| Seed & Strategic Investors | 10% | 100,000,000 $APH |
| Insurance Reserve | 10% | 100,000,000 $APH |
| Liquidity & Exchanges | 6% | 60,000,000 $APH |
| Operations | 5% | 50,000,000 $APH |

### Token Info

| Property | Value |
|----------|-------|
| Token Name | Apollo Care |
| Symbol | $APH |
| Network | Solana |
| Program | **Token-2022** (Token Extensions) |
| Address | `6S3T6f1mmhxQWKR1gMiZR1SZLpu396jnnRMGqAZUj3Qj` |
| Decimals | 9 |
| Total Supply | 1,000,000,000 APH |

> **🔧 Token-2022**: APH uses Solana's Token-2022 program with Transfer Fee extension support. A 2% transfer fee will be enabled post-presale to fund protocol sustainability. See [`docs/TOKEN_2022_INTEGRATION.md`](docs/TOKEN_2022_INTEGRATION.md) for technical details.

### Staking Tiers

| Tier | APY Range | Max Loss | Lock Period |
|------|-----------|----------|-------------|
| 🛡️ Conservative | 3-5% | 2% | 30 days |
| ⚖️ Standard | 6-8% | 5% | 90 days |
| 🔥 Aggressive | 10-15% | 10% | 180 days |

### $APH Token Utility

- **Governance**: Vote on coverage policies, premium rates, committee elections, and protocol upgrades
- **Capital Staking**: Stake $APH to backstop the insurance pool and earn yield from member premiums
- **Community Incentives**: Earn $APH through referrals, wellness programs, and liquidity provision
- **Value Accrual**: Surplus from operations can fund buyback & burn or member rebates

> **Note**: Holding $APH is **not required** for coverage. Members pay premiums in USDC only. $APH is for governance and capital participation.

---

## Actuarial Framework

### CMS-Compliant Pricing

- **3:1 age ratio** compliance (ACA standard)
- **10 age bands** from 0-14 to 64+
- **Regional adjustments** for cost variation
- **Tobacco surcharge** capped at 1.2x

### Capital Adequacy Ratio (CAR)

```
CAR = (USDC Reserves + Eligible Staked APH) / Expected Annual Claims

Zone Thresholds:
🟢 Green  (>150%): Unlimited enrollment
🟡 Yellow (125-150%): Max 500 enrollments/month
🟠 Orange (100-125%): Max 100 enrollments/month  
🔴 Red    (<100%): Enrollment frozen
```

### Reserve Tiers

| Tier | Purpose | Description |
|------|---------|-------------|
| Tier 0 | Liquidity Buffer | Real-time claims fund, continuously refilled |
| Tier 1 | Operating Reserve | Absorbs normal claim volatility, months of coverage |
| Tier 2 | Contingent Capital | DAO Treasury + Staked $APH, emergency backstop |

**Claims Payment Waterfall**: Tier 0 → Tier 1 → Tier 2 → Staked APH Liquidation

---

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) 1.70+
- [Solana CLI](https://docs.solana.com/cli/install-solana-cli-tools) 1.18+
- [Anchor](https://www.anchor-lang.com/docs/installation) 0.30+
- [Node.js](https://nodejs.org/) 18+

### Installation

```bash
# Clone the repository
git clone https://github.com/ApolloCareIO/apollo-care-protocol.git
cd apollo-care-protocol

# Install dependencies
yarn install

# Build all programs
anchor build

# Run tests
anchor test

# Deploy to devnet
anchor deploy --provider.cluster devnet
```

### Project Structure

```
apollo-care-protocol/
├── programs/
│   ├── apollo_governance/    # DAO & committee governance
│   ├── apollo_staking/       # APH staking mechanics
│   ├── apollo_reserves/      # USDC reserve management
│   ├── apollo_risk_engine/   # Actuarial pricing & CAR
│   ├── apollo_claims/        # Claims processing
│   └── apollo_membership/    # Member lifecycle
├── tests/                    # Integration tests
├── migrations/               # Deployment scripts
└── sdk/                      # TypeScript SDK
```

---

## Roadmap

### Phase 1: Foundation ✅
- [x] Smart contract architecture
- [x] Actuarial model design
- [x] Tokenomics specification
- [x] Whitepaper publication

### Phase 2: Development 🔄
- [x] Core program implementation
- [ ] Security audits
- [ ] Testnet deployment
- [ ] Frontend development

### Phase 3: Launch 📅
- [ ] ICO on SolSale
- [ ] Mainnet deployment
- [ ] Initial enrollment window
- [ ] First claims processing

### Phase 4: Scale 🚀
- [ ] Multi-state licensing
- [ ] Reinsurance partnerships
- [ ] Provider network integration
- [ ] Mobile app release

---

## Security

Security is paramount for healthcare financial infrastructure.

### Measures Implemented

- ✅ Reentrancy protection on all state changes
- ✅ Emergency pause mechanisms
- ✅ Multi-signature requirements for treasury
- ✅ TWAP liquidation with circuit breakers
- ✅ Committee attestation for large claims

### Audits

Audit reports will be published here upon completion.

---

## Contributing

We welcome contributions from the community!

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

Please read our [Contributing Guidelines](CONTRIBUTING.md) before submitting.

---

## Community

Join the Apollo Care community:

- 🌐 **Website**: [apollocare.io](https://apollocare.io)
- 🐦 **Twitter/X**: [@apollocareio](https://x.com/apollocareio)
- 📸 **Instagram**: [@apollocare.io](https://www.instagram.com/apollocare.io)
- 📘 **Facebook**: [apollocare.io](https://www.facebook.com/apollocare.io)
- 💻 **GitHub**: [ApolloCareIO](https://github.com/ApolloCareIO)

---

## License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.

---

<p align="center">
  <img src="https://img.shields.io/badge/Built_with-❤️-red?style=for-the-badge" alt="Built with love"/>
</p>

<p align="center">
  <strong>💊 Most tokens promise moon. $APH promises your family can afford healthcare. 💊</strong>
</p>

<p align="center">
  <sub>© 2024-2025 Apollo Care Protocol. All rights reserved.</sub>
</p>

<p align="center">
  <a href="https://apollocare.io">
    <img src="https://img.shields.io/badge/Website-apollocare.io-00D4AA?style=flat-square&logo=safari&logoColor=white" alt="Website"/>
  </a>
  <a href="https://x.com/apollocareio">
    <img src="https://img.shields.io/badge/Twitter-@apollocareio-1DA1F2?style=flat-square&logo=twitter&logoColor=white" alt="Twitter"/>
  </a>
  <a href="https://www.instagram.com/apollocare.io">
    <img src="https://img.shields.io/badge/Instagram-@apollocare.io-E4405F?style=flat-square&logo=instagram&logoColor=white" alt="Instagram"/>
  </a>
  <a href="https://www.facebook.com/apollocare.io">
    <img src="https://img.shields.io/badge/Facebook-apollocare.io-1877F2?style=flat-square&logo=facebook&logoColor=white" alt="Facebook"/>
  </a>
  <a href="https://github.com/ApolloCareIO">
    <img src="https://img.shields.io/badge/GitHub-ApolloCareIO-181717?style=flat-square&logo=github&logoColor=white" alt="GitHub"/>
  </a>
</p>
# Apollo-Care-Protocol
