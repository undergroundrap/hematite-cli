# SearXNG Local Setup Guide (Windows + Docker)

Connect Hematite to a local SearXNG instance for private, unlimited, and high-fidelity web search.

## 🚀 The Fastest Path (Automated)

If you are on Windows, you can use the automated scaffolding script included in the repo:

1. Open PowerShell in the Hematite root.
2. Run the script:
   ```powershell
   powershell -ExecutionPolicy Bypass -File scripts/setup-searxng.ps1
   ```
3. **Move/Backup**: You can now move the `searxng-scaffold` folder anywhere (e.g., your Projects directory). The Docker container remains managed globally.

## Managing the Engine

Hematite now automates much of the upkeep, but you can manually control the SearXNG backend using these commands:

### Verify Service Health
Hematite performs a heartbeat check at startup. You can manually check if it's responding:
```powershell
curl http://localhost:8080
```

### Stopping the Engine
If you want to free up resources or stop the search capability:
```powershell
docker stop searxng
```

### Restarting / Manual Boot
If the engine is offline, you can start it again with:
```powershell
docker start searxng
```
Or simply rerun the setup script:
```powershell
./scripts/setup-searxng.ps1
```

### Auto-Boot Feature
By default, Hematite v0.5.7+ will attempt to **automatically start** the `searxng` container if it detects it is offline during startup. 

To disable this behavior, edit your `.hematite/settings.json`:
```json
{
  "auto_start_searx": false
}
```

## Troubleshooting
- **Docker Not Found**: ensure Docker Desktop is running.
- **Port Conflict**: If port `8080` is taken, edit the `docker-run` command in `scripts/setup-searxng.ps1` to map to a different host port (e.g. `-p 8888:8080`) and update your Hematite `searx_url` setting.

---

## 🛠️ The Manual Path
- **Docker Desktop** installed and running on Windows.
- At least 1GB of free RAM for the SearXNG containers.

## 2. Directory Structure
Create a dedicated folder for your SearXNG instance:
```powershell
mkdir searxng-local
cd searxng-local
mkdir searxng
```

## 3. Configuration Files

### `docker-compose.yaml`
Create this in your `searxng-local` root:
```yaml
services:
  searxng:
    container_name: searxng
    image: docker.io/searxng/searxng:latest
    restart: always
    networks:
      - searxng
    ports:
      - "8080:8080"
    volumes:
      - ./searxng:/etc/searxng:rw
    environment:
      - SEARXNG_BASE_URL=http://localhost:8080/
    cap_drop:
      - ALL
    cap_add:
      - CHOWN
      - SETGID
      - SETUID
    logging:
      driver: "json-file"
      options:
        max-size: "1m"
        max-file: "1"

networks:
  searxng:
    ipam:
      driver: default
```

### `searxng/settings.yml`
Create this in the `searxng/` subfolder. **IMPORTANT**: The `formats` section must include `json` for Hematite to work.

```yaml
use_default_settings: true

server:
  # Secret key is required for the container to start.
  # You can regenerate this with: openssl rand -hex 32
  secret_key: "CHANGE_ME_TO_SOMETHING_RANDOM"
  limiter: false # Disable rate limiter for local dev
  image_proxy: true

search:
  safe_search: 0
  autocomplete: ""
  formats:
    - html
    - json # REQUIRED for Hematite

engines:
  - name: duckduckgo
    engine: duckduckgo
    shortcut: ddg
  - name: google
    engine: google
    shortcut: g
    use_official_api: false
  - name: bing
    engine: bing
    shortcut: b

ui:
  static_use_hash: true
  query_in_title: true
```

## 4. Launch
From your `searxng-local` folder, run:
```powershell
docker compose up -d
```

## 5. Verify Setup
Run this command in any terminal:
```powershell
curl "http://localhost:8080/search?q=hematite&format=json"
```
If you see a wall of JSON text, you are ready!

## 6. Configure Hematite
Hematite v0.6.0+ now **auto-detects** SearXNG on port 8080. If you have it running, it will automatically use it!

If you moved it to a different port, update `.hematite/settings.json`:
```json
{
  "searx_url": "http://localhost:8888"
}
```
