# E2E Tests

End-to-end tests for the Decent Cloud website authentication flows using Playwright.

## Prerequisites

Before running E2E tests, ensure you have:

### Option 1: Docker Setup (Recommended)

```bash
# From project root
cd cf
./deploy.py deploy dev

# This will start:
# - API server at http://localhost:59001
# - Website at http://localhost:59000
# - Cloudflare tunnel
```

### Option 2: Manual Setup

1. **API Server Running**
   ```bash
   # From project root
   cd api
   cargo run
   # API should be running at http://localhost:8080
   ```

2. **Dev Server Running**
   ```bash
   # From website directory
   npm run dev
   # Dev server should be running at http://localhost:5173
   ```

   Set environment variable:
   ```bash
   export PLAYWRIGHT_BASE_URL=http://localhost:5173
   ```

### Database

Tests create accounts with timestamp-based prefixes (e.g., `t17320278909823`) to avoid conflicts between test runs.

**Clean Database Before Tests (Recommended)**

Run the cleanup script before each test run to remove old test accounts:

```bash
# From website directory
./tests/e2e/cleanup-test-data.sh
npm run test:e2e
```

Or manually with SQL:
```bash
# Delete test accounts directly
psql "$DATABASE_URL" -c "DELETE FROM accounts WHERE username ~ '^t[0-9]'
```

**Alternative: Full Reset**

If you need a completely fresh database:

```bash
# Stop containers, remove database volume, restart
docker compose -f ../cf/docker-compose.dev.yml down -v
cd ../cf && ./deploy.py deploy dev
```

## Running Tests

**Note:** Tests expect servers to already be running. Start Docker containers first (see Prerequisites).

### Run All E2E Tests (Headless)
```bash
npm run test:e2e
```

**Tip:** If you get connection errors, verify:
```bash
# Check if website is accessible
curl http://localhost:59000/

# Check if API is accessible
curl http://localhost:59001/api/v1/stats
```

### Run with Playwright UI (Recommended for Development)
```bash
npm run test:e2e:ui
```
- Interactive UI to run/debug tests
- See test execution in real-time
- Inspect DOM, network, console logs

### Run in Debug Mode
```bash
npm run test:e2e:debug
```
- Opens Playwright Inspector
- Step through tests line-by-line
- Set breakpoints

### Run in Headed Mode (See Browser)
```bash
npm run test:e2e:headed
```
- Runs tests with visible browser window
- Useful for visual debugging

### Run Specific Test File
```bash
npx playwright test registration-flow
npx playwright test signin-flow
npx playwright test account-page
```

### Run Specific Test
```bash
npx playwright test -g "should complete full registration flow"
```

## Test Structure

```
tests/e2e/
├── registration-flow.spec.ts  # Account registration tests
├── signin-flow.spec.ts        # Sign-in authentication tests
├── account-page.spec.ts       # Account settings page tests
└── fixtures/
    └── auth-helpers.ts        # Reusable test helpers
```

## Test Coverage

### Registration Flow (`registration-flow.spec.ts`)
- ✅ Complete registration with seed phrase
- ✅ Username validation (format, length)
- ✅ Username availability checking
- ✅ Seed phrase generation and backup
- ✅ Skip backup with warning
- ✅ Network error handling

### Sign-In Flow (`signin-flow.spec.ts`)
- ✅ Sign in with valid credentials
- ✅ Invalid seed phrase rejection
- ✅ Non-existent username handling
- ✅ Public key verification
- ✅ Session persistence after refresh
- ✅ Sign out functionality
- ✅ Auto-detect account from seed phrase

### Account Page (`account-page.spec.ts`)
- ✅ Display account overview
- ✅ Copy username to clipboard
- ✅ Copy account ID to clipboard
- ✅ Account link in sidebar
- ✅ Username in header
- ✅ Navigation between pages
- ✅ Date formatting
- ✅ Direct URL access
- ✅ Edit device name
- ✅ Cancel device name edit
- ✅ Single key account (no Remove button)
- ✅ Display device key info
- ✅ Show Add Device button
- ✅ Open Add Device modal
- ✅ Add new device with seed phrase
- ✅ Cancel Add Device modal

## Test Helpers

### `auth-helpers.ts`

**`generateTestUsername()`**
- Generates unique username based on timestamp
- Ensures no conflicts between test runs

**`registerNewAccount(page)`**
- Complete registration flow automation
- Returns `{ username, seedPhrase }`
- Use in test setup to create authenticated user

**`signIn(page, credentials)`**
- Sign in with existing account
- Takes username and seed phrase
- Verifies successful authentication

**`signOut(page)`**
- Sign out from application
- Verifies redirect to home page

**`waitForApiResponse(page, urlPattern)`**
- Wait for specific API call to complete
- Useful for timing-sensitive tests

## Example Usage

```typescript
import { test, expect } from '@playwright/test';
import { registerNewAccount, signIn, signOut } from './fixtures/auth-helpers';

test('my test', async ({ page }) => {
  // Create and sign in as new user
  const credentials = await registerNewAccount(page);

  // Do something...
  await page.goto('/dashboard/account');

  // Sign out
  await signOut(page);

  // Sign back in
  await signIn(page, credentials);
});
```

## Debugging Tips

1. **Use Playwright UI** (`npm run test:e2e:ui`)
   - Best way to see what's happening
   - Inspect element selectors
   - View network requests

2. **Check Screenshots on Failure**
   - Automatically saved to `test-results/`
   - Shows exact state when test failed

3. **Enable Video Recording**
   - Videos saved on test failure
   - See full test execution

4. **Add `page.pause()`**
   - Freezes test execution
   - Opens Playwright Inspector
   - Manually interact with page

5. **Check Console Logs**
   ```typescript
   page.on('console', msg => console.log('PAGE LOG:', msg.text()));
   ```

## Common Issues

### Tests Fail: "Sign In button not found"
- **Cause**: Dev server not running
- **Fix**: Start dev server with `npm run dev`

### Tests Fail: API errors (500, 404)
- **Cause**: API server not running or wrong port
- **Fix**: Start API server, verify port 8080

### Tests Fail: "Username already taken"
- **Cause**: Previous test run left data in database
- **Fix**: Reset test database or use unique usernames

### Tests Timeout
- **Cause**: Slow API responses or network issues
- **Fix**: Check API logs, increase timeout in test

### Flaky Tests
- **Cause**: Race conditions, timing issues
- **Fix**: Add explicit waits with `waitForApiResponse()`

## CI/CD Integration

Tests are configured for GitHub Actions:

```yaml
- name: Install Playwright
  run: npx playwright install chromium

- name: Run E2E Tests
  run: npm run test:e2e
  env:
    CI: true
```

## Best Practices

1. **Use Helpers** - Don't duplicate auth flows, use `auth-helpers.ts`
2. **Unique Data** - Generate unique usernames to avoid conflicts
3. **Clean State** - Each test should be independent
4. **Explicit Waits** - Wait for API responses, not arbitrary timeouts
5. **Descriptive Names** - Test names should explain what they validate
6. **One Assertion** - Focus each test on one behavior
7. **Fast Feedback** - Run specific tests during development

## Maintenance

- Update selectors if UI changes
- Add new helpers for common operations
- Keep tests DRY (Don't Repeat Yourself)
- Document non-obvious test logic
- Review failed tests promptly

## Next Steps

Future test additions:
- Key removal with confirmation dialog (requires multiple keys)
- Sign in with newly added device key
- Error boundary testing
- Accessibility testing (axe)
- Performance testing (Lighthouse)
