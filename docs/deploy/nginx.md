# nginx Reverse Proxy

Put nginx in front of arrgh to get HTTPS and a clean domain.

## Basic config

```nginx
server {
    listen 80;
    server_name arrgh.example.com;
    return 301 https://$host$request_uri;
}

server {
    listen 443 ssl;
    server_name arrgh.example.com;

    ssl_certificate     /etc/letsencrypt/live/arrgh.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/arrgh.example.com/privkey.pem;

    # Increase for chapter image proxying
    proxy_buffer_size        128k;
    proxy_buffers            4 256k;
    proxy_busy_buffers_size  256k;

    location / {
        proxy_pass         http://localhost:8080;
        proxy_set_header   Host $host;
        proxy_set_header   X-Real-IP $remote_addr;
        proxy_set_header   X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header   X-Forwarded-Proto $scheme;
        proxy_read_timeout 120s;
    }
}
```

## With Docker Compose

Bind arrgh to localhost only so it's not directly reachable:

```yaml
arrgh:
  ports:
    - "127.0.0.1:8080:8080"
```

Then use the nginx config above. Certbot manages the SSL cert:

```bash
certbot --nginx -d arrgh.example.com
```

## LAN-only (no domain, self-signed)

For a NAS or home server with no public domain:

```bash
openssl req -x509 -nodes -days 3650 -newkey rsa:2048 \
  -keyout arrgh.key -out arrgh.crt \
  -subj "/CN=arrgh.local"
```

```nginx
server {
    listen 443 ssl;
    server_name arrgh.local;
    ssl_certificate     /etc/nginx/ssl/arrgh.crt;
    ssl_certificate_key /etc/nginx/ssl/arrgh.key;
    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_read_timeout 120s;
    }
}
```
