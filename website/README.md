# Decent Cloud Website

This is the official frontend website for Decent Cloud, built with [Next.js](https://nextjs.org) and TypeScript.

## Overview

The website provides a user-friendly interface for interacting with the Decent Cloud platform, including:

- Ledger browsing and transaction history
- Account management and authentication
- Provider offerings and marketplace
- Real-time data visualization

## Development Setup

### Prerequisites

- Node.js 22+ (use `nvm` to manage versions)
- npm or yarn package manager

### Getting Started

1. **Install dependencies:**

   ```bash
   npm install
   ```

2. **Start development server:**

   ```bash
   npm run dev
   ```

   This automatically builds the WASM package first, then starts the Next.js dev server at [http://localhost:50200](http://localhost:50200).

3. **For development with auto-rebuild on WASM changes:**
   ```bash
   npm run dev:watch
   ```
   This watches for changes in the WASM source files and automatically rebuilds when needed.

### Build Commands

- `npm run dev` - Start development server (builds WASM first)
- `npm run dev:watch` - Start development server with WASM file watching
- `npm run build` - Build for production (builds WASM first)
- `npm run start` - Start production server
- `npm run lint` - Run ESLint

## Architecture

### WASM Integration

The website integrates with the Decent Cloud WebAssembly client package located in `../wasm`. The build process is configured to:

1. **Automatic Building**: WASM package is automatically built before starting the website
2. **TypeScript Path Mapping**: Imports `@decent-stuff/dc-client` from the built distribution (`../wasm/dist`)
3. **Watch Mode**: Monitors WASM source files for changes during development

### Key Dependencies

- **Next.js 15+** - React framework
- **TypeScript** - Type safety
- **Tailwind CSS** - Styling
- **@decent-stuff/dc-client** - WASM client (local package)
- **@dfinity/agent** - Internet Computer integration
- **Dexie** - IndexedDB wrapper for local data storage

## Project Structure

```
website/
├── app/                 # Next.js app directory (pages)
├── components/          # Reusable React components
├── lib/                # Utility libraries and services
├── public/             # Static assets
├── styles/             # Global styles
└── hooks/              # Custom React hooks
```

## Development Workflow

### Making Changes to WASM Code

When working on both the website and WASM package:

1. Use `npm run dev:watch` to automatically rebuild WASM on changes
2. Or manually rebuild WASM: `cd ../wasm && npm run build`
3. The website will automatically pick up the new build

### Adding New Features

1. Create components in `components/`
2. Add pages in `app/`
3. Use the WASM client via `@decent-stuff/dc-client` imports
4. Add utility functions in `lib/`

### Troubleshooting

**Import errors for `@decent-stuff/dc-client`:**

- Ensure WASM package is built: `cd ../wasm && npm run build`

**Changes not reflected:**

- Use `npm run dev:watch` for automatic rebuilding
- Clear Next.js cache: `rm -rf .next`

**Build failures:**

- Check that all dependencies are installed in both `/wasm` and `/website`
- Ensure Node.js version is 22+

## Deployment

The website is automatically deployed to Cloudflare Pages on pushes to the main branch.

For manual deployment or other hosting services, the website can be built as a static site:

```bash
npm run build
```

This generates a static site that can be deployed to any static hosting service.

See [Cloudflare Deployment Setup](../docs/cloudflare-deployment.md) for detailed instructions
on setting up automatic deployment to Cloudflare Pages.

## Contributing

See the main [Development Guide](../docs/development.md) for contribution guidelines and setup instructions.

## License

This project is licensed under the MIT License - see the [LICENSE](../LICENSE) file for details.
