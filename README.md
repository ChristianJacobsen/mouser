# Mouser

Dynamic seedbox IP reporter for [MyAnonaMouse](https://www.myanonamouse.net). Periodically checks the host's public IP and reports changes to MAM's dynamic seedbox API.

Based on the backend logic of [t-mart/mousehole](https://github.com/t-mart/mousehole), rewritten in Rust.

## Configuration

All configuration is via environment variables.

| Variable | Default | Description |
|----------|---------|-------------|
| `MOUSER_MAM_ID` | — | MAM session cookie (required for updates) |
| `MOUSER_PORT` | `7878` | HTTP server port |
| `MOUSER_STATE_FILE` | `/data/mouser/state.json` | Path to persistent state file |
| `MOUSER_CHECK_INTERVAL` | `300` | Seconds between IP checks |
| `MOUSER_STALE_THRESHOLD` | `86400` | Seconds before forcing a re-report |
| `MOUSER_USER_AGENT` | `mouser/0.1.0` | User-Agent for MAM API requests |
| `LOG_LEVEL` | `info` | Log level filter (`debug`, `info`, `warn`, `error`) |

## API

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/health` | `200` if synced, `503` if stale/no cookie/last error |
| `GET` | `/state` | Current persistent state as JSON |
| `PUT` | `/state` | Update cookie: `{"cookie": "..."}` |
| `POST` | `/update` | Trigger immediate check/update cycle |

## Running

```sh
MOUSER_MAM_ID=your_mam_id MOUSER_STATE_FILE=/tmp/mouser.json cargo run
```

## Docker

```sh
docker build -t mouser .
docker run -e MOUSER_MAM_ID=your_mam_id -v mouser-data:/data/mouser mouser
```
