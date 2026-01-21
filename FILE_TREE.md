# Apollo Care Protocol - File Structure

```
apollo-care-protocol/
├── .github/
│   ├── CODEOWNERS                    # Code ownership rules
│   ├── dependabot.yml               # Dependency updates config
│   ├── mlc_config.json              # Markdown link checker config
│   ├── ISSUE_TEMPLATE/
│   │   ├── bug_report.yml           # Bug report template
│   │   └── feature_request.yml      # Feature request template
│   ├── PULL_REQUEST_TEMPLATE.md     # PR template
│   └── workflows/
│       ├── ci.yml                   # CI pipeline
│       └── release.yml              # Release automation
│
├── docs/
│   ├── ACTUARIAL_ALIGNMENT.md       # Actuarial standards documentation
│   ├── AI_OPTIMIZED_ARCHITECTURE.md # AI/ML integration details
│   ├── BOOTSTRAP_VIABILITY_ANALYSIS.md
│   ├── BOOTSTRAP_VIABILITY_SPEC.md
│   ├── IMPLEMENTATION_CHANGES_SUMMARY.md
│   ├── IMPLEMENTATION_STATUS.md
│   ├── IMPLEMENTATION_SUMMARY.md
│   ├── SMALL_SCALE_VIABILITY_AUDIT.md
│   └── TOKEN_2022_INTEGRATION.md    # Token-2022 technical details
│
├── programs/
│   ├── apollo_core/                 # Shared infrastructure
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs               # Main entry point
│   │       ├── constants.rs         # Protocol constants
│   │       ├── errors.rs            # Error definitions
│   │       ├── events.rs            # Event emissions
│   │       └── state.rs             # Account structures
│   │
│   ├── apollo_governance/           # DAO governance
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── instructions/
│   │       └── state.rs
│   │
│   ├── apollo_staking/              # APH staking (3-tier)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── instructions/
│   │       └── state.rs
│   │
│   ├── apollo_reserves/             # USDC reserves (3-tier + IBNR)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── instructions/
│   │       └── state.rs
│   │
│   ├── apollo_risk_engine/          # Actuarial pricing & CAR
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── instructions/
│   │       └── state.rs
│   │
│   ├── apollo_membership/           # Member enrollment
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── instructions/
│   │       └── state.rs
│   │
│   ├── apollo_claims/               # Claims processing
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── instructions/
│   │       └── state.rs
│   │
│   └── apollo_reinsurance/          # Reinsurance management
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── instructions/
│           └── state.rs
│
├── sdk/                             # TypeScript SDK
│   ├── package.json
│   ├── README.md
│   ├── tsconfig.json
│   └── src/
│       ├── index.ts                 # Main exports
│       ├── client.ts                # ApolloClient class
│       ├── types.ts                 # TypeScript types
│       ├── constants.ts             # Protocol constants
│       └── utils.ts                 # Utility functions
│
├── scripts/
│   ├── deploy-devnet.sh             # Devnet deployment
│   └── deploy-mainnet.sh            # Mainnet deployment
│
├── tests/
│   ├── apollo_claims.ts
│   ├── apollo_governance.ts
│   ├── apollo_membership.ts
│   ├── apollo_reinsurance.ts
│   ├── apollo_reserves.ts
│   ├── apollo_risk_engine.ts
│   ├── apollo_staking.ts
│   ├── bootstrap_viability.ts       # Bootstrap scenario tests
│   ├── integration.ts               # Full integration tests
│   └── utils/
│       └── test-helpers.ts
│
├── migrations/
│   └── deploy.ts                    # Anchor migrations
│
├── .gitignore
├── Anchor.toml                      # Anchor configuration
├── ARCHITECTURE.md                  # System architecture
├── Cargo.toml                       # Rust workspace
├── CHANGELOG.md                     # Version history
├── CONTRIBUTING.md                  # Contribution guide
├── docker-compose.yml               # Development environment
├── Dockerfile                       # Development container
├── LICENSE                          # Apache 2.0
├── package.json                     # Node.js dependencies
├── README.md                        # Main documentation
├── SECURITY.md                      # Security policy
└── tsconfig.json                    # TypeScript config
```

## Key Files

| File | Purpose |
|------|---------|
| `ARCHITECTURE.md` | System design and component overview |
| `COMPREHENSIVE_GAP_ANALYSIS.md` | Actuarial review with Morrisey citations |
| `docs/AI_OPTIMIZED_ARCHITECTURE.md` | AI/ML integration details |
| `docs/BOOTSTRAP_VIABILITY_SPEC.md` | Bootstrap phase specifications |
| `docs/TOKEN_2022_INTEGRATION.md` | APH token technical details |
| `sdk/src/client.ts` | TypeScript client for protocol interaction |

## Program IDs (Devnet)

| Program | Address |
|---------|---------|
| apollo_core | `ApLCoRECxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx` |
| apollo_governance | `ApLGoVxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx` |
| apollo_staking | `ApLStKxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx` |
| apollo_reserves | `ApLRsVxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx` |
| apollo_risk_engine | `ApLRsKxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx` |
| apollo_membership | `ApLMbRxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx` |
| apollo_claims | `ApLClMxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx` |
| apollo_reinsurance | `ApLReIxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx` |

> **Note**: Replace with actual deployed addresses after deployment.

## APH Token

| Property | Value |
|----------|-------|
| Mint | `6S3T6f1mmhxQWKR1gMiZR1SZLpu396jnnRMGqAZUj3Qj` |
| Program | Token-2022 |
| Decimals | 9 |
| Total Supply | 1,000,000,000 APH |
| Transfer Fee | 2% (post-presale) |
