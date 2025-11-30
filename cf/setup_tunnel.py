#!/usr/bin/env python3
"""Interactive setup script for Cloudflare Tunnel configuration."""

import sys
from pathlib import Path
from cf_common import print_header, print_success, print_error, print_warning, print_info, BLUE, NC


def main() -> int:
    """Main setup function."""
    print_header("Decent Cloud - Cloudflare Tunnel Setup")

    # Change to cf directory if not already there
    cf_dir = Path(__file__).parent
    env_file = cf_dir / ".env"
    example_file = cf_dir / ".env.tunnel.example"

    if env_file.exists():
        print_warning(f".env file already exists at {env_file}")
        response = input("Do you want to overwrite it? (y/N): ").strip().lower()
        if response != "y":
            print_info("Setup cancelled")
            return 0

    print()
    print("To set up Cloudflare Tunnel, follow these steps:")
    print()
    print(f"  1. Go to: {BLUE}https://one.dash.cloudflare.com/{NC}")
    print("  2. Navigate to: Networks > Connectors > Cloudflare Tunnels")
    print("  3. Click 'Create a tunnel' > Choose 'Cloudflared'")
    print("  4. Name it: 'decent-cloud-website'")
    print("  5. Click 'Save tunnel'")
    print()
    print("  6. Configure Public Hostname:")
    print("     - Subdomain: your choice (e.g., 'app' or 'decent-cloud')")
    print("     - Domain: your domain")
    print("     - Service Type: HTTP")
    print("     - URL: website:59100")
    print("  7. Click 'Save hostname'")
    print()
    print("  8. Select 'Docker' environment")
    print("  9. Copy the token from the installation command")
    print(f"     (the long string after {BLUE}--token{NC})")
    print()

    token = input("Paste your tunnel token here: ").strip()

    if not token:
        print_error("No token provided")
        return 1

    if not token.startswith("eyJ"):
        print_warning("Token doesn't look correct (should start with 'eyJ')")
        response = input("Continue anyway? (y/N): ").strip().lower()
        if response != "y":
            return 1

    # Create .env file
    with open(env_file, "w") as f:
        if example_file.exists():
            # Copy template if exists
            with open(example_file) as template:
                for line in template:
                    if line.startswith("TUNNEL_TOKEN=") or line.startswith("export TUNNEL_TOKEN="):
                        f.write(f"export TUNNEL_TOKEN={token}\n")
                    else:
                        f.write(line)
        else:
            # Create minimal file
            f.write("# Cloudflare Tunnel Configuration\n")
            f.write(f"export TUNNEL_TOKEN={token}\n")

    print()
    print_success(f"Configuration saved to {env_file}")
    print()
    print("Next steps:")
    print(f"  • For development: {BLUE}python3 deploy_dev.py{NC}")
    print(f"  • For production:  {BLUE}python3 deploy_prod.py{NC}")
    print()

    return 0


if __name__ == "__main__":
    sys.exit(main())
