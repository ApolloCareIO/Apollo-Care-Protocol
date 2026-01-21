# Changelog

All notable changes to the Apollo Care Protocol will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Bootstrap viability analysis for small-scale launch
- Token-2022 integration with 2% transfer fee
- Three-tier reserve system (Tier-0, Tier-1, Tier-2)
- AI-driven claims processing with fast-lane auto-approval
- Reinsurance program with specific and aggregate stop-loss
- CAR-based enrollment gating (Green/Yellow/Orange/Red zones)
- Phase management for HCSM → Insurance transition

### Changed
- Updated token allocations to official spec (47/22/10/10/6/5)
- Increased ShockFactor maximum to 2.0x for catastrophic scenarios
- Standardized reserve tier definitions per actuarial spec

### Fixed
- Corrected gap analysis document token allocation percentages
- Fixed IBNR calculation methodology

## [0.1.0] - 2026-01-20

### Added
- Initial release of Apollo Care Protocol smart contracts
- 8 interconnected Anchor programs:
  - `apollo_core` - Core constants, Token-2022 integration
  - `apollo_membership` - Member enrollment and contributions
  - `apollo_claims` - Claims submission and processing
  - `apollo_reserves` - Three-tier reserve management
  - `apollo_reinsurance` - Stop-loss coverage
  - `apollo_risk_engine` - CAR zones and rating
  - `apollo_staking` - APH staking with tiered risk
  - `apollo_governance` - Multisig and committees
- Comprehensive test suite (6,300+ lines)
- Full documentation suite
- CI/CD pipeline with GitHub Actions

### Technical Specifications
- **MLR Target**: 90%+ (exceeds ACA 80-85% minimum)
- **Loading Percentage**: 10% (8% admin + 2% reserve)
- **CAR Target**: 125%
- **Token Supply**: 1,000,000,000 APH (fixed)
- **Mint Address**: `6S3T6f1mmhxQWKR1gMiZR1SZLpu396jnnRMGqAZUj3Qj`

### Actuarial Alignment
- Premium computation per Morrisey formula
- Objective risk formula: σ/(μ√N)
- Stop-loss coverage: Specific ($50k) + Aggregate (110%)
- CMS-compliant 3:1 age band ratio

---

## Version History

| Version | Date | Description |
|---------|------|-------------|
| 0.1.0 | 2026-01-20 | Initial release |

---

## Legend

- **Added** - New features
- **Changed** - Changes in existing functionality
- **Deprecated** - Soon-to-be removed features
- **Removed** - Removed features
- **Fixed** - Bug fixes
- **Security** - Vulnerability fixes
