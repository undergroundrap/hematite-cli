# SearXNG Local Setup Guide (Windows + Docker)

Connect Hematite to a local SearXNG instance for private, unlimited, and high-fidelity web search.

## Why Local Search?

- **Privacy-first**: your search queries stay behind your own SearXNG proxy instead of being tied directly to your identity.
- **Unlimited research volume**: no per-query billing, no API quotas, no central search proxy bottleneck.
- **No tracking**: your search history is not being logged by a third-party agent platform.
- **Authoritative technical truth**: Hematite can look up current API versions, package releases, and runtime behavior beyond model cutoff dates.
- **Zero ongoing cost**: once Docker is installed, the research layer runs locally on your machine.
- **Hardened defaults**: Hematite ships an opinionated SearXNG scaffold tuned for technical research instead of making you assemble it manually.
- **Safer upstream posture**: the default scaffold now favors a smaller technical-source pool rather than the older broad 12-engine profile, reducing unnecessary upstream query fan-out and lowering the chance of rate limits or bans.

## The Fastest Path

Hematite v0.6.0+ can now scaffold and boot the local search stack for you.

1. Install Docker Desktop and make sure the Docker daemon is running.
2. Leave `auto_start_searx` enabled in `.hematite/settings.json`, or set it explicitly:
   ```json
   {
     "auto_start_searx": true
   }
   ```
3. Launch Hematite.

If `searx_url` is unset or points at a local address such as `http://localhost:8080`, Hematite will:

1. Scaffold the stack under `~/.hematite/searxng-local` unless `HEMATITE_SEARX_ROOT` overrides it.
2. Detect whether SearXNG is already reachable.
3. Run `docker compose up -d` if it is offline.
4. Wait for the local search endpoint to respond before continuing startup.

If the service is already running, Hematite reuses it and does not blindly restart the stack.

If Docker Desktop is missing or the daemon is offline, Hematite now surfaces a compact startup note telling you exactly what is wrong instead of failing silently.

You can still scaffold the files manually from the repo root:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/setup-searxng.ps1
```

## Managing the Engine

Hematite now automates much of the upkeep, but you can manually control the SearXNG backend too.

### Verify Service Health

```powershell
curl http://localhost:8080
```

Or verify the JSON API directly:

```powershell
curl "http://localhost:8080/search?q=hematite&format=json"
```

### Stopping the Engine

```powershell
cd $HOME\.hematite\searxng-local
docker compose down
```

### Restarting / Manual Boot

```powershell
cd $HOME\.hematite\searxng-local
docker compose up -d
```

### Default Engine Profile

The current scaffold defaults to this safer technical pool:

- duckduckgo
- brave
- mojeek
- wikipedia
- github
- stackoverflow
- npm
- crates.io

That is a better fit for a personal development machine than the older broad 12-engine setup, which created more upstream traffic and more opportunities for rate limits.

If you want to widen it later, edit `searxng/settings.yml` manually.

### Auto-Boot and Auto-Stop

By default, Hematite v0.6.0+ will attempt to automatically start the local SearXNG stack if:

- `auto_start_searx` is `true`
- `searx_url` is unset or points at a local address
- Docker Desktop is installed and running

To disable startup automation:

```json
{
  "auto_start_searx": false
}
```

If you want Hematite to stop only the SearXNG instance it started in the current session when the app exits:

```json
{
  "auto_stop_searx": true
}
```

Hematite only auto-stops session-owned stacks. It does not blindly tear down a SearXNG instance that was already running before Hematite started.

## Troubleshooting

- **Docker not found**: install Docker Desktop or set `auto_start_searx` to `false`.
- **Docker daemon offline**: start Docker Desktop, then relaunch Hematite.
- **Port conflict**: edit `docker-compose.yaml` in your SearXNG root to map a different host port, then update `searx_url` in `.hematite/settings.json`.
- **Custom location**: set `HEMATITE_SEARX_ROOT` if you want the stack outside `~/.hematite/searxng-local`.
- **Already running elsewhere**: point `searx_url` at that instance and Hematite will use it instead of trying to manage a local stack.

---

## The Manual Path

- Docker Desktop installed and running on Windows.
- At least 1 GB of free RAM for the SearXNG containers.

## 1. Directory Structure

Create a dedicated folder for your SearXNG instance:

```powershell
mkdir searxng-local
cd searxng-local
mkdir searxng
```

## 2. Configuration Files

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

Create this in the `searxng/` subfolder. The `formats` section must include `json` for Hematite to work.

```yaml
use_default_settings: true

server:
  secret_key: "CHANGE_ME_TO_SOMETHING_RANDOM"
  limiter: false
  image_proxy: true

search:
  safe_search: 0
  autocomplete: ""
  formats:
    - html
    - json

engines:
  - name: google
    engine: google
    shortcut: g
    use_official_api: false
  - name: duckduckgo
    engine: duckduckgo
    shortcut: ddg
  - name: bing
    engine: bing
    shortcut: b
  - name: brave
    engine: brave
    shortcut: br
  - name: qwant
    engine: qwant
    shortcut: qw
  - name: startpage
    engine: startpage
    shortcut: sp
  - name: mojeek
    engine: mojeek
    shortcut: mj
  - name: wikipedia
    engine: wikipedia
    shortcut: wp
  - name: github
    engine: github
    shortcut: gh
  - name: stackoverflow
    engine: stackexchange
    shortcut: so
  - name: crates.io
    engine: crates
    shortcut: crio

ui:
  static_use_hash: true
  query_in_title: true
```

## 3. Launch

From your `searxng-local` folder, either double-click `start_searx.bat` or run:

```powershell
docker compose up -d
```

## 4. Configure Hematite

Hematite uses `http://localhost:8080` by default for local search. If your SearXNG instance is there and reachable, `research_web` will use it automatically.

If you moved it to a different port, update `.hematite/settings.json`:

```json
{
  "searx_url": "http://localhost:8888"
}
```

If you want Hematite to manage that custom local instance too:

```json
{
  "searx_url": "http://localhost:8888",
  "auto_start_searx": true,
  "auto_stop_searx": false
}
```
