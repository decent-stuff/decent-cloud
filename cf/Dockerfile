# Simplified build for Decent Cloud website
# Assumes WASM modules are built natively first
# Builds Next.js static website and serves with nginx

# Simplified build for Decent Cloud website
# Assumes WASM modules and website are built natively first
# Serves pre-built Next.js static website with nginx

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

    # SPA routing - try files, fallback to index.html
    location / {
        try_files \$uri \$uri/ /index.html;
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
COPY website/out /usr/share/nginx/html

EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

CMD ["nginx", "-g", "daemon off;"]