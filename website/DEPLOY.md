# NoteCapt Website Deployment

## Server Info

- **Host**: `101.47.159.9`
- **User**: `root`
- **Password**: `Aa/12345678`
- **SSH**: `ssh root@101.47.159.9`

## Deployment

- **Web Root**: `/var/www/notecapt/`
- **Nginx Config**: `/etc/nginx/conf.d/*.conf`
- **Domain**: `www.notecapt.cn` / `notecapt.cn`
- **Port**: 80

## Deploy Steps

```bash
# 1. Build locally
npm run build

# 2. Package dist
tar -czf /tmp/notecapt-dist.tar.gz -C dist .

# 3. Upload to server
scp /tmp/notecapt-dist.tar.gz root@101.47.159.9:/tmp/

# 4. SSH in and deploy
ssh root@101.47.159.9
cd /var/www/notecapt
tar -xzf /tmp/notecapt-dist.tar.gz -C /var/www/notecapt/
nginx -s reload
```
