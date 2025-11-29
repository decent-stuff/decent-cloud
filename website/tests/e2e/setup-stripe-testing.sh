#!/bin/bash
# Setup script for Stripe E2E testing
# Usage: ./setup-stripe-testing.sh [your_stripe_secret_key] [your_stripe_publishable_key]
#    Or: ./setup-stripe-testing.sh (will prompt for keys)

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
WEBSITE_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
API_DIR="$(cd "$WEBSITE_DIR/../api" && pwd)"

echo "🔧 Setting up Stripe E2E testing environment"
echo ""

# Check if keys provided as arguments
if [ -n "$1" ] && [ -n "$2" ]; then
    STRIPE_SECRET_KEY="$1"
    STRIPE_PUBLISHABLE_KEY="$2"
    echo "✅ Using provided Stripe keys from command line"
    REAL_KEYS=true
else
    # Prompt for keys interactively
    echo "Get your test keys from: https://dashboard.stripe.com/test/apikeys"
    echo ""

    read -p "Enter your Stripe SECRET key (sk_test_...): " STRIPE_SECRET_KEY
    read -p "Enter your Stripe PUBLISHABLE key (pk_test_...): " STRIPE_PUBLISHABLE_KEY

    # Validate keys
    if [[ -z "$STRIPE_SECRET_KEY" ]] || [[ -z "$STRIPE_PUBLISHABLE_KEY" ]]; then
        echo ""
        echo "❌ ERROR: Both keys are required!"
        echo "   Please run again and provide your Stripe test keys"
        exit 1
    fi

    if [[ ! "$STRIPE_SECRET_KEY" =~ ^sk_test_ ]]; then
        echo ""
        echo "⚠️  WARNING: Secret key should start with 'sk_test_' for test mode"
        read -p "Continue anyway? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 1
        fi
    fi

    if [[ ! "$STRIPE_PUBLISHABLE_KEY" =~ ^pk_test_ ]]; then
        echo ""
        echo "⚠️  WARNING: Publishable key should start with 'pk_test_' for test mode"
        read -p "Continue anyway? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 1
        fi
    fi

    echo ""
    echo "✅ Keys validated"
    REAL_KEYS=true
fi

STRIPE_WEBHOOK_SECRET="whsec_test_secret"

# Create API .env
echo "📝 Creating API .env at $API_DIR/.env"
cat > "$API_DIR/.env" << EOF
# Database Configuration
DATABASE_URL=sqlite:./data/ledger.db?mode=rwc

# Frontend URL (required for recovery emails)
FRONTEND_URL=http://localhost:59000

# Stripe Payment Configuration
STRIPE_SECRET_KEY=$STRIPE_SECRET_KEY
STRIPE_PUBLISHABLE_KEY=$STRIPE_PUBLISHABLE_KEY

# Stripe Webhook Secret (for local testing)
STRIPE_WEBHOOK_SECRET=$STRIPE_WEBHOOK_SECRET
EOF

# Create website .env
echo "📝 Creating website .env at $WEBSITE_DIR/.env"
cat > "$WEBSITE_DIR/.env" << EOF
# Development/staging API endpoint
VITE_DECENT_CLOUD_API_URL=http://localhost:59001

# Stripe API publishable key (for frontend)
VITE_STRIPE_PUBLISHABLE_KEY=$STRIPE_PUBLISHABLE_KEY
EOF

echo ""
echo "✅ Environment files created with your Stripe test keys!"
echo ""
echo "🚀 To start testing:"
echo ""
echo "   Terminal 1 (API Server):"
echo "   cd $API_DIR"
echo "   cargo run --bin api-server"
echo ""
echo "   Terminal 2 (Website - if not running):"
echo "   cd $WEBSITE_DIR"
echo "   npm run dev"
echo ""
echo "   Terminal 3 (Run Tests):"
echo "   cd $WEBSITE_DIR"
echo "   npx playwright test tests/e2e/payment-flows.spec.ts"
echo ""

# Check if API server is already running
if curl -s http://localhost:59001/health > /dev/null 2>&1; then
    echo "✅ API server is already running on http://localhost:59001"
else
    echo "⚠️  API server is NOT running. Start it with: cd $API_DIR && cargo run --bin api-server"
fi

# Check if website is running
if curl -s http://localhost:59000 > /dev/null 2>&1; then
    echo "✅ Website is already running on http://localhost:59000"
else
    echo "⚠️  Website is NOT running. Start it with: cd $WEBSITE_DIR && npm run dev"
fi

echo ""
echo "📚 For more details, see: $WEBSITE_DIR/tests/e2e/STRIPE_TESTING_SETUP.md"
