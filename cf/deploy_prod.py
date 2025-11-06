#!/usr/bin/env python3
"""Deploy Decent Cloud website to production environment with Cloudflare Tunnel."""

import sys
from pathlib import Path
from cf_common import deploy

def main() -> int:
    """Main deployment function."""
    cf_dir = Path(__file__).parent

    env_vars = {
        'ENVIRONMENT': 'production',
        'NETWORK_NAME': 'decent-cloud-prod'
    }

    compose_files = [
        str(cf_dir / 'docker-compose.yml'),
        str(cf_dir / 'docker-compose.prod.yml')
    ]

    return deploy('production', env_vars, compose_files)

if __name__ == "__main__":
    sys.exit(main())
