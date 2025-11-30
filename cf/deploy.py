#!/usr/bin/env python3
"""Deployment script with dev/prod environments and management commands."""

import os
import subprocess
import sys
import shlex
import hashlib
from pathlib import Path
from typing import Optional
import argparse
from dotenv import dotenv_values


def calculate_binary_hash() -> str:
    """Calculate SHA256 hash of API binary for Docker cache invalidation.

    This ensures Docker rebuilds when the binary changes for ANY reason:
    - Migration changes (embedded via sqlx::migrate!)
    - Code changes (bug fixes, features)
    - Dependency updates
    """
    cf_dir = Path(__file__).parent
    binary_path = cf_dir.parent / "target" / "x86_64-unknown-linux-gnu" / "release" / "api-server"

    if not binary_path.exists():
        return "no-binary"

    hasher = hashlib.sha256()

    # Hash the binary content
    with open(binary_path, "rb") as f:
        # Read in chunks for memory efficiency (binary can be large)
        for chunk in iter(lambda: f.read(4096), b""):
            hasher.update(chunk)

    return hasher.hexdigest()[:16]  # Short hash for readability


def get_env_config(environment: str) -> tuple[dict[str, str], list[str]]:
    """Get environment-specific configuration."""
    cf_dir = Path(__file__).parent

    if environment == "prod":
        env_vars = {"ENVIRONMENT": "prod", "NETWORK_NAME": "decent-cloud-prod"}
        compose_files = [str(cf_dir / "docker-compose.yml"), str(cf_dir / "docker-compose.prod.yml")]
    else:  # dev
        env_vars = {"ENVIRONMENT": "dev", "NETWORK_NAME": "decent-cloud-dev"}
        compose_files = [str(cf_dir / "docker-compose.yml"), str(cf_dir / "docker-compose.dev.yml")]

    return env_vars, compose_files


def deploy_environment(environment: str) -> int:
    """Deploy to specified environment."""
    env_vars, compose_files = get_env_config(environment)

    return deploy(environment, env_vars, compose_files)


def stop_environment(environment: str) -> int:
    """Stop services for specified environment."""
    env_vars, compose_files = get_env_config(environment)
    project_name = f"decent-cloud-{environment}"

    print_header(f"Stopping {environment} services")

    if not run_docker_compose(compose_files, ["down"], env_vars, project_name):
        print_error(f"Failed to stop {environment} services")
        return 1

    print_success(f"{environment.title()} services stopped successfully")
    return 0


def show_logs(environment: str, follow: bool = False, service: Optional[str] = None) -> int:
    """Show logs for specified environment."""
    env_vars, compose_files = get_env_config(environment)
    project_name = f"decent-cloud-{environment}"

    print_header(f"{environment.title()} logs")

    cmd = ["logs"]
    if follow:
        cmd.append("-f")
    if service:
        cmd.append(service)

    if not run_docker_compose(compose_files, cmd, env_vars, project_name):
        print_error(f"Failed to get logs for {environment}")
        return 1

    return 0


def show_status(environment: str) -> int:
    """Show status for specified environment."""
    env_vars, compose_files = get_env_config(environment)
    project_name = f"decent-cloud-{environment}"

    print_header(f"{environment.title()} status")

    if not run_docker_compose(compose_files, ["ps"], env_vars, project_name):
        print_error(f"Failed to get status for {environment}")
        return 1

    # Check tunnel status if running
    status = check_tunnel_status(compose_files, environment)
    print()
    if status == "connected":
        print_success("Tunnel connection: Active")
    elif status == "unauthorized":
        print_warning("Tunnel connection: Unauthorized")
    else:
        print_warning("Tunnel connection: Unknown")

    return 0


def restart_environment(environment: str) -> int:
    """Restart services for specified environment."""
    env_vars, compose_files = get_env_config(environment)
    project_name = f"decent-cloud-{environment}"

    print_header(f"Restarting {environment} services")

    if not run_docker_compose(compose_files, ["restart"], env_vars, project_name):
        print_error(f"Failed to restart {environment} services")
        return 1

    print_success(f"{environment.title()} services restarted successfully")
    return 0


# ANSI color codes
RED = "\033[0;31m"
GREEN = "\033[0;32m"
YELLOW = "\033[1;33m"
BLUE = "\033[0;34m"
NC = "\033[0m"


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
    """Load environment variables from a file using python-dotenv."""
    if not env_file.exists():
        return None

    # dotenv_values handles various formats: KEY=value, export KEY=value, quoted values, etc.
    env_vars = dotenv_values(env_file)

    # Convert to regular dict[str, str] (dotenv_values returns dict[str, str | None])
    return {k: v for k, v in env_vars.items() if v is not None}


def get_tunnel_token(env_file: Path) -> Optional[str]:
    """Get tunnel token from environment file."""
    env_vars = load_env_file(env_file)
    if not env_vars:
        return None
    return env_vars.get("TUNNEL_TOKEN")


def run_docker_compose(
    compose_files: list[str], command: list[str], env_vars: dict[str, str], project_name: Optional[str] = None
) -> bool:
    """Run docker compose with specified files and environment."""
    cmd = ["docker", "compose"]
    if project_name:
        cmd.extend(["-p", project_name])
    for file in compose_files:
        cmd.extend(["-f", file])
    cmd.extend(command)

    try:
        print_info(f"$ {' '.join(shlex.quote(arg) for arg in cmd)}")
        subprocess.run(cmd, check=True, env={**os.environ, **env_vars})
        return True
    except subprocess.CalledProcessError:
        return False


def check_tunnel_status(compose_files: list[str], env_name: str) -> str:
    """Check tunnel connection status from logs."""
    try:
        project_name = f"decent-cloud-{env_name[:4]}"  # dev -> dev, prod -> prod

        cmd = ["docker", "compose", "-p", project_name]
        for f in compose_files:
            cmd.extend(["-f", f])
        cmd.extend(["logs", "cloudflared"])

        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            env=os.environ,
            check=False,
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


def check_prerequisites() -> bool:
    """Check if required build tools are installed."""
    print_header("Checking prerequisites")

    # Check Rust toolchain
    try:
        subprocess.run(["rustc", "--version"], check=True, capture_output=True)
        print_success("Rust toolchain found")
    except (subprocess.CalledProcessError, FileNotFoundError):
        print_error("Rust not found. Please install Rust: https://rustup.rs/")
        return False

    # Check Node.js
    try:
        subprocess.run(["node", "--version"], check=True, capture_output=True)
        print_success("Node.js found")
    except (subprocess.CalledProcessError, FileNotFoundError):
        print_error("Node.js not found. Please install Node.js: https://nodejs.org/")
        return False

    # Check Rust target for cross-compilation
    try:
        result = subprocess.run(["rustup", "target", "list"], check=True, capture_output=True, text=True)
        if "x86_64-unknown-linux-musl" not in result.stdout:
            print_info("Adding Rust target for cross-compilation...")
            subprocess.run(["rustup", "target", "add", "x86_64-unknown-linux-musl"], check=True)
            print_success("Added x86_64-unknown-linux-musl target")
    except (subprocess.CalledProcessError, FileNotFoundError):
        print_warning("rustup not found. Cross-compilation may fail")

    return True


def build_rust_binaries_natively() -> bool:
    """Build API server binary natively before Docker build."""
    cf_dir = Path(__file__).parent
    api_dir = cf_dir.parent / "api"

    print_header("Building API server natively")

    if not api_dir.exists():
        print_error("API directory not found")
        return False

    try:
        # Change to project root and build API binary
        project_root = cf_dir.parent
        os.chdir(project_root)

        # Build for linux/amd64 target (required for Docker)
        # SQLX_OFFLINE=true uses pre-prepared .sqlx queries instead of live DB
        build_env = {**os.environ, "SQLX_OFFLINE": "true"}
        subprocess.run(
            [
                "cargo",
                "build",
                "--release",
                "--bin",
                "api-server",
                "--bin",
                "dc",
                "--target",
                "x86_64-unknown-linux-gnu",
            ],
            check=True,
            env=build_env,
        )

        # Verify binary was created
        binary_path = project_root / "target" / "x86_64-unknown-linux-gnu" / "release" / "api-server"
        if not binary_path.exists():
            print_error(f"API binary not found at {binary_path}")
            return False

        print_success(f"API server built successfully: {binary_path}")
        return True
    except subprocess.CalledProcessError as e:
        print_error(f"API build failed: {e}")
        print_error(f"stdout: {e.stdout}")
        print_error(f"stderr: {e.stderr}")
        return False
    except FileNotFoundError:
        print_error("Cargo not found. Please install Rust")
        return False
    except Exception as e:
        print_error(f"Unexpected error during API build: {e}")
        return False


def build_website_natively(environment: str, env_vars: dict[str, str]) -> bool:
    """Build SvelteKit website natively before Docker build.

    Args:
        environment: 'dev' or 'prod' - determines API endpoint configuration
        env_vars: Environment variables from .env file (contains Stripe keys)
    """
    cf_dir = Path(__file__).parent
    project_root = cf_dir.parent
    website_dir = project_root / "website"

    print_header(f"Building SvelteKit website for {environment}")

    if not website_dir.exists():
        print_error("Website directory not found")
        return False

    try:
        # Configure API endpoint and Stripe keys based on environment
        env_local_file = website_dir / ".env.local"

        # Check for Stripe publishable key
        stripe_key = env_vars.get("STRIPE_PUBLISHABLE_KEY")
        if not stripe_key:
            print_warning("STRIPE_PUBLISHABLE_KEY not found in environment config")
            print_warning("Credit card payments will NOT work without this key")
            print_info(f"Add to {cf_dir}/.env.{environment}:")
            print_info("  export STRIPE_PUBLISHABLE_KEY=pk_test_... (for dev)")
            print_info("  export STRIPE_PUBLISHABLE_KEY=pk_live_... (for prod)")
            print()
            # Don't fail - allow deployment without Stripe (DCT payments still work)

        with open(env_local_file, "w") as f:
            f.write("# Auto-generated by deploy.py - DO NOT EDIT\n")
            f.write(f"# Environment: {environment}\n")
            f.write("\n")

            if environment == "dev":
                f.write("# Development/staging API endpoint\n")
                f.write("VITE_DECENT_CLOUD_API_URL=https://dev-api.decent-cloud.org\n")
            else:  # prod
                f.write("# Production API endpoint (uses default from .env)\n")
                f.write("VITE_DECENT_CLOUD_API_URL=https://api.decent-cloud.org\n")

            f.write("\n")

            if stripe_key:
                f.write("# Stripe publishable key (safe to embed in client-side code)\n")
                key_type = "TEST" if stripe_key.startswith("pk_test_") else "LIVE"
                f.write(f"# Key type: {key_type}\n")
                f.write(f"VITE_STRIPE_PUBLISHABLE_KEY={stripe_key}\n")
                print_success(f"Configured Stripe {key_type} key for website build")
            else:
                f.write("# Stripe not configured - credit card payments disabled\n")
                f.write("# VITE_STRIPE_PUBLISHABLE_KEY=pk_test_...\n")

        print_success(f"Created .env.local for {environment} build")
        print()

        # Change to website directory and run build
        os.chdir(website_dir)
        subprocess.run(["npm", "run", "build"], check=True)
        print_success("Website built successfully")

        # Verify build output exists
        build_dir = website_dir / "build"
        if not build_dir.exists():
            print_error(f"Build directory not found at {build_dir}")
            return False

        print_success(f"Build output verified at {build_dir}")
        return True
    except subprocess.CalledProcessError as e:
        print_error(f"Website build failed: {e}")
        return False
    except FileNotFoundError:
        print_error("Node.js not found. Please install Node.js")
        return False
    except Exception as e:
        print_error(f"Unexpected error during website build: {e}")
        return False


def deploy(env_name: str, env_vars: dict[str, str], compose_files: list[str]) -> int:
    """Shared deployment logic for dev and prod environments."""
    is_prod = env_name == "prod"

    # Header
    print_header(f"Decent Cloud - {env_name.title()} Deployment")

    # Check Docker
    if not check_docker():
        return 1
    print()

    # Check for environment-specific .env file
    cf_dir = Path(__file__).parent
    env_file = cf_dir / f".env.{env_name}"

    if not env_file.exists():
        print_warning(f"Environment config not found: {env_file}")
        print()
        print(f"Create {env_file} with required configuration")
        print(f"See {cf_dir}/.env.dev or {cf_dir}/.env.prod for examples")
        print()
        return 1

    print_success(f"Found {env_file.name}")
    print()

    # Load all environment variables from file
    file_env_vars = load_env_file(env_file)
    if not file_env_vars:
        print_error(f"Failed to load environment variables from {env_file}")
        return 1

    # Merge file env vars into env_vars (file vars take precedence)
    env_vars.update(file_env_vars)

    # Verify tunnel token exists
    if not env_vars.get("TUNNEL_TOKEN"):
        if is_prod:
            print_error("TUNNEL_TOKEN not found in production config")
            print()
            print(f"Add TUNNEL_TOKEN to {env_file}")
            print(f"Get token from: https://one.dash.cloudflare.com/")
            print()
            return 1
        else:
            print_warning("TUNNEL_TOKEN not found - public access will not work")
            print_info(f"Add TUNNEL_TOKEN to {env_file} for public access")
            print()
    else:
        print_success("Tunnel token loaded")
        print()

    # Log loaded OAuth config (without showing secrets)
    if env_vars.get("GOOGLE_OAUTH_CLIENT_ID"):
        print_success("Google OAuth credentials loaded")
        print_info(f"  Redirect URL: {env_vars.get('GOOGLE_OAUTH_REDIRECT_URL', 'not set')}")
        print_info(f"  Frontend URL: {env_vars.get('FRONTEND_URL', 'not set')}")
        print()
    else:
        print_warning("Google OAuth not configured (optional)")
        print()

    # Build website natively first with environment-specific API configuration and Stripe keys
    if not build_website_natively(env_name, env_vars):
        print_error("Failed to build website")
        return 1
    print()

    # Build API server natively first
    if not build_rust_binaries_natively():
        print_error("Failed to build API server")
        return 1
    print()

    # Calculate binary hash for Docker cache invalidation
    binary_hash = calculate_binary_hash()
    env_vars["BINARY_HASH"] = binary_hash
    print_info(f"API binary hash: {binary_hash}")
    print()

    # Build and start services
    action = "production services" if is_prod else "services"
    print_warning(f"Building and starting {action}...")
    print()

    # Use a specific project name to isolate dev and prod environments
    project_name = f"decent-cloud-{env_name}"  # dev -> dev, prod -> prod

    if not run_docker_compose(compose_files, ["up", "-d", "--build", "--remove-orphans"], env_vars, project_name):
        print()
        print_error(f"{env_name.title()} deployment failed")
        print()
        compose_args = " ".join(f"-f {f}" for f in compose_files)
        project_args = f"-p {project_name}"
        print(f"Check logs: {BLUE}docker compose {project_args} {compose_args} logs{NC}")
        print()
        return 1

    # Success message
    print()
    print(f"{GREEN}========================================")
    if is_prod:
        print("Production Deployment Complete!")
    else:
        print("Development Deployment Complete!")
    print(f"========================================{NC}")
    print()
    print("Services started:")
    if is_prod:
        print("  • Decent Cloud Website (production)")
        print("  • Cloudflare Tunnel (api.decent-cloud.org)")
    else:
        print("  • Decent Cloud Website (development)")
        print("  • Cloudflare Tunnel (dev-api.decent-cloud.org)")
    print()

    # Check tunnel connection (both dev and prod now use tunnels)
    if env_vars.get("TUNNEL_TOKEN"):
        print_warning("Verifying tunnel connection...")
        import time

        time.sleep(5)

        status = check_tunnel_status(compose_files, env_name)

        if status == "connected":
            print_success("Tunnel connected successfully!")
            print()
            if is_prod:
                print("Your website is live at: https://decent-cloud.org")
                print("API available at: https://api.decent-cloud.org")
            else:
                print("Your website is live at: https://dev.decent-cloud.org")
                print("API available at: https://dev-api.decent-cloud.org")
            print()
            print("Verify deployment:")
            print("  • Check tunnel status in Cloudflare dashboard")
            domain = "decent-cloud.org" if is_prod else "dev.decent-cloud.org"
            print(f"  • Test your domain: https://{domain}/health")
        elif status == "unauthorized":
            print_error("Tunnel authentication failed!")
            print()
            print("Possible causes:")
            print("  1. Tunnel doesn't exist in Cloudflare dashboard")
            print("  2. Token is invalid or expired")
            print()
            print(f"Fix: {BLUE}python3 setup_tunnel.py{NC}")
            print()
            if is_prod:
                return 1
        else:
            msg = "Could not verify tunnel status" if status == "error" else "Tunnel status unclear"
            print_warning(msg)
            compose_args = " ".join(f"-f {f}" for f in compose_files)
            project_args = f"-p {project_name}"
            print(f"  {BLUE}docker compose {project_args} {compose_args} logs cloudflared{NC}")
            print()

    # Management commands
    print("Useful commands:" if not is_prod else "Management commands:")
    compose_args = " ".join(f"-f {f}" for f in compose_files)
    project_args = f"-p {project_name}"
    if is_prod:
        print(f"  View logs:    {BLUE}docker compose {project_args} {compose_args} logs -f{NC}")
        print(f"  Check status: {BLUE}docker compose {project_args} {compose_args} ps{NC}")
        print(f"  Restart:      {BLUE}docker compose {project_args} {compose_args} restart{NC}")
        print(f"  Stop:         {BLUE}docker compose {project_args} {compose_args} down{NC}")
    else:
        print(f"  {BLUE}docker compose {project_args} {compose_args} logs -f{NC}")
        print(f"  {BLUE}docker compose {project_args} {compose_args} ps{NC}")
        print(f"  {BLUE}docker compose {project_args} {compose_args} down{NC}")
    print()

    return 0


def main() -> int:
    """Main entry point."""
    parser = argparse.ArgumentParser(
        description="Deploy and manage Decent Cloud environments",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  %(prog)s deploy dev                 # Deploy to development
  %(prog)s deploy prod               # Deploy to production
  %(prog)s stop dev                  # Stop development services
  %(prog)s logs prod -f website      # Follow production website logs
  %(prog)s status dev                # Show development status
  %(prog)s restart prod             # Restart production services
        """,
    )

    subparsers = parser.add_subparsers(dest="command", help="Available commands")

    # Deploy command
    deploy_parser = subparsers.add_parser("deploy", aliases=["start", "up"], help="Deploy to environment")
    deploy_parser.add_argument("environment", choices=["dev", "development", "prod", "production"], help="Target environment")

    # Stop command
    stop_parser = subparsers.add_parser("stop", help="Stop environment services")
    stop_parser.add_argument("environment", choices=["dev", "development", "prod", "production"], help="Target environment")

    # Logs command
    logs_parser = subparsers.add_parser("logs", help="Show environment logs")
    logs_parser.add_argument("environment", choices=["dev", "development", "prod", "production"], help="Target environment")
    logs_parser.add_argument("-f", "--follow", action="store_true", help="Follow log output")
    logs_parser.add_argument(
        "service",
        nargs="?",
        choices=["website", "api-serve", "api-sync", "cloudflared"],
        help="Specific service to show logs for",
    )

    # Status command
    status_parser = subparsers.add_parser("status", help="Show environment status")
    status_parser.add_argument("environment", choices=["dev", "development", "prod", "production"], help="Target environment")

    # Restart command
    restart_parser = subparsers.add_parser("restart", help="Restart environment services")
    restart_parser.add_argument("environment", choices=["dev", "development", "prod", "production"], help="Target environment")

    args = parser.parse_args()

    if not args.command:
        parser.print_help()
        return 1

    # Normalize environment names
    env_map = {"dev": "dev", "development": "dev", "prod": "prod", "production": "prod"}
    environment = env_map[args.environment]

    # Execute command
    try:
        if args.command in ("deploy", "start", "up"):
            return deploy_environment(environment)
        elif args.command == "stop":
            return stop_environment(environment)
        elif args.command == "logs":
            return show_logs(environment, args.follow, args.service)
        elif args.command == "status":
            return show_status(environment)
        elif args.command == "restart":
            return restart_environment(environment)
        else:
            print_error(f"Unknown command: {args.command}")
            return 1
    except KeyboardInterrupt:
        print_warning("\nOperation cancelled by user")
        return 130
    except Exception as e:
        print_error(f"Unexpected error: {e}")
        return 1


if __name__ == "__main__":
    sys.exit(main())
