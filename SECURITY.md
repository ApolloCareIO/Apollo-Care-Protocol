# Security Policy

## Reporting a Vulnerability

The Apollo Care team takes security seriously. We appreciate your efforts to responsibly disclose your findings.

### How to Report

**DO NOT** create a public GitHub issue for security vulnerabilities.

Instead, please report security vulnerabilities via email to:
- **Email**: security@apollocare.io

### What to Include

Please include the following in your report:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

### Response Timeline

- **Initial Response**: Within 48 hours
- **Status Update**: Within 7 days
- **Resolution Target**: Within 30 days (depending on severity)

### Scope

The following are in scope:
- All smart contracts in `/programs/`
- Token handling and transfers
- Access control mechanisms
- Reserve management logic
- Claims processing logic

### Out of Scope

- Frontend/UI vulnerabilities (report separately)
- Social engineering attacks
- Physical security
- Third-party dependencies (report to maintainers)

### Recognition

We believe in recognizing security researchers for their contributions:
- Hall of Fame recognition
- Potential bug bounty (coming soon)
- Public acknowledgment (with permission)

### Safe Harbor

We will not pursue legal action against researchers who:
- Follow this responsible disclosure policy
- Make good faith efforts to avoid privacy violations
- Do not exploit vulnerabilities beyond proof of concept
- Do not disrupt services or destroy data

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Security Measures

### Smart Contract Security

- Reentrancy guards on all state-changing functions
- Input validation on all external calls
- Emergency pause functionality
- Multi-signature requirements for treasury operations
- TWAP liquidation with circuit breakers

### Operational Security

- Multi-sig wallets for deployment keys
- Regular access reviews
- Audit trail for all privileged actions

## Known Dependency Vulnerabilities

The following are **transitive dependency vulnerabilities** from the Solana/Anchor ecosystem that we cannot directly fix. These are tracked upstream by the Solana team:

| Advisory | Crate | Source | Status |
|----------|-------|--------|--------|
| RUSTSEC-2024-0344 | curve25519-dalek | solana-zk-token-sdk | Awaiting Solana update |
| RUSTSEC-2022-0093 | ed25519-dalek | solana-sdk | Awaiting Solana update |
| RUSTSEC-2024-0375 | atty | solana-logger | Unmaintained (warning) |
| RUSTSEC-2025-0141 | bincode | solana-program | Unmaintained (warning) |
| RUSTSEC-2024-0388 | derivative | ark-* crates | Unmaintained (warning) |
| RUSTSEC-2024-0436 | paste | ark-ff | Unmaintained (warning) |
| RUSTSEC-2021-0145 | atty | solana-logger | Unsound (warning) |
| RUSTSEC-2023-0033 | borsh | solana-program | Unsound (warning) |

**Note**: These vulnerabilities are in dependencies pulled in by `anchor-lang 0.30.1` and `solana-program 1.18`. They affect the entire Solana ecosystem and will be resolved when Solana Labs updates their dependencies. Our smart contract code does not directly use these vulnerable APIs.

Thank you for helping keep Apollo Care secure! 🛡️
