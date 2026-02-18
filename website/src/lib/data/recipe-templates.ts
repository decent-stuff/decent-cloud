export interface RecipeTemplate {
	key: string;
	label: string;
	script: string;
}

export const RECIPE_TEMPLATES: RecipeTemplate[] = [
	{
		key: 'docker',
		label: 'Docker container',
		script: `#!/bin/bash
set -euo pipefail

# Install Docker
curl -fsSL https://get.docker.com | sh

# Pull and run your container
# docker run -d --name myapp -p 10001:80 nginx:latest

echo "Docker installed. Edit this script to run your container."
`
	},
	{
		key: 'docker-compose',
		label: 'Docker Compose stack',
		script: `#!/bin/bash
set -euo pipefail

# Install Docker + Compose
curl -fsSL https://get.docker.com | sh

# Write your compose file
cat > /opt/docker-compose.yml << 'COMPOSE'
services:
  web:
    image: nginx:latest
    ports:
      - "10001:80"
    restart: unless-stopped
COMPOSE

# Start the stack
cd /opt && docker compose -f docker-compose.yml up -d

echo "Docker Compose stack running. Edit /opt/docker-compose.yml to customize."
`
	},
	{
		key: 'podman',
		label: 'Podman container',
		script: `#!/bin/bash
set -euo pipefail

# Install Podman
apt-get update && apt-get install -y podman

# Pull and run your container
# podman run -d --name myapp -p 10001:80 docker.io/library/nginx:latest

echo "Podman installed. Edit this script to run your container."
`
	},
	{
		key: 'nodejs',
		label: 'Node.js application',
		script: `#!/bin/bash
set -euo pipefail

# Install Node.js LTS
curl -fsSL https://deb.nodesource.com/setup_lts.x | bash -
apt-get install -y nodejs

# Install PM2 for process management
npm install -g pm2

# Clone your app (edit the URL)
# git clone https://github.com/your/repo.git /opt/app
# cd /opt/app && npm install && pm2 start npm -- start

echo "Node.js + PM2 installed. Edit this script to deploy your app."
`
	},
	{
		key: 'caddy',
		label: 'Static site (Caddy)',
		script: `#!/bin/bash
set -euo pipefail

# Install Caddy
apt-get update && apt-get install -y debian-keyring debian-archive-keyring apt-transport-https curl
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | tee /etc/apt/sources.list.d/caddy-stable.list
apt-get update && apt-get install -y caddy

# Place your site files in /var/www/html
mkdir -p /var/www/html
echo "<h1>Hello from Decent Cloud</h1>" > /var/www/html/index.html

# Configure Caddy to serve on port 10001
cat > /etc/caddy/Caddyfile << 'EOF'
:10001 {
    root * /var/www/html
    file_server
}
EOF

systemctl restart caddy
echo "Caddy serving on port 10001."
`
	}
];
