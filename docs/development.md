# Development Guide

This guide covers everything you need to know about developing and contributing to the Decent Cloud project.

Note that the Decent Cloud project is split across three repositories:

- **LedgerMap**: The backing ledger implementation

  - GitHub: [github.com/decent-stuff/ledger-map](https://github.com/decent-stuff/ledger-map/)
  - NPM Package: [@decent-stuff/ledger-map](https://www.npmjs.com/package/@decent-stuff/ledger-map)

- **Decent-Cloud**: This repository, Internet Computer canister and the client code

  - Console Client: Available in this repository under `/cli` and published on GitHub as release binaries
  - Browser Client: [@decent-stuff/dc-client](https://www.npmjs.com/package/@decent-stuff/dc-client) NPM package

- **Frontend Website**: The official Decent Cloud web interface (located in `/website` directory)

## Development Environment Setup

### Prerequisites

1. **Essential Build Tools**

```bash
sudo apt update -y
sudo apt install build-essential curl git libssl-dev pkg-config
```

These packages are required for building the project:
- `build-essential`: Contains essential build tools like gcc, g++, and make
- `curl`: Used for downloading dependencies and tools
- `git`: Required for version control
- `libssl-dev`: OpenSSL development headers required by some Rust crates
- `pkg-config`: Tool for finding installed libraries and their compile flags

## Troubleshooting

### Build Failures

If you encounter build failures, especially related to OpenSSL or other system libraries, ensure you have installed all the required system dependencies listed above.

### Missing Tools

If the build process fails due to missing tools like `dfx` or `pocket-ic`, the build script will attempt to automatically install them. If this fails, you can install them manually using the instructions below in the appropriate sections.

2. **Internet Computer SDK (dfx)**

```bash
DFXVM_INIT_YES=yes sh -ci "$(curl -fsSL https://internetcomputer.org/install.sh)"
```

3. **Node.js Setup**

```bash
# Install NVM (Node Version Manager)
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
source ~/.bashrc  # or restart your terminal

# Install and use Node.js 22
nvm install 22
nvm use 22

# Install required global npm packages
npm install --global jest rimraf
```

4. **Rust Setup**

Visit [rustup.rs](https://rustup.rs) or use:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install wasm-pack for WebAssembly support
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
```

5. **Cargo Make**

```bash
cargo install --force cargo-make
```

6. **Python Environment**
   Set up Python environment for the project:

```bash
python3 scripts/setup-python-env.py
```

This will create a virtual environment and install all necessary dependencies.

### Docker Setup for Volume Permissions

The API service uses Docker volumes to persist the SQLite database. To ensure proper file permissions, the container's `appuser` UID/GID can be configured at build time.

**Default Configuration:**
By default, the API container uses UID/GID 1000:1000, which matches most single-user Linux systems.

**Custom UID/GID:**
If your host data directory has different ownership (e.g., UID 1001), set environment variables before building:

```bash
# Check your data directory ownership
stat -c "UID: %u, GID: %g" data/api-data

# Set matching UID/GID for Docker build
export USER_ID=1001
export GROUP_ID=1001

# Rebuild the API container
docker compose -f cf/docker-compose.yml -f cf/docker-compose.prod.yml build api
```

The build arguments are defined in `cf/docker-compose.yml` and default to 1000 if not specified.

## Building the Project

### Building Binaries

#### Local Development Build

```bash
cargo build --release --bin dc
```

The binary will be available at `target/release/dc`

#### Cross-Platform Builds

For Windows builds, first install [cross](https://github.com/cross-rs/cross), then:

```bash
cross build --release --target x86_64-pc-windows-gnu
```

Example output binaries:

```
target/release/dc                               # Linux/MacOS binary
target/x86_64-pc-windows-gnu/release/dc.exe    # Windows binary
```

### Building the Website

The website is located in the `/website` directory and uses Next.js. The website depends on the WebAssembly (WASM) client package located in `/wasm`.

#### Prerequisites for Website Development

Ensure you have completed the Node.js setup from the prerequisites section above.

#### Website Development Workflow

The website build process has been optimized to automatically build the WASM package when needed, eliminating the need to publish to npmjs.com during development.

**For Production Build:**

```bash
cd website
npm install  # Install dependencies (first time only)
npm run build  # Automatically builds WASM package first, then website
```

**For Development (standard):**

```bash
cd website
npm install  # Install dependencies (first time only)
npm run dev  # Automatically builds WASM package first, then starts dev server
```

**For Development (with auto-rebuild on WASM changes):**

```bash
cd website
npm install  # Install dependencies (first time only)
npm run dev:watch  # Watches WASM files and rebuilds automatically
```

#### How the Build Process Works

1. **Automatic WASM Building**: The website's `prebuild` and `predev` scripts automatically build the WASM package before starting the website build or dev server.

2. **TypeScript Path Mapping**: The website's `tsconfig.json` is configured to import `@decent-stuff/dc-client` from the built WASM distribution (`../wasm/dist`) rather than the source files.

3. **Watch Mode**: The `dev:watch` script uses `nodemon` and `concurrently` to:
   - Watch for changes in WASM source files (`.rs`, `.ts`, `.js`)
   - Automatically rebuild the WASM package when changes are detected
   - Run the Next.js development server simultaneously

#### WASM Package Structure

The WASM package (`/wasm`) contains:

- **Source files**: TypeScript and Rust source code
- **Build script**: `build.js` that compiles Rust to WASM and TypeScript to JavaScript
- **Distribution**: `dist/` directory containing the built package ready for consumption

#### Authentication Architecture

The website uses a tiered authentication approach that allows anonymous browsing while protecting sensitive operations.

**Public Routes** (No authentication required)
- Home page (`/`)
- Dashboard layout (`/dashboard/*` - layout only)
- Public pages: marketplace, offerings, validators, reputation, user profiles

**Protected Routes** (Authentication required)
- Account pages (`/dashboard/account/*`)
- Rental management (`/dashboard/rentals`)
- Provider dashboard (`/dashboard/provider/*`)

**Auth Guard Pattern**

Protected pages use the `requireAuth` utility from `lib/utils/auth-guard.ts`:

```typescript
import { requireAuth } from '$lib/utils/auth-guard';
import { page } from '$app/stores';

onMount(() => {
    const unsubAuth = authStore.isAuthenticated.subscribe((isAuth) => {
        requireAuth(isAuth, $page.url.pathname);
    });
});
```

**Anonymous UX Components**

- `AuthPromptBanner.svelte` - Top banner encouraging account creation for anonymous users
- `AuthPromptModal.svelte` - Modal shown when users attempt auth-required actions
- Both components support `returnUrl` parameter for seamless post-auth navigation

**Return URL Flow**

1. Anonymous user attempts protected action
2. `requireAuth` redirects to `/?returnUrl=/protected/page`
3. User completes authentication
4. `Header.svelte` detects auth change and navigates to `returnUrl`
5. User continues their original task

#### Troubleshooting Website Development

**Issue: Import errors for `@decent-stuff/dc-client`**

- Solution: Ensure the WASM package is built by running `cd wasm && npm run build`

**Issue: Changes to WASM code not reflected in website**

- Solution: Use `npm run dev:watch` instead of `npm run dev` for automatic rebuilding

**Issue: Build fails with missing dependencies**

- Solution: Run `npm install` in both `/wasm` and `/website` directories

## Email Configuration

The API backend includes optional email support using [MailChannels](https://www.mailchannels.com/). When configured, the system will send transactional emails such as welcome messages to new users.

### Getting a MailChannels API Key

1. Sign up for a MailChannels account at [app.mailchannels.com](https://app.mailchannels.com/)
2. Obtain your API key from the dashboard
3. Configure the environment variables (see below)

### Email Environment Variables

Add these to your `api/.env` file (see `api/.env.example` for reference).

### How Email Works

- **Queue-based**: Emails are queued in the database and processed asynchronously
- **Retry logic**: Failed emails are retried with exponential backoff (2^attempts minutes)
- **Non-blocking**: Email failures never block business logic (account creation, etc.)
- **Optional**: If `MAILCHANNELS_API_KEY` is not set, emails are queued but not sent

### DKIM Configuration (Optional)

DKIM (DomainKeys Identified Mail) improves email deliverability by signing emails with your domain. To configure:

1. Generate a DKIM key pair (MailChannels dashboard or `openssl`)
2. Add the public key as a TXT record in your DNS
3. Add the private key (base64 encoded) to your `.env` file

Without DKIM, emails will still be sent but may have lower deliverability rates.

### Testing Email Locally

During development, emails are queued in the database but won't be sent unless you have a valid API key. To test:

1. **Check logs**: Email queueing is logged even without an API key
2. **Query database**: Check the `email_queue` table to see queued emails
3. **Use test API key**: Set up a test MailChannels account for development

```bash
# Check queued emails in the database
sqlite3 data/api-data-dev/ledger.db "SELECT * FROM email_queue;"
```

## Stripe Configuration

Optional payment processing for marketplace rentals. Without Stripe, only DCT token payments are available.

### Setup

1. **Create Account**: [dashboard.stripe.com/register](https://dashboard.stripe.com/register) (free)
2. **Get API Keys**: Dashboard → Developers → API keys
   - Copy `pk_test_...` (publishable) and `sk_test_...` (secret)
   - Use test keys for development, live keys (`pk_live_...`, `sk_live_...`) for production

3. **Set Environment Variables** in `api/.env`:
```bash
STRIPE_SECRET_KEY=sk_test_your_key
STRIPE_PUBLISHABLE_KEY=pk_test_your_key
STRIPE_WEBHOOK_SECRET=whsec_test_secret  # Placeholder for local dev
```

And in `website/.env`:
```bash
VITE_STRIPE_PUBLISHABLE_KEY=pk_test_your_key
```

4. **Test Locally**: Use test cards (no real charges)
   - Success: `4242 4242 4242 4242`
   - Decline: `4000 0000 0000 0002`
   - Any future expiry, any CVC

### Webhook Configuration (Production Only)

Dashboard → Developers → Webhooks → Add endpoint:
- URL: `https://your-domain.com/api/v1/webhooks/stripe`
- API version: `2025-11-17` (or latest)
- Events: **`payment_intent.succeeded`** and **`payment_intent.payment_failed`** only
- Replace `STRIPE_WEBHOOK_SECRET` in production `.env` with signing secret (`whsec_...`)

**Local Webhook Testing** (optional - for realistic testing):
```bash
stripe listen --forward-to localhost:59001/api/v1/webhooks/stripe
# Use webhook secret from CLI output in api/.env
```

For E2E test setup, see [website/tests/e2e/STRIPE_TESTING_SETUP.md](../website/tests/e2e/STRIPE_TESTING_SETUP.md).

## Testing

### Running Tests

1. **Unit Tests**

```bash
cargo test
```

2. **Complete Test Suite**
   Includes unit tests, canister tests using PocketIC, and website checks:

```bash
cargo make
```

   This runs:
   - Rust formatting, linting (clippy), and unit tests
   - Canister builds and integration tests
   - Website TypeScript checks (`npm run check`)
   - Website build (`npm run build`)

### Writing Tests

1. **Unit Test Guidelines**

- Place tests in a `tests` module
- Use meaningful test names
- Cover edge cases
- Add documentation

2. **Integration Test Guidelines**

- Test real-world scenarios
- Cover main workflows
- Test error conditions
- Document test setup

## Documentation

### Building Documentation

#### Whitepaper

```bash
# Activate virtual environment first (if not already activated)
source .venv/bin/activate  # On Linux/Mac
# or .venv\Scripts\activate  # On Windows

python3 ./docs/whitepaper/build.py
```

The PDF will be generated at `build/docs/whitepaper/whitepaper.pdf`

#### API Documentation

```bash
cargo doc --no-deps --open
```

### Documentation Guidelines

1. **Code Documentation**

- Document public interfaces
- Explain complex algorithms
- Include usage examples
- Note any limitations

2. **README Updates**

- Keep installation instructions current
- Document new features
- Update troubleshooting guides
- Maintain changelog

## CI/CD

### Local CI Image

Build the CI image locally:

```bash
docker build -f .github/container/Dockerfile --tag ghcr.io/decent-stuff/decent-cloud/ci-image:latest .
```

Push to registry:

```bash
docker push ghcr.io/decent-stuff/decent-cloud/ci-image:latest
```

If push fails with "denied":

1. Get new token from [GitHub settings/tokens](https://github.com/settings/tokens?page=1)
2. Login:

```bash
docker login ghcr.io
username: <your-username>
password: <generated token>
```

### GitHub Actions

- CI workflow runs on all PRs
- Release workflow triggers on tags
- Container image workflow can be manually triggered

## Contributing

### Getting Started

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Submit a pull request

### Contribution Guidelines

1. **Code Style**

- Follow Rust style guidelines
- Use meaningful variable names
- Keep functions focused
- Add appropriate comments

2. **Commit Messages**

- Use clear, descriptive messages
- Reference issues when applicable
- Follow conventional commits

3. **Pull Requests**

- Describe changes thoroughly
- Include test coverage
- Update documentation
- Add to changelog

### Review Process

1. **Before Submitting**

- Run all tests
- Update documentation
- Check formatting
- Verify CI passes

2. **During Review**

- Respond to feedback
- Make requested changes
- Keep discussion professional
- Update as needed

## Project Structure

```
decent-cloud/
├── cli/            # Command-line interface
├── common/         # Shared utilities
├── ic-canister/    # Internet Computer canister
├── ledger-map/     # A secure, persistent key-value storage
├── wasm/           # WebAssembly client package
│   ├── src/        # Rust source code
│   ├── dist/       # Built package (auto-generated)
│   ├── build.js    # Build script
│   └── *.ts        # TypeScript source files
├── website/        # Next.js frontend website
│   ├── app/        # Next.js app directory
│   ├── components/ # React components
│   ├── lib/        # Utility libraries
│   └── public/     # Static assets
├── docs/           # Documentation
```

## Support

- 📝 [Open an Issue](https://github.com/decent-stuff/decent-cloud/issues)
- 💬 [Join Discussions](https://github.com/orgs/decent-stuff/discussions)
- 📚 [Read the Whitepaper](https://decent-cloud.org/)

## License

This project is licensed under the Apache 2.0 License - see the [LICENSE](../LICENSE) file for details.
