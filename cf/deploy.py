#!/usr/bin/env python3
"""Unified deployment script for Decent Cloud with dev/prod environments and management commands."""

import argparse
import os
import subprocess
import sys
from pathlib import Path
from typing import Optional

# Import shared utilities
from cf_common import (
    print_header, print_success, print_error, print_warning, print_info,
    check_docker, load_env_file, get_tunnel_token, run_docker_compose,
    check_tunnel_status
)

def get_env_config(environment: str) -> tuple[dict[str, str], list[str]]:
    """Get environment-specific configuration."""
    cf_dir = Path(__file__).parent
    
    if environment == "production":
        env_vars = {
            'ENVIRONMENT': 'production',
            'NETWORK_NAME': 'decent-cloud-prod'
        }
        compose_files = [
            str(cf_dir / 'docker-compose.yml'),
            str(cf_dir / 'docker-compose.prod.yml')
        ]
    else:  # development
        env_vars = {
            'ENVIRONMENT': 'development',
            'NETWORK_NAME': 'decent-cloud-dev'
        }
        compose_files = [
            str(cf_dir / 'docker-compose.yml'),
            str(cf_dir / 'docker-compose.dev.yml')
        ]
    
    return env_vars, compose_files

def deploy_environment(environment: str) -> int:
    """Deploy to specified environment."""
    env_vars, compose_files = get_env_config(environment)
    
    # Import deploy function from cf_common
    from cf_common import deploy
    return deploy(environment, env_vars, compose_files)

def stop_environment(environment: str) -> int:
    """Stop services for specified environment."""
    env_vars, compose_files = get_env_config(environment)
    project_name = f"decent-cloud-{environment[:4]}"
    env_vars['COMPOSE_PROJECT_NAME'] = project_name
    
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
    env_vars['COMPOSE_PROJECT_NAME'] = project_name
    
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
    env_vars['COMPOSE_PROJECT_NAME'] = project_name
    
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
    env_vars['COMPOSE_PROJECT_NAME'] = project_name
    
    print_header(f"Restarting {environment} services")
    
    if not run_docker_compose(compose_files, ["restart"], env_vars):
        print_error(f"Failed to restart {environment} services")
        return 1
    
    print_success(f"{environment.title()} services restarted successfully")
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
        """
    )
    
    subparsers = parser.add_subparsers(dest='command', help='Available commands')
    
    # Deploy command
    deploy_parser = subparsers.add_parser('deploy', help='Deploy to environment')
    deploy_parser.add_argument('environment', choices=['dev', 'development', 'prod', 'production'],
                             help='Target environment')
    
    # Stop command
    stop_parser = subparsers.add_parser('stop', help='Stop environment services')
    stop_parser.add_argument('environment', choices=['dev', 'development', 'prod', 'production'],
                           help='Target environment')
    
    # Logs command
    logs_parser = subparsers.add_parser('logs', help='Show environment logs')
    logs_parser.add_argument('environment', choices=['dev', 'development', 'prod', 'production'],
                           help='Target environment')
    logs_parser.add_argument('-f', '--follow', action='store_true',
                           help='Follow log output')
    logs_parser.add_argument('service', nargs='?', choices=['website', 'api', 'cloudflared'],
                           help='Specific service to show logs for')
    
    # Status command
    status_parser = subparsers.add_parser('status', help='Show environment status')
    status_parser.add_argument('environment', choices=['dev', 'development', 'prod', 'production'],
                             help='Target environment')
    
    # Restart command
    restart_parser = subparsers.add_parser('restart', help='Restart environment services')
    restart_parser.add_argument('environment', choices=['dev', 'development', 'prod', 'production'],
                             help='Target environment')
    
    args = parser.parse_args()
    
    if not args.command:
        parser.print_help()
        return 1
    
    # Normalize environment names
    env_map = {
        'dev': 'development',
        'development': 'development',
        'prod': 'production', 
        'production': 'production'
    }
    environment = env_map[args.environment]
    
    # Execute command
    try:
        if args.command == 'deploy':
            return deploy_environment(environment)
        elif args.command == 'stop':
            return stop_environment(environment)
        elif args.command == 'logs':
            return show_logs(environment, args.follow, args.service)
        elif args.command == 'status':
            return show_status(environment)
        elif args.command == 'restart':
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
