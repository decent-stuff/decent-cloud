# Decent Cloud Website (SvelteKit)

Modern, lightweight rebuild of the Decent Cloud website using SvelteKit 2.

## Features

- 🚀 **Instant HMR** - Sub-100ms hot reload
- 📦 **4KB runtime** vs 45KB+ with React
- 🔌 **Full ICP integration** - Same @dfinity packages
- 🎨 **Tailwind CSS 4** - Modern styling
- 🔐 **Seed phrase auth** - BIP39 wallet support

## Development

```bash
# Install dependencies
npm install

# Start dev server
npm run dev

# Start dev server with network access
npm run dev -- --host

# Build for production
npm run build

# Preview production build
npm run preview

# Type check
npm run check
```

## Structure

```
src/
├── lib/
│   ├── components/     # Svelte components
│   ├── services/       # ICP & API services
│   ├── stores/         # Svelte stores (auth, etc)
│   └── utils/          # Utility functions
├── routes/
│   ├── +layout.svelte  # Root layout
│   └── +page.svelte    # Homepage
└── app.css             # Global Tailwind styles
```

## Tech Stack

- **Framework**: SvelteKit 2 (Svelte 5)
- **Styling**: Tailwind CSS 4
- **ICP SDK**: @dfinity/agent, @dfinity/auth-client
- **Wallet**: BIP39 + Ed25519 (same as old site)
- **Build**: Vite 7

## Performance

Build output (production):
- Client bundle: ~197KB (67KB gzipped)
- Server bundle: ~126KB
- Build time: ~5s
