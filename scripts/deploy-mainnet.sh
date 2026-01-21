#!/bin/bash
# Apollo Care Protocol - Mainnet Deployment Script
# 
# ⚠️  CAUTION: This deploys to MAINNET. Ensure you have:
#   - Completed security audits
#   - Tested thoroughly on devnet
#   - Multi-sig wallet configured
#   - Sufficient SOL for deployment
#
# Usage: ./scripts/deploy-mainnet.sh

set -e

echo "🚀 Apollo Care Protocol - MAINNET Deployment"
echo "============================================="
echo ""
echo "⚠️  WARNING: You are deploying to MAINNET!"
echo ""

# Configuration
CLUSTER="mainnet-beta"
ANCHOR_PROVIDER_URL="https://api.mainnet-beta.solana.com"
WALLET_PATH="${WALLET_PATH:-~/.config/solana/mainnet-deployer.json}"

# Safety checks
read -p "Have you completed security audits? (yes/no): " AUDIT_CONFIRM
if [ "$AUDIT_CONFIRM" != "yes" ]; then
    echo "❌ Security audit required before mainnet deployment."
    exit 1
fi

read -p "Have you tested on devnet? (yes/no): " DEVNET_CONFIRM
if [ "$DEVNET_CONFIRM" != "yes" ]; then
    echo "❌ Devnet testing required before mainnet deployment."
    exit 1
fi

read -p "Are you sure you want to deploy to MAINNET? (DEPLOY/no): " FINAL_CONFIRM
if [ "$FINAL_CONFIRM" != "DEPLOY" ]; then
    echo "❌ Deployment cancelled."
    exit 1
fi

# Check prerequisites
command -v solana >/dev/null 2>&1 || { echo "❌ Solana CLI required"; exit 1; }
command -v anchor >/dev/null 2>&1 || { echo "❌ Anchor CLI required"; exit 1; }

echo ""
echo "📋 Configuration:"
echo "  Cluster: $CLUSTER"
echo "  Wallet: $WALLET_PATH"
echo ""

# Verify wallet exists
if [ ! -f "$WALLET_PATH" ]; then
    echo "❌ Wallet not found at $WALLET_PATH"
    echo "  Set WALLET_PATH environment variable or create the wallet."
    exit 1
fi

# Set Solana config
echo "🔧 Configuring Solana CLI..."
solana config set --url $ANCHOR_PROVIDER_URL
solana config set --keypair $WALLET_PATH

# Check balance (mainnet requires significant SOL)
BALANCE=$(solana balance | awk '{print $1}')
echo "💰 Wallet balance: $BALANCE SOL"

MIN_BALANCE=10
if (( $(echo "$BALANCE < $MIN_BALANCE" | bc -l) )); then
    echo "❌ Insufficient balance. Need at least $MIN_BALANCE SOL for mainnet deployment."
    exit 1
fi

# Build programs with release profile
echo "🔨 Building programs (release)..."
anchor build -- --release

# Verify builds
echo "🔍 Verifying program builds..."
for so in target/deploy/*.so; do
    echo "  ✓ $(basename $so)"
done

# Deploy with confirmation
echo ""
echo "📤 Deploying programs to mainnet..."
echo "  This may take several minutes..."
echo ""

anchor deploy --provider.cluster mainnet-beta

# Get program IDs and save to file
echo ""
echo "✅ MAINNET Deployment complete!"
echo ""
echo "📝 Program IDs (save these!):"
echo "================================" | tee deployment-mainnet.txt
date | tee -a deployment-mainnet.txt
echo "" | tee -a deployment-mainnet.txt

for idl in target/idl/*.json; do
    program_name=$(basename "$idl" .json)
    program_id=$(grep -A1 "\[$program_name\]" Anchor.toml | grep "address" | cut -d'"' -f2 2>/dev/null || echo "Not found")
    echo "  $program_name: $program_id" | tee -a deployment-mainnet.txt
done

echo ""
echo "🎉 Mainnet deployment successful!"
echo ""
echo "⚠️  IMPORTANT POST-DEPLOYMENT STEPS:"
echo "  1. Transfer upgrade authority to multi-sig"
echo "  2. Initialize protocol with governance parameters"
echo "  3. Set up monitoring and alerting"
echo "  4. Announce deployment to community"
echo "  5. Begin phased rollout"
