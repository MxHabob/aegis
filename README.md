# assetctl

`assetctl` is a local-first Rust CLI for organizing assets you own or are explicitly authorized to manage. It records operator-entered scope and inventory, checks local inventory consistency, and creates reports. It does not scan networks, log in to devices, exploit vulnerabilities, access cameras, or hide activity.

## Requirements

- Rust stable toolchain

## Run

```powershell
cargo run -- --help
cargo run
```

Running without a subcommand opens a keyboard-navigable interactive menu with a colored ASSETCTL banner, scope/inventory counters, search, facets, asset details, scope management, JSON import, local checks, reports, and audit history. Use the arrow keys and Enter; press Ctrl+C or choose Exit to leave. Noninteractive commands remain available for scripting.

By default, state and audit history are stored under `%LOCALAPPDATA%\assetctl` on Windows (or `$HOME/.assetctl` elsewhere). This keeps double-click launches independent of the current working directory. Pass `--data-dir <PATH>` before the command to select another local directory.

For a standalone Windows executable, build with `cargo build --release` and launch `target\release\assetctl.exe`, or use the packaged `dist\assetctl.exe`. No Rust installation is needed to run that executable; double-clicking opens its console-based interactive menu. For a terminal window that stays available, run it from PowerShell or Windows Terminal.

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
cargo run -- ssh audit-config .\sshd_config
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

The interactive menu groups workflows into **Digital asset map**, **Camera inventory**, **SSH configuration audit**, **Saved maps and archives**, **Scopes and groups**, **Inventory management**, and **Reports and local checks**. A domain map is assembled from previously imported inventory records under a confirmed authorized scope. Save it with `map save prod-snapshot --domain example.com --group production`, then later find/open snapshots with `map list` and `map open`; snapshots are stored in the `archives` subfolder under the local data directory and cannot overwrite an existing archive.

Camera inventory accepts records with `"asset_type": "camera"` from a local authorized inventory export or the interactive Add Asset flow; it is an inventory browser, not a camera finder or live viewer. SSH audit reads a local `sshd_config` file and flags a small set of explicitly configured risky server settings. It does not connect to an IP, attempt authentication, or exploit SSH services. This is a basic configuration review, not a complete evaluation of OpenSSH's `Include`, `Match`, and platform-specific effective settings.

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
