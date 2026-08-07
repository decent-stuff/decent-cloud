# Simplified build for Decent Cloud website
# Assumes website is built natively first
# Serves pre-built SvelteKit static website with nginx

# Runtime with nginx
FROM --platform=linux/amd64 nginx:alpine

# Install runtime dependencies
RUN apk add --no-cache curl ca-certificates

# Copy nginx config
COPY cf/nginx-default.conf /etc/nginx/conf.d/default.conf

# Copy pre-built website output (assumes website was built natively first)
COPY website/build /usr/share/nginx/html

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:59010/health || exit 1

CMD ["nginx", "-g", "daemon off;"]
