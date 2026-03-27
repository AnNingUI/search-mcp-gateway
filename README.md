# search-mcp-gateway

`search-mcp-gateway` is a Rust project that exposes the same search gateway
capabilities through a local MCP server and a direct CLI. It is designed to
keep MCP support complete while also supporting lazy-loading through skills
that invoke the CLI only when needed.

## Features

- Unified search gateway with provider-agnostic request and response models.
- Local `stdio` MCP server with a compact tool surface.
- CLI with equivalent `search`, `extract`, `crawl`, and `status` subcommands.
- Automatic provider selection, health-aware rotation, retryable fallback, and
  circuit breaking.
- Default search routing prefers Tavily for richer answer-oriented queries,
  while DuckDuckGo remains available as a lightweight fallback.
- Stable JSON envelope output for MCP and CLI JSON mode:
  `{ "ok": true|false, "data": ..., "error": ... }`.
- DuckDuckGo compatibility strategies across both `html` and `lite` endpoints
  with POST and GET request fallbacks.
- Tavily and DuckDuckGo providers in the first version.

## Commands

Run the CLI directly:

```powershell
cargo run -- search --query "latest AI agent news" --include-answer --json
```

The `--json` flag prints a stable envelope:

```json
{
  "ok": true,
  "data": {
    "provider_used": "tavily"
  },
  "error": null
}
```

Run the local MCP server:

```powershell
cargo run -- mcp
```

## CI and releases

The repository includes GitHub Actions workflows for validation and tagged
releases. The CI workflow builds and tests the project on Linux, Windows, and
macOS. The release workflow builds tagged binaries and uploads them to GitHub
Releases.

### CI workflow

The CI workflow runs on pushes to `main` and on pull requests. It currently
builds these targets:

- `x86_64-unknown-linux-gnu`
- `x86_64-pc-windows-msvc`
- `aarch64-apple-darwin`

Each matrix job runs:

1. `cargo test --quiet --target <target>`
2. `cargo build --release --target <target>`

### Release workflow

The release workflow runs when you push a tag that matches `v*`. For example:

```bash
git tag v0.1.0
git push origin v0.1.0
```

The workflow uploads these release assets:

- Linux archive: `search-mcp-gateway-x86_64-unknown-linux-gnu.tar.gz`
- Windows archive: `search-mcp-gateway-x86_64-pc-windows-msvc.zip`
- macOS archive: `search-mcp-gateway-aarch64-apple-darwin.tar.gz`
- `SHA256SUMS`

### Dependency updates

The repository includes `dependabot.yml` for two update streams:

- GitHub Actions versions
- Cargo dependencies

This keeps the workflow actions and Rust dependencies moving forward through
normal pull requests instead of silently drifting out of date.

## Configuration

Configuration is optional. If you don't provide a config file, the gateway
starts with built-in defaults and only reads secrets such as the Tavily token
from environment variables.

### Configuration loading order

The gateway resolves configuration in the following order:

1. If you pass `--config <path>`, it uses that file.
2. Otherwise, if `SEARCH_MCP_GATEWAY_CONFIG` is set, it uses that path.
3. Otherwise, it looks for `search-mcp-gateway.toml` in the current working
   directory.
4. If no file exists, it uses built-in defaults.

### Minimal setup

The smallest working setup is an environment variable only. This is enough for
the built-in Tavily provider.

On Windows PowerShell:

```powershell
$env:TAVILY_HIKARI_TOKEN = "your-token"
cargo run -- search --query "latest AI agent news" --json
```

If you run the installed binary directly:

```powershell
$env:TAVILY_HIKARI_TOKEN = "your-token"
<path-to-search-mcp-gateway> search --query "latest AI agent news" --json
```

### Example config file

If you want to override defaults, create `search-mcp-gateway.toml`:

```toml
[gateway]
default_timeout_ms = 20000
cache_enabled = true
cache_ttl_seconds = 120
circuit_failure_threshold = 3
circuit_open_seconds = 30
search_provider_order = ["tavily", "ddg"]

[tavily]
enabled = true
base_url = "https://tavily.ivanli.cc"
api_key_env = "TAVILY_HIKARI_TOKEN"
search_path = "/api/tavily/search"
extract_path = "/api/tavily/extract"
crawl_path = "/api/tavily/crawl"

[ddg]
enabled = true
base_url = "https://html.duckduckgo.com/html/"
lite_url = "https://lite.duckduckgo.com/lite/"
region = "wt-wt"
safe_search = "moderate"
user_agent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36"
```

### Config reference

These are the currently supported configuration keys.

#### `[gateway]`

The `[gateway]` section controls request flow, caching, circuit breaking, and
default provider ordering.

- `default_timeout_ms`
  The HTTP timeout for provider requests. Default: `20000`.
- `cache_enabled`
  Enables the in-memory search cache. Default: `true`.
- `cache_ttl_seconds`
  Sets the cache TTL in seconds. Default: `120`.
- `circuit_failure_threshold`
  Opens the provider circuit after this many consecutive failures. Default:
  `3`.
- `circuit_open_seconds`
  Keeps a provider circuit open for this many seconds. Default: `30`.
- `search_provider_order`
  Sets the base provider order before ranking and health penalties are applied.
  Default: `["tavily", "ddg"]`.

#### `[tavily]`

The `[tavily]` section configures the built-in Tavily-compatible provider.

- `enabled`
  Enables or disables Tavily. Default: `true`.
- `base_url`
  Sets the Tavily API host. Default: `https://tavily.ivanli.cc`.
- `api_key_env`
  Sets the environment variable name that stores the token. Default:
  `TAVILY_HIKARI_TOKEN`.
- `api_key`
  Optionally embeds the token directly in the config file. If both `api_key`
  and `api_key_env` are present, `api_key` wins.
- `search_path`
  Sets the search endpoint path. Default: `/api/tavily/search`.
- `extract_path`
  Sets the extract endpoint path. Default: `/api/tavily/extract`.
- `crawl_path`
  Sets the crawl endpoint path. Default: `/api/tavily/crawl`.

#### `[ddg]`

The `[ddg]` section configures the built-in DuckDuckGo fallback provider.

- `enabled`
  Enables or disables DDG. Default: `true`.
- `base_url`
  Sets the DDG HTML endpoint. Default: `https://html.duckduckgo.com/html/`.
- `lite_url`
  Sets the DDG Lite endpoint. Default: `https://lite.duckduckgo.com/lite/`.
- `region`
  Sets the DDG region code. Default: `wt-wt`.
- `safe_search`
  Sets safe search mode. Supported values in current code are `off`, `strict`,
  and any other value for moderate mode. Default: `moderate`.
- `user_agent`
  Sets the User-Agent header used for DDG requests. The default is a
  browser-like User-Agent because DDG is more stable with it.

### Codex MCP example

If you want Codex to launch the local MCP server, add an entry like this to
`~/.codex/config.toml`:

```toml
[mcp_servers.tavily_hikari]
command = "/absolute/path/to/search-mcp-gateway"
args = ["mcp"]

[mcp_servers.tavily_hikari.env]
TAVILY_HIKARI_TOKEN = "your-token"
```

If you store your project config somewhere else, add
`SEARCH_MCP_GATEWAY_CONFIG`:

```toml
[mcp_servers.tavily_hikari]
command = "/absolute/path/to/search-mcp-gateway"
args = ["mcp"]

[mcp_servers.tavily_hikari.env]
TAVILY_HIKARI_TOKEN = "your-token"
SEARCH_MCP_GATEWAY_CONFIG = "/absolute/path/to/search-mcp-gateway.toml"
```

Replace `/absolute/path/to/search-mcp-gateway` with the absolute path to the
installed executable on your machine. For example:

- Windows: `D:\\Software\\Dev\\CLI\\search-mcp-gateway.exe`
- macOS or Linux: `/usr/local/bin/search-mcp-gateway`

### Current limitation

The current version supports only built-in providers configured through the
`[tavily]` and `[ddg]` sections. It does not yet support user-defined custom
providers through arbitrary `toml` blocks.
