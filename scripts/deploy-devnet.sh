#!/bin/bash
# Apollo Care Protocol - Devnet Deployment Script
# Usage: ./scripts/deploy-devnet.sh

set -e

echo "🚀 Apollo Care Protocol - Devnet Deployment"
echo "============================================"

# Configuration
CLUSTER="devnet"
ANCHOR_PROVIDER_URL="https://api.devnet.solana.com"
WALLET_PATH="${WALLET_PATH:-~/.config/solana/id.json}"

# Check prerequisites
command -v solana >/dev/null 2>&1 || { echo "❌ Solana CLI required"; exit 1; }
command -v anchor >/dev/null 2>&1 || { echo "❌ Anchor CLI required"; exit 1; }

echo "📋 Configuration:"
echo "  Cluster: $CLUSTER"
echo "  Wallet: $WALLET_PATH"
echo ""

# Set Solana config
echo "🔧 Configuring Solana CLI..."
solana config set --url $ANCHOR_PROVIDER_URL
solana config set --keypair $WALLET_PATH

# Check balance
BALANCE=$(solana balance | awk '{print $1}')
echo "💰 Wallet balance: $BALANCE SOL"

if (( $(echo "$BALANCE < 5" | bc -l) )); then
    echo "⚠️  Low balance. Requesting airdrop..."
    solana airdrop 2
    sleep 5
fi

# Build programs
echo "🔨 Building programs..."
anchor build

# Deploy
echo "📤 Deploying programs to devnet..."
anchor deploy --provider.cluster devnet

# Get program IDs
echo ""
echo "✅ Deployment complete!"
echo ""
echo "📝 Program IDs:"
for idl in target/idl/*.json; do
    program_name=$(basename "$idl" .json)
    # Extract program ID from Anchor.toml
    program_id=$(grep -A1 "\[$program_name\]" Anchor.toml | grep "address" | cut -d'"' -f2 2>/dev/null || echo "Not found")
    echo "  $program_name: $program_id"
done

echo ""
echo "🎉 Devnet deployment successful!"
echo ""
echo "Next steps:"
echo "  1. Run tests: anchor test --provider.cluster devnet"
echo "  2. Initialize protocol: npx ts-node scripts/initialize.ts"
echo "  3. Update frontend config with new program IDs"
