# Cloudflare Pages Deployment Setup

This document explains how to set up Cloudflare Pages deployment for the Decent Cloud website using GitHub Actions.

## Prerequisites

1. A Cloudflare account
2. A Cloudflare Pages project created
3. API credentials for deployment

## Required GitHub Secrets

To enable automatic deployment to Cloudflare Pages, you need to add the following secrets to your GitHub repository:

1. `CF_API_TOKEN` - Cloudflare API token with permissions to deploy to Pages
2. `CF_ACCOUNT_ID` - Your Cloudflare account ID

### Creating a Cloudflare API Token

1. Go to the [Cloudflare dashboard](https://dash.cloudflare.com/profile/api-tokens)
2. Click **Create Token**
3. Use the **API Token Template** for "Edit Cloudflare Workers"
   - If the template is not available, create a custom token with these permissions:
     - Permissions:
       - Account | Cloudflare Pages | Edit
       - Account | Workers R2 Storage | Read
     - Account Resources: Include your account
4. Click **Continue to summary**
5. Click **Create Token**
6. Copy the generated token

### Finding Your Account ID

1. Go to the [Cloudflare dashboard](https://dash.cloudflare.com/)
2. Select your account
3. The account ID is visible in the URL: `https://dash.cloudflare.com/{ACCOUNT_ID}`
4. Copy the account ID

### Adding Secrets to GitHub

1. Go to your GitHub repository
2. Navigate to **Settings** → **Secrets and variables** → **Actions**
3. Click **New repository secret**
4. Add the following secrets:
   - Name: `CF_API_TOKEN`
     - Value: Your Cloudflare API token
   - Name: `CF_ACCOUNT_ID`
     - Value: Your Cloudflare account ID

## Cloudflare Pages Project Setup

1. Go to the [Cloudflare dashboard](https://dash.cloudflare.com/)
2. Navigate to **Compute (Workers)**
3. Click **Create application** → **Pages** → **Connect to Git**
4. Connect your GitHub repository
5. Configure the build settings:
   - Framework preset: Next.js
   - Build command: `npx @cloudflare/next-on-pages@1`
   - Build output directory: `.vercel/output/static`
6. Click **Save and Deploy**

## Testing the Deployment

After setting up the secrets, the deployment workflow will automatically run on pushes to the main branch. You can also manually trigger the workflow from the GitHub Actions tab.

## Troubleshooting

If deployment fails, check:

1. That all required secrets are correctly set
2. That the Cloudflare API token has the necessary permissions
3. That the Cloudflare Pages project is correctly configured
4. The workflow logs for specific error messages