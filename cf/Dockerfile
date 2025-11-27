# Simplified build for Decent Cloud website
# Assumes website is built natively first
# Serves pre-built SvelteKit static website with nginx

# Runtime with nginx
FROM --platform=linux/amd64 nginx:alpine

# Install runtime dependencies
RUN apk add --no-cache curl ca-certificates

# Copy nginx config
COPY <<EOF /etc/nginx/conf.d/default.conf
server {
    listen 3000;
    server_name _;

    # Serve static website
    root /usr/share/nginx/html;
    index index.html;

    # Serve static assets directly - return 404 if missing
    location ~* \.(ico|png|svg|jpg|jpeg|js|css|json|txt|xml|woff|woff2|ttf|eot)$ {
        expires 1y;
        add_header Cache-Control "public, immutable";
        try_files \$uri =404;
    }

    # SvelteKit build files - return 404 if missing
    location /_app/ {
        expires 1y;
        add_header Cache-Control "public, immutable";
        try_files \$uri =404;
    }

    # Images directory - return 404 if missing
    location /images/ {
        expires 1y;
        add_header Cache-Control "public, immutable";
        try_files \$uri =404;
    }

    # Docs directory - serve files directly with proper mime types
    location /docs/ {
        expires 1y;
        add_header Cache-Control "public, immutable";
        try_files \$uri =404;
    }

    # Favicon - return 404 if missing
    location /favicon.svg {
        expires 1y;
        add_header Cache-Control "public, immutable";
        try_files \$uri =404;
    }

    # SPA routing for HTML requests only - fallback to index.html
    # This ensures that only navigation requests get the SPA fallback
    location / {
        try_files \$uri \$uri/index.html /index.html;
    }

    # Health check endpoint
    location /health {
        access_log off;
        return 200 "healthy";
        add_header Content-Type text/plain;
    }
}
EOF

# Copy pre-built website output (assumes website was built natively first)
COPY website/build /usr/share/nginx/html

EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

CMD ["nginx", "-g", "daemon off;"]
