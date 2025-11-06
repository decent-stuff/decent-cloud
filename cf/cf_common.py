#!/usr/bin/env python3
"""Shared utilities for Cloudflare tunnel deployment scripts."""

import os
import subprocess
import sys
from pathlib import Path
from typing import Optional

# ANSI color codes
RED = '\033[0;31m'
GREEN = '\033[0;32m'
YELLOW = '\033[1;33m'
BLUE = '\033[0;34m'
NC = '\033[0m'

def print_header(text: str) -> None:
    """Print a colored header."""
    print(f"{GREEN}{text}{NC}")
    print("=" * len(text))
    print()

def print_success(text: str) -> None:
    """Print success message."""
    print(f"{GREEN}✓{NC} {text}")

def print_error(text: str) -> None:
    """Print error message."""
    print(f"{RED}✗{NC} {text}", file=sys.stderr)

def print_warning(text: str) -> None:
    """Print warning message."""
    print(f"{YELLOW}⚠{NC}  {text}")

def print_info(text: str) -> None:
    """Print info message."""
    print(f"{BLUE}→{NC} {text}")

def check_docker() -> bool:
    """Check if Docker and Docker Compose are installed."""
    try:
        subprocess.run(["docker", "--version"], check=True, capture_output=True)
        subprocess.run(["docker", "compose", "version"], check=True, capture_output=True)
        print_success("Docker and Docker Compose are installed")
        return True
    except (subprocess.CalledProcessError, FileNotFoundError):
        print_error("Docker or Docker Compose is not installed")
        print_info("Install Docker: https://docs.docker.com/get-docker/")
        return False

def load_env_file(env_file: Path) -> Optional[dict[str, str]]:
    """Load environment variables from a file."""
    if not env_file.exists():
        return None

    env_vars = {}
    with open(env_file) as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith('#'):
                continue
            if line.startswith('export '):
                line = line[7:]  # Remove 'export ' prefix
            if '=' in line:
                key, value = line.split('=', 1)
                env_vars[key.strip()] = value.strip()

    return env_vars

def get_tunnel_token(env_file: Path) -> Optional[str]:
    """Get tunnel token from environment file."""
    env_vars = load_env_file(env_file)
    if not env_vars:
        return None
    return env_vars.get('TUNNEL_TOKEN')

def run_docker_compose(compose_files: list[str], command: list[str], env_vars: dict[str, str]) -> bool:
    """Run docker compose with specified files and environment."""
    cmd = ["docker", "compose"]
    for file in compose_files:
        cmd.extend(["-f", file])
    cmd.extend(command)

    try:
        subprocess.run(cmd, check=True, env={**os.environ, **env_vars})
        return True
    except subprocess.CalledProcessError:
        return False

def check_tunnel_status(compose_files: list[str]) -> str:
    """Check tunnel connection status from logs."""
    try:
        result = subprocess.run(
            ["docker", "compose"] +
            [arg for f in compose_files for arg in ["-f", f]] +
            ["logs", "cloudflared"],
            capture_output=True,
            text=True,
            check=False
        )
        logs = result.stdout + result.stderr

        if "Registered tunnel connection connIndex=" in logs:
            return "connected"
        elif "Unauthorized" in logs:
            return "unauthorized"
        else:
            return "unclear"
    except Exception:
        return "error"

def deploy(env_name: str, env_vars: dict[str, str], compose_files: list[str]) -> int:
    """Shared deployment logic for dev and prod environments."""
    is_prod = env_name == "production"

    # Header
    print_header(f"Decent Cloud - {env_name.title()} Deployment")

    # Check Docker
    if not check_docker():
        return 1
    print()

    # Check for .env file
    cf_dir = Path(__file__).parent
    env_file = cf_dir / '.env'

    if not env_file.exists():
        print_warning(f".env file not found at {env_file}")
        print()
        print(f"Run setup first: {BLUE}python3 setup_tunnel.py{NC}")
        print()
        return 1

    print_success("Found .env file")
    print()

    # Load tunnel token
    token = get_tunnel_token(env_file)
    if not token:
        print_error("TUNNEL_TOKEN not found in .env file")
        print()
        print(f"Run setup again: {BLUE}python3 setup_tunnel.py{NC}")
        print()
        return 1

    print_success("Tunnel token loaded")
    print()

    # Add token to env_vars
    env_vars['TUNNEL_TOKEN'] = token

    # Build and start services
    action = "production services" if is_prod else "services"
    print_warning(f"Building and starting {action}...")
    print()

    if not run_docker_compose(compose_files, ["up", "-d", "--build"], env_vars):
        print()
        print_error(f"{env_name.title()} deployment failed")
        print()
        compose_args = " ".join(f"-f {f}" for f in compose_files)
        print(f"Check logs: {BLUE}docker compose {compose_args} logs{NC}")
        print()
        return 1

    # Success message
    print()
    print(f"{GREEN}========================================")
    if is_prod:
        print("Production Deployment Complete!")
    else:
        print("Containers Started!")
    print(f"========================================{NC}")
    print()
    print("Services started:")
    if is_prod:
        print("  • Decent Cloud Website (production build)")
    else:
        print("  • Decent Cloud Website")
    print("  • Cloudflare Tunnel")
    print()

    # Check tunnel connection
    print_warning("Verifying tunnel connection..." if is_prod else "Checking tunnel connection...")
    import time
    time.sleep(5)

    status = check_tunnel_status(compose_files)

    if status == "connected":
        print_success("Tunnel connected successfully!")
        print()
        if is_prod:
            print("Your website is now live and accessible")
            print()
            print("Verify deployment:")
            print(f"  • Check tunnel status in Cloudflare dashboard")
            print(f"  • Test your domain: https://your-domain.com/health")
        else:
            print("Your website is now accessible through Cloudflare")
    elif status == "unauthorized":
        print_error("Tunnel authentication failed!")
        print()
        if is_prod:
            print("Possible causes:")
            print("  1. Tunnel doesn't exist in Cloudflare dashboard")
            print("  2. Token is invalid or expired")
        else:
            print("The tunnel token is invalid or the tunnel doesn't exist")
        print()
        print(f"Fix: {BLUE}python3 setup_tunnel.py{NC}")
        print()
        if is_prod:
            return 1
    else:
        msg = f"Could not verify tunnel status" if status == "error" else "Tunnel status unclear"
        print_warning(msg)
        compose_args = " ".join(f"-f {f}" for f in compose_files)
        print(f"  {BLUE}docker compose {compose_args} logs cloudflared{NC}")
        print()

    # Management commands
    print("Useful commands:" if not is_prod else "Management commands:")
    compose_args = " ".join(f"-f {f}" for f in compose_files)
    if is_prod:
        print(f"  View logs:    {BLUE}docker compose {compose_args} logs -f{NC}")
        print(f"  Check status: {BLUE}docker compose {compose_args} ps{NC}")
        print(f"  Restart:      {BLUE}docker compose {compose_args} restart{NC}")
        print(f"  Stop:         {BLUE}docker compose {compose_args} down{NC}")
    else:
        print(f"  {BLUE}docker compose {compose_args} logs -f{NC}")
        print(f"  {BLUE}docker compose {compose_args} ps{NC}")
        print(f"  {BLUE}docker compose {compose_args} down{NC}")
    print()

    return 0
