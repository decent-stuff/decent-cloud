#!/usr/bin/env node
/**
 * Validate build-time environment variables
 *
 * This script checks that required Vite environment variables are set
 * before building the website. It helps catch configuration errors early.
 */

import { config } from 'dotenv';
import { resolve } from 'path';
import { fileURLToPath } from 'url';
import { dirname } from 'path';

// Load .env.local file (created by deploy.py)
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const envPath = resolve(__dirname, '..', '.env.local');
config({ path: envPath });

// ANSI color codes
const RED = '\x1b[31m';
const YELLOW = '\x1b[33m';
const GREEN = '\x1b[32m';
const BLUE = '\x1b[34m';
const NC = '\x1b[0m';

function printError(message) {
    console.error(`${RED}✗${NC} ${message}`);
}

function printWarning(message) {
    console.warn(`${YELLOW}⚠${NC}  ${message}`);
}

function printSuccess(message) {
    console.log(`${GREEN}✓${NC} ${message}`);
}

function printInfo(message) {
    console.log(`${BLUE}→${NC} ${message}`);
}

// Check Stripe publishable key
const stripeKey = process.env.VITE_STRIPE_PUBLISHABLE_KEY;

if (!stripeKey) {
    console.log('');
    printWarning('VITE_STRIPE_PUBLISHABLE_KEY is not set');
    printWarning('Credit card payments will be DISABLED in the built website');
    console.log('');
    printInfo('To enable Stripe payments:');
    printInfo('  1. Get your Stripe key from: https://dashboard.stripe.com/apikeys');
    printInfo('  2. For dev/test: export VITE_STRIPE_PUBLISHABLE_KEY=pk_test_...');
    printInfo('  3. For production: export VITE_STRIPE_PUBLISHABLE_KEY=pk_live_...');
    printInfo('  4. Or add to website/.env.local before building');
    console.log('');
    printInfo('DCT token payments will still work without Stripe');
    console.log('');

    // Don't fail the build - allow deployment without Stripe
    // Users can still use DCT payments
    process.exit(0);
}

// Validate key format
if (!stripeKey.startsWith('pk_test_') && !stripeKey.startsWith('pk_live_')) {
    console.log('');
    printError('Invalid VITE_STRIPE_PUBLISHABLE_KEY format');
    printError(`Got: ${stripeKey.substring(0, 20)}...`);
    printError('Expected: pk_test_... or pk_live_...');
    console.log('');
    printInfo('Get your Stripe key from: https://dashboard.stripe.com/apikeys');
    console.log('');
    process.exit(1);
}

// Check key type matches expectations
const isTest = stripeKey.startsWith('pk_test_');
const isProduction = process.env.NODE_ENV === 'production';

if (isProduction && isTest) {
    console.log('');
    printWarning('Using TEST Stripe key in production build');
    printWarning('This will only accept test cards, not real payments');
    console.log('');
    printInfo('For production, use: export VITE_STRIPE_PUBLISHABLE_KEY=pk_live_...');
    console.log('');
    // Warning only, don't fail - sometimes you want to test in prod
}

if (!isProduction && !isTest) {
    console.log('');
    printWarning('Using LIVE Stripe key in development build');
    printWarning('This will charge REAL cards');
    console.log('');
    printInfo('For development, use: export VITE_STRIPE_PUBLISHABLE_KEY=pk_test_...');
    console.log('');
    // Warning only, don't fail
}

// Success
const keyType = isTest ? 'TEST' : 'LIVE';
printSuccess(`Stripe ${keyType} key configured (${stripeKey.substring(0, 20)}...)`);
process.exit(0);
