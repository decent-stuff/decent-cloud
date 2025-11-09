#!/usr/bin/env python3
"""Deployment script with dev/prod environments and management commands."""

import os
import subprocess
import sys
from pathlib import Path
from typing import Optional
import argparse

def get_env_config(environment: str) -> tuple[dict[str, str], list[str]]:
    """Get environment-specific configuration."""
    cf_dir = Path(__file__).parent

    if environment == "production":
        env_vars = {"ENVIRONMENT": "production", "NETWORK_NAME": "decent-cloud-prod"}
        compose_files = [str(cf_dir / "docker-compose.yml"), str(cf_dir / "docker-compose.prod.yml")]
    else:  # development
        env_vars = {"ENVIRONMENT": "development", "NETWORK_NAME": "decent-cloud-dev"}
        compose_files = [str(cf_dir / "docker-compose.yml"), str(cf_dir / "docker-compose.dev.yml")]

    return env_vars, compose_files


def deploy_environment(environment: str) -> int:
    """Deploy to specified environment."""
    env_vars, compose_files = get_env_config(environment)

    return deploy(environment, env_vars, compose_files)


def stop_environment(environment: str) -> int:
    """Stop services for specified environment."""
    env_vars, compose_files = get_env_config(environment)
    project_name = f"decent-cloud-{environment[:4]}"
    env_vars["COMPOSE_PROJECT_NAME"] = project_name

    print_header(f"Stopping {environment} services")

    if not run_docker_compose(compose_files, ["down"], env_vars):
        print_error(f"Failed to stop {environment} services")
        return 1

    print_success(f"{environment.title()} services stopped successfully")
    return 0


def show_logs(environment: str, follow: bool = False, service: Optional[str] = None) -> int:
    """Show logs for specified environment."""
    env_vars, compose_files = get_env_config(environment)
    project_name = f"decent-cloud-{environment[:4]}"
    env_vars["COMPOSE_PROJECT_NAME"] = project_name

    print_header(f"{environment.title()} logs")

    cmd = ["logs"]
    if follow:
        cmd.append("-f")
    if service:
        cmd.append(service)

    if not run_docker_compose(compose_files, cmd, env_vars):
        print_error(f"Failed to get logs for {environment}")
        return 1

    return 0


def show_status(environment: str) -> int:
    """Show status for specified environment."""
    env_vars, compose_files = get_env_config(environment)
    project_name = f"decent-cloud-{environment[:4]}"
    env_vars["COMPOSE_PROJECT_NAME"] = project_name

    print_header(f"{environment.title()} status")

    if not run_docker_compose(compose_files, ["ps"], env_vars):
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
    project_name = f"decent-cloud-{environment[:4]}"
    env_vars["COMPOSE_PROJECT_NAME"] = project_name

    print_header(f"Restarting {environment} services")

    if not run_docker_compose(compose_files, ["restart"], env_vars):
        print_error(f"Failed to restart {environment} services")
        return 1

    print_success(f"{environment.title()} services restarted successfully")
    return 0


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

def check_tunnel_status(compose_files: list[str], env_name: str) -> str:
    """Check tunnel connection status from logs."""
    try:
        project_name = f"decent-cloud-{env_name[:4]}"  # dev -> dev, prod -> prod
        env_vars = {'COMPOSE_PROJECT_NAME': project_name}

        result = subprocess.run(
            ["docker", "compose"] +
            [arg for f in compose_files for arg in ["-f", f]] +
            ["logs", "cloudflared"],
            capture_output=True,
            text=True,
            env={**os.environ, **env_vars},
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

    # Check wasm-pack
    try:
        subprocess.run(["wasm-pack", "--version"], check=True, capture_output=True)
        print_success("wasm-pack found")
    except (subprocess.CalledProcessError, FileNotFoundError):
        print_info("Installing wasm-pack...")
        try:
            subprocess.run(["cargo", "install", "wasm-pack"], check=True)
            print_success("wasm-pack installed")
        except subprocess.CalledProcessError:
            print_error("Failed to install wasm-pack")
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


def build_api_natively() -> bool:
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
        subprocess.run([
            "cargo", "build", "--release", "--bin", "api-server",
            "--target", "x86_64-unknown-linux-gnu"
        ], check=True)

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

def build_website_natively() -> bool:
    """Build website natively before Docker build."""
    cf_dir = Path(__file__).parent
    website_dir = cf_dir.parent / "website"

    print_header("Building website natively")

    if not website_dir.exists():
        print_error("Website directory not found")
        return False

    try:
        # Change to website directory and run build
        os.chdir(website_dir)
        subprocess.run(["npm", "run", "build"], check=True)
        print_success("Website built successfully")
        return True
    except subprocess.CalledProcessError as e:
        print_error(f"Website build failed: {e}")
        print_error(f"stdout: {e.stdout}")
        print_error(f"stderr: {e.stderr}")
        return False
    except FileNotFoundError:
        print_error("Node.js not found. Please install Node.js")
        return False
    except Exception as e:
        print_error(f"Unexpected error during website build: {e}")
        return False


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

    if is_prod:
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

    # Build website natively first
    if not build_website_natively():
        print_error("Failed to build website")
        return 1
    print()

    # Build API server natively first
    if not build_api_natively():
        print_error("Failed to build API server")
        return 1
    print()

    # Build and start services
    action = "production services" if is_prod else "services"
    print_warning(f"Building and starting {action}...")
    print()

    # Use a specific project name to isolate dev and prod environments
    project_name = f"decent-cloud-{env_name[:4]}"  # dev -> dev, prod -> prod
    env_vars_with_project = {**env_vars, 'COMPOSE_PROJECT_NAME': project_name}

    if not run_docker_compose(compose_files, ["up", "-d", "--build", "--remove-orphans"], env_vars_with_project):
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

    if is_prod:
        # Check tunnel connection
        print_warning("Verifying tunnel connection..." if is_prod else "Checking tunnel connection...")
        import time
        time.sleep(5)

        status = check_tunnel_status(compose_files, env_name)

        if status == "connected":
            print_success("Tunnel connected successfully!")
            print()
            if is_prod:
                print("Your website is now live and accessible")
                print()
                print("Verify deployment:")
                print("  • Check tunnel status in Cloudflare dashboard")
                print("  • Test your domain: https://your-domain.com/health")
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
    deploy_parser = subparsers.add_parser("deploy", help="Deploy to environment")
    deploy_parser.add_argument("environment", choices=["dev", "development", "prod", "production"], help="Target environment")

    # Stop command
    stop_parser = subparsers.add_parser("stop", help="Stop environment services")
    stop_parser.add_argument("environment", choices=["dev", "development", "prod", "production"], help="Target environment")

    # Logs command
    logs_parser = subparsers.add_parser("logs", help="Show environment logs")
    logs_parser.add_argument("environment", choices=["dev", "development", "prod", "production"], help="Target environment")
    logs_parser.add_argument("-f", "--follow", action="store_true", help="Follow log output")
    logs_parser.add_argument(
        "service", nargs="?", choices=["website", "api-serve", "api-sync", "cloudflared"], help="Specific service to show logs for"
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
    env_map = {"dev": "development", "development": "development", "prod": "production", "production": "production"}
    environment = env_map[args.environment]

    # Execute command
    try:
        if args.command == "deploy":
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
