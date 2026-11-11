# AEGIS

**AEGIS** is a modular cybersecurity operations workbench being built to support multiple security domains, not only asset inventory. Its current foundation includes authorized attack-surface records, local infrastructure configuration review, security data imports, transport diagnostics, audit history, and reporting. The architecture is intended to grow into application and cloud security, incident response, digital forensics, detection engineering, and other defensive workflows; those domains are not represented as implemented until their workflows are actually available.

## Requirements

- Rust stable toolchain

## Run

```powershell
cargo run -- --help
cargo run
```

Running without a subcommand opens a keyboard-navigable interactive menu with a large AEGIS banner, neon cyber palette, local status counts, and security-domain navigation. It includes attack-surface intelligence, infrastructure security review, scoped IoT inventory, evidence/map archives, security data management, reports, and audit history. Use the arrow keys and Enter; press Ctrl+C or choose Exit to leave. Noninteractive commands remain available for scripting.

The executable and Rust package are named `aegis`. To preserve existing installations and records, the default data directory remains `%LOCALAPPDATA%\assetctl` on Windows (or `$HOME/.assetctl` elsewhere). Pass `--data-dir <PATH>` before the command to select another local directory.

For a standalone Windows executable, build with `cargo build --release` and launch `target\release\aegis.exe`, or use the packaged `dist\aegis.exe`. No Rust installation is needed to run that executable; double-clicking opens its console-based interactive menu. For a terminal window that stays available, run it from PowerShell or Windows Terminal.

## Commands

```powershell
cargo run -- modules
cargo run -- scope add example.com --group production
cargo run -- scope list
cargo run -- groups
cargo run -- asset add api.example.com "Public API" --group production
cargo run -- asset add cam1.example.com "Lobby camera" --group production --asset-type camera
cargo run -- import .\inventory.json --group production
cargo run -- search --group production --asset-type host --application HTTP --port 443
cargo run -- facets --group production
cargo run -- asset show api.example.com --group production
cargo run -- map save prod-2026-11 --domain example.com --group production
cargo run -- map list
cargo run -- map open prod-2026-11
cargo run -- camera list --group production
cargo run -- camera add cam1.example.com "Lobby camera" --group production --protocol ONVIF --protocol RTSP
cargo run -- camera show cam1.example.com --group production
cargo run -- camera remove cam1.example.com --group production
cargo run -- ssh audit-config .\sshd_config
cargo run -- import-url https://inventory.example.com/v1/assets --group production
cargo run -- transport diagnostics
cargo run -- inventory
cargo run -- check --format text
cargo run -- report --format json
cargo run -- audit
```

Adding a scope requires interactive authorization confirmation. `--authorized` is available for scripted setup and is an explicit operator assertion, not independent ownership verification. Each scope and asset belongs to a named group (default: `default`). An asset must be within a configured domain scope in the same group on a DNS-label boundary or exactly match an IP scope. Scope removal is blocked while assets in that group still depend on it.

The local JSON import format is an array of manually collected inventory records. Optional metadata powers local search and facet counts:

```json
[
  {
    "host": "api.example.com",
    "label": "Public API",
    "asset_type": "host",
    "ip_addresses": ["192.0.2.10"],
    "ports": [443],
    "networks": ["Example AS"],
    "locations": ["Example Region"],
    "applications": ["HTTP"],
    "protocols": ["TCP"],
    "service_labels": ["WEB"],
    "web_labels": ["LOGIN_PAGE"],
    "software_vendors": ["Example Vendor"],
    "software_products": ["Example Server"],
    "certificate_labels": ["leaf"],
    "issuers": ["Example CA"],
    "endpoints": ["/"],
    "matched_fields": {
      "certificate.subject": "api.example.com",
      "http.title": "Example"
    }
  }
]
```

`asset_type` is `host`, `certificate`, `web_property`, or `camera` (default: `host`). Every other metadata field is optional. `search` filters only locally stored records by text, group, asset type, application, protocol, service/web label, port, network, location, software vendor/product, certificate label, or issuer. `facets` shows per-field aggregate counts, and `asset show` displays one record's imported details. Import validates the entire file against the selected authorized group before saving any records; it does not fetch or discover metadata itself.

The interactive menu organizes current workflows as **Attack Surface Intelligence**, **IoT and Camera Inventory**, **Infrastructure Security Review**, **Network Transport Diagnostics**, **Evidence and Map Archives**, **Engagement Scope and Groups**, **Security Data Management**, **Security Posture and Reporting**, and **Activity Audit Trail**. A domain map is assembled from previously imported inventory records under a confirmed authorized scope. Save it with `map save prod-snapshot --domain example.com --group production`, then later find/open snapshots with `map list` and `map open`; snapshots are stored in the `archives` subfolder under the local data directory and cannot overwrite an existing archive.

Camera inventory accepts records with `"asset_type": "camera"` from a local authorized inventory export, `camera add`, or the interactive Camera inventory menu. Protocol labels such as ONVIF and RTSP are descriptive inventory metadata only; `camera list`, `camera show`, and `camera remove` manage saved records. This is not a camera finder or live viewer: it does not discover nearby devices, connect to cameras, authenticate, inspect streams, or control them. SSH audit reads a local `sshd_config` file and flags a small set of explicitly configured risky server settings. It does not connect to an IP, attempt authentication, or exploit SSH services. This is a basic configuration review, not a complete evaluation of OpenSSH's `Include`, `Match`, and platform-specific effective settings.

## Network transport

The only network transport currently implemented is a reusable direct HTTPS client for importing JSON inventory from an endpoint whose hostname is already inside an authorized scope in the selected group:

```powershell
aegis.exe scope add example.com --group production --authorized
aegis.exe import-url https://inventory.example.com/v1/assets --group production --timeout-seconds 15
aegis.exe transport diagnostics
```

The endpoint must use HTTPS and must not embed credentials in the URL. TLS certificate and hostname verification use `reqwest`/Rustls defaults; redirects are disabled, responses are limited to 8 MiB, and errors do not print the request URL. A single client is retained by the application for connection pooling and reuse. Request futures can be cancelled by dropping the future; there are no background workers. The client honors the host OS's normal routing and proxy environment. The timeout defaults to 20 seconds and can be changed with `--timeout-seconds`.

There is **no Tor/SOCKS5 mode** and no fallback between transports. VPN diagnostics only look for likely interface names (`netsh` on Windows, `ip` on Linux, and `ifconfig` on macOS); this is a heuristic, not proof that a VPN is active or that a specific request used it. No VPN, routes, firewall settings, or other system networking services are modified. HTTPS requests are only for inventory import, not scanning or probing assets.

### Troubleshooting

- `outside the authorized ... scope`: add the API hostname's owning domain to the same group, after confirming authorization.
- `URL must use HTTPS`: use an HTTPS API endpoint with a publicly or privately trusted certificate installed through approved OS trust settings.
- TLS or timeout errors: check the API service, system clock, OS trust store, authorized VPN connection, and the `--timeout-seconds` value.
- `response was not JSON` or response-size errors: make sure the endpoint returns a JSON array in the documented inventory schema and stays below 8 MiB.
- `transport diagnostics` cannot see a VPN: interface naming is heuristic; inspect the OS VPN client. Even a detected adapter does not prove request routing.

`check` runs only local consistency checks against the stored inventory. It makes no network connections and does not verify whether an asset is reachable or vulnerable.

The search/facet interface is an offline inventory view inspired by the result categories visible in the provided screenshots. It is not a copy of Censys and does not query its service or reproduce an Internet-wide index. A geographic map requires coordinates in a future presentation layer; current location facets only summarize location strings supplied in local inventory files.

## Architecture

```text
src/
├── app/          Application context and use cases
├── audit/        Append-only JSONL activity log
├── connectors/   Extension contract for authorized inventory sources
├── core/         Shared domain models
├── inventory/    Local search and aggregate facets
├── modules/      Local, read-only check modules
├── output/       Text and JSON report rendering
├── scope/        Host normalization and authorization boundaries
├── storage/      Local JSON persistence
├── transport/    Shared HTTPS inventory client and OS VPN hints
└── ui/           Colored banner and interactive prompts
```

New inventory connectors should consume local exports or documented, authorized inventory APIs. New check modules should be read-only, scoped, and auditable.

The terminal theme uses a cyan/blue/indigo header, green success indicators, amber scope notices, and a consistent keyboard prompt style. Machine-readable JSON commands omit the decorative banner so their output remains parseable.

## Development

```powershell
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```
