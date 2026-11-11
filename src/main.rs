use anyhow::{Context, Result, bail};
use assetctl::{
    AppContext,
    connectors::{InventorySource, JsonFileSource},
    core::models::Finding,
    inventory::{AssetFilters, facets, filter_assets},
    modules::default_modules,
    output::{render_json_report, render_text_report},
};
use clap::{Args, Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

mod ui;

#[derive(Parser)]
#[command(
    name = "assetctl",
    version,
    about = "Local-first authorized asset inventory"
)]
struct Cli {
    #[arg(long, global = true, value_name = "PATH")]
    data_dir: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    Menu,
    Modules,
    Scope {
        #[command(subcommand)]
        command: ScopeCommand,
    },
    Groups,
    Asset {
        #[command(subcommand)]
        command: AssetCommand,
    },
    Search(Box<SearchArgs>),
    Facets {
        #[arg(long)]
        group: Option<String>,
    },
    Inventory,
    Check(FormatArgs),
    Report(FormatArgs),
    Audit,
    Import {
        path: PathBuf,
        #[arg(long, default_value = "default")]
        group: String,
    },
    Map {
        #[command(subcommand)]
        command: MapCommand,
    },
    Ssh {
        #[command(subcommand)]
        command: SshCommand,
    },
    Camera {
        #[command(subcommand)]
        command: CameraCommand,
    },
}

#[derive(Subcommand)]
enum CameraCommand {
    List {
        #[arg(long)]
        group: Option<String>,
    },
}

#[derive(Subcommand)]
enum MapCommand {
    Save {
        name: String,
        #[arg(long)]
        domain: String,
        #[arg(long, default_value = "default")]
        group: String,
    },
    List,
    Open {
        name: String,
    },
}

#[derive(Subcommand)]
enum SshCommand {
    AuditConfig { path: PathBuf },
}

#[derive(Subcommand)]
enum ScopeCommand {
    Add {
        target: String,
        #[arg(long, default_value = "default")]
        group: String,
        #[arg(long, help = "Assert that you are authorized to manage this scope")]
        authorized: bool,
    },
    List,
    Remove {
        target: String,
        #[arg(long, default_value = "default")]
        group: String,
    },
}

#[derive(Subcommand)]
enum AssetCommand {
    Add {
        host: String,
        label: String,
        #[arg(long, default_value = "default")]
        group: String,
        #[arg(long, default_value = "host", value_parser = ["host", "certificate", "web_property", "camera"])]
        asset_type: String,
    },
    Remove {
        host: String,
        #[arg(long, default_value = "default")]
        group: String,
    },
    Show {
        host: String,
        #[arg(long, default_value = "default")]
        group: String,
    },
}

#[derive(Args, Default)]
struct SearchArgs {
    #[arg(long)]
    text: Option<String>,
    #[arg(long)]
    group: Option<String>,
    #[arg(long, value_parser = ["host", "certificate", "web_property", "camera"])]
    asset_type: Option<String>,
    #[arg(long)]
    application: Option<String>,
    #[arg(long)]
    protocol: Option<String>,
    #[arg(long)]
    service_label: Option<String>,
    #[arg(long)]
    web_label: Option<String>,
    #[arg(long)]
    port: Option<u16>,
    #[arg(long)]
    network: Option<String>,
    #[arg(long)]
    location: Option<String>,
    #[arg(long)]
    software_vendor: Option<String>,
    #[arg(long)]
    software_product: Option<String>,
    #[arg(long)]
    certificate_label: Option<String>,
    #[arg(long)]
    issuer: Option<String>,
    #[arg(long, default_value = "text", value_parser = ["text", "json"])]
    format: String,
}

#[derive(Args)]
struct FormatArgs {
    #[arg(long, default_value = "text", value_parser = ["text", "json"])]
    format: String,
}

fn main() -> Result<()> {
    #[cfg(windows)]
    colored::control::set_virtual_terminal(true).ok();

    let cli = Cli::parse();
    let command = cli.command.unwrap_or(Command::Menu);
    let data_dir = cli.data_dir.unwrap_or_else(default_data_dir);
    let mut app = AppContext::open(data_dir)?;

    if !matches!(&command, Command::Menu) && !is_machine_output(&command) {
        ui::print_banner(app.state(), false);
    }

    match command {
        Command::Menu => menu(&mut app),
        Command::Modules => {
            println!("Built-in modules:");
            println!("  scope       Manage explicitly authorized domains and IPs");
            println!("  inventory   Track assets entered by the operator");
            println!("  camera      Browse camera records from imported authorized inventory");
            println!("  ssh         Audit a locally supplied sshd_config file");
            println!("  map         Save, list, and open local map snapshots");
            for module in default_modules() {
                println!("  {:<12} {}", module.name(), module.description());
            }
            println!("  report      Export local inventory and consistency findings");
            println!(
                "No network scanning, device login, exploitation, or stealth actions are implemented."
            );
            Ok(())
        }
        Command::Scope { command } => match command {
            ScopeCommand::Add {
                target,
                group,
                authorized,
            } => {
                if !authorized
                    && !ui::prompt_confirm(
                        &format!("Confirm authorization to manage {target}?"),
                        false,
                    )?
                {
                    bail!("scope was not added because authorization was not confirmed");
                }
                app.add_scope(&target, &group)?;
                println!("Added authorized scope to {group}: {target}");
                Ok(())
            }
            ScopeCommand::List => {
                if app.state().scopes.is_empty() {
                    println!("No authorized scopes configured.");
                } else {
                    for scope in &app.state().scopes {
                        println!(
                            "{}: {} (added {})",
                            scope.group, scope.target, scope.added_at
                        );
                    }
                }
                Ok(())
            }
            ScopeCommand::Remove { target, group } => {
                app.remove_scope(&target, &group)?;
                println!("Removed scope from {group}: {target}");
                Ok(())
            }
        },
        Command::Groups => {
            let groups = app
                .state()
                .scopes
                .iter()
                .map(|scope| scope.group.as_str())
                .chain(app.state().assets.iter().map(|asset| asset.group.as_str()))
                .collect::<std::collections::BTreeSet<_>>();
            if groups.is_empty() {
                println!("No groups configured.");
            } else {
                for group in groups.iter() {
                    println!("{group}");
                }
            }
            Ok(())
        }
        Command::Asset { command } => match command {
            AssetCommand::Add {
                host,
                label,
                group,
                asset_type,
            } => {
                app.add_asset_with_type(&host, &label, &group, &asset_type)?;
                println!("Added {host} to inventory.");
                Ok(())
            }
            AssetCommand::Remove { host, group } => {
                app.remove_asset(&host, &group)?;
                println!("Removed {host} from inventory.");
                Ok(())
            }
            AssetCommand::Show { host, group } => {
                let asset = app
                    .state()
                    .assets
                    .iter()
                    .find(|asset| {
                        asset.host.eq_ignore_ascii_case(&host)
                            && asset.group.eq_ignore_ascii_case(&group)
                    })
                    .with_context(|| format!("{host} was not found in group {group}"))?;
                println!("{}", serde_json::to_string_pretty(asset)?);
                Ok(())
            }
        },
        Command::Search(args) => {
            let args = *args;
            let filters = search_filters(&args);
            let matches = filter_assets(&app.state().assets, &filters);
            if args.format == "json" {
                println!("{}", serde_json::to_string_pretty(&matches)?);
            } else if matches.is_empty() {
                println!("No locally inventoried assets match those filters.");
            } else {
                for asset in matches {
                    let ports = asset
                        .ports
                        .iter()
                        .map(u16::to_string)
                        .collect::<Vec<_>>()
                        .join(", ");
                    println!(
                        "[{}] {} — {}{}",
                        asset.group,
                        asset.host,
                        asset.label,
                        if ports.is_empty() {
                            String::new()
                        } else {
                            format!(" | ports: {ports}")
                        }
                    );
                }
            }
            Ok(())
        }
        Command::Facets { group } => {
            let filters = AssetFilters {
                group: group.as_deref(),
                ..AssetFilters::default()
            };
            let matches = filter_assets(&app.state().assets, &filters);
            for facet_group in facets(&matches) {
                println!("\n{}", facet_group.name);
                if facet_group.values.is_empty() {
                    println!("  (no data)");
                } else {
                    for facet in facet_group.values {
                        println!("  {:<32} {}", facet.value, facet.count);
                    }
                }
            }
            println!("\nAssets: {}", matches.len());
            Ok(())
        }
        Command::Inventory => {
            let findings = app.findings()?;
            print!("{}", render_text_report(app.state(), &findings));
            Ok(())
        }
        Command::Check(args) => {
            let findings = app.findings()?;
            print_findings(args, app.state(), &findings)
        }
        Command::Report(args) => {
            let findings = app.findings()?;
            print_findings(args, app.state(), &findings)
        }
        Command::Audit => {
            let log = app.read_audit_log()?;
            if log.is_empty() {
                println!("Audit log is empty.");
            } else {
                print!("{log}");
            }
            Ok(())
        }
        Command::Import { path, group } => {
            let source = JsonFileSource::new(path);
            let assets = source.load_assets()?;
            let count = app.import_assets(&assets, &group)?;
            println!("Imported {count} assets from {}.", source.name());
            Ok(())
        }
        Command::Map { command } => match command {
            MapCommand::Save {
                name,
                domain,
                group,
            } => {
                let map = app.save_map(&name, &domain, &group)?;
                println!(
                    "Saved map '{}' for {} in group '{}' ({} assets).",
                    map.id,
                    map.target,
                    map.group,
                    map.state.assets.len()
                );
                Ok(())
            }
            MapCommand::List => {
                let maps = app.saved_maps()?;
                if maps.is_empty() {
                    println!("No saved maps.");
                } else {
                    for map in maps {
                        println!(
                            "{} — {} [{}], assets: {}, saved: {}",
                            map.id,
                            map.target,
                            map.group,
                            map.state.assets.len(),
                            map.created_at
                        );
                    }
                }
                Ok(())
            }
            MapCommand::Open { name } => {
                let map = app.open_saved_map(&name)?;
                print_saved_map(&map);
                Ok(())
            }
        },
        Command::Ssh { command } => match command {
            SshCommand::AuditConfig { path } => {
                let findings = assetctl::modules::ssh_config::audit_sshd_config(&path)?;
                print_ssh_findings(&findings);
                Ok(())
            }
        },
        Command::Camera { command } => match command {
            CameraCommand::List { group } => {
                let filters = AssetFilters {
                    group: group.as_deref(),
                    asset_type: Some("camera"),
                    ..AssetFilters::default()
                };
                let cameras = filter_assets(&app.state().assets, &filters);
                print_camera_inventory(&cameras);
                Ok(())
            }
        },
    }
}

fn default_data_dir() -> PathBuf {
    #[cfg(windows)]
    if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(local_app_data).join("assetctl");
    }

    if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
        return PathBuf::from(home).join(".assetctl");
    }

    PathBuf::from(".assetctl")
}

fn search_filters(args: &SearchArgs) -> AssetFilters<'_> {
    AssetFilters {
        text: args.text.as_deref(),
        group: args.group.as_deref(),
        asset_type: args.asset_type.as_deref(),
        application: args.application.as_deref(),
        protocol: args.protocol.as_deref(),
        service_label: args.service_label.as_deref(),
        web_label: args.web_label.as_deref(),
        port: args.port,
        network: args.network.as_deref(),
        location: args.location.as_deref(),
        software_vendor: args.software_vendor.as_deref(),
        software_product: args.software_product.as_deref(),
        certificate_label: args.certificate_label.as_deref(),
        issuer: args.issuer.as_deref(),
    }
}

fn print_findings(
    args: FormatArgs,
    state: &assetctl::AppState,
    findings: &[Finding],
) -> Result<()> {
    let output = if args.format == "json" {
        render_json_report(state, findings)?
    } else {
        render_text_report(state, findings)
    };
    print!("{output}");
    Ok(())
}

fn is_machine_output(command: &Command) -> bool {
    matches!(
        command,
        Command::Report(FormatArgs { format }) | Command::Check(FormatArgs { format })
            if format == "json"
    ) || matches!(
        command,
        Command::Search(args) if args.format == "json"
    )
}

fn menu(app: &mut AppContext) -> Result<()> {
    const MAP: &str = "Digital asset map";
    const CAMERAS: &str = "Camera inventory";
    const SSH: &str = "SSH configuration audit";
    const ARCHIVES: &str = "Saved maps and archives";
    const SCOPES: &str = "Scopes and groups";
    const ASSETS: &str = "Inventory management";
    const REPORTS: &str = "Reports and local checks";
    const AUDIT: &str = "Audit history";
    const MODULES: &str = "Available modules";
    const EXIT: &str = "Exit";

    let actions = vec![
        MAP, CAMERAS, SSH, ARCHIVES, SCOPES, ASSETS, REPORTS, AUDIT, MODULES, EXIT,
    ];

    loop {
        ui::print_banner(app.state(), true);
        let selected = match ui::prompt_select(
            "What would you like to do?",
            actions.iter().map(|action| (*action).to_owned()).collect(),
        ) {
            Ok(value) => value,
            Err(
                inquire::InquireError::OperationCanceled
                | inquire::InquireError::OperationInterrupted,
            ) => {
                println!("\n{}", "Session closed.".dimmed());
                return Ok(());
            }
            Err(error) => return Err(error.into()),
        };
        if selected == EXIT {
            return Ok(());
        }

        let result = match selected.as_str() {
            MAP => menu_digital_map(app),
            CAMERAS => menu_camera_inventory(app),
            SSH => menu_ssh_audit(),
            ARCHIVES => menu_archives(app),
            SCOPES => menu_scope_group(app),
            ASSETS => menu_inventory_management(app),
            REPORTS => menu_reports(app),
            AUDIT => menu_audit(app),
            MODULES => menu_modules(),
            _ => Ok(()),
        };
        match result {
            Ok(()) => {}
            Err(error) => {
                eprintln!("\n{} {error:#}", "Error:".red().bold());
            }
        }
        match ui::prompt_select("Return to the main menu", vec!["Continue".to_owned()]) {
            Ok(_) => {}
            Err(
                inquire::InquireError::OperationCanceled
                | inquire::InquireError::OperationInterrupted,
            ) => return Ok(()),
            Err(error) => return Err(error.into()),
        }
    }
}

fn choose_action(message: &str, actions: &[&str]) -> Result<String> {
    let options = actions.iter().map(|action| (*action).to_owned()).collect();
    Ok(ui::prompt_select(message, options)?)
}

fn menu_digital_map(app: &mut AppContext) -> Result<()> {
    let choices = [
        "Prepare map for authorized domain",
        "Import inventory metadata",
        "Search inventory",
        "Browse aggregate facets",
        "Save a domain map",
        "Back",
    ];
    match choose_action("Digital asset map", &choices)?.as_str() {
        "Prepare map for authorized domain" => prepare_domain_map(app),
        "Import inventory metadata" => menu_import(app),
        "Search inventory" => menu_search(app),
        "Browse aggregate facets" => menu_facets(app),
        "Save a domain map" => menu_save_map(app),
        _ => Ok(()),
    }
}

fn prepare_domain_map(app: &mut AppContext) -> Result<()> {
    let target = ui::prompt_text("Domain or IP to map", None)?;
    let domain = assetctl::scope::normalize_host(&target)?;
    let existing = app
        .state()
        .scopes
        .iter()
        .find(|scope| assetctl::scope::is_within_scope(&domain, &scope.target));
    let group = if let Some(scope) = existing {
        scope.group.clone()
    } else {
        if !ui::prompt_confirm(&format!("Confirm authorization to manage {target}?"), false)? {
            println!(
                "{}",
                "Mapping cancelled; authorization was not confirmed.".yellow()
            );
            return Ok(());
        }
        let group = ui::prompt_text("Inventory group", Some("default"))?;
        app.add_scope(&target, &group)?;
        group
    };

    let assets = app
        .state()
        .assets
        .iter()
        .filter(|asset| {
            asset.group == group && assetctl::scope::is_within_scope(&asset.host, &domain)
        })
        .collect::<Vec<_>>();
    println!(
        "\n{} {} — group: {}",
        "LOCAL DIGITAL ASSET MAP".cyan().bold(),
        domain.bold(),
        group
    );
    println!("Known inventory records: {}", assets.len());
    if assets.is_empty() {
        println!(
            "{}",
            "No imported records yet. Import an authorized inventory JSON file to populate this map."
                .yellow()
        );
        return Ok(());
    }
    for asset in &assets {
        print_asset_summary(asset);
    }
    for facet in facets(&assets) {
        if !facet.values.is_empty() {
            println!("\n{}", facet.name.cyan().bold());
            for value in facet.values {
                println!("  {:<32} {}", value.value, value.count.to_string().dimmed());
            }
        }
    }
    Ok(())
}

fn menu_camera_inventory(app: &AppContext) -> Result<()> {
    let groups = app
        .state()
        .assets
        .iter()
        .filter(|asset| asset.asset_type == "camera")
        .map(|asset| asset.group.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let selected_group = if groups.is_empty() {
        None
    } else {
        Some(ui::prompt_select(
            "Select camera inventory group",
            groups.iter().map(|group| (*group).to_owned()).collect(),
        )?)
    };
    let cameras = app
        .state()
        .assets
        .iter()
        .filter(|asset| {
            asset.asset_type == "camera"
                && selected_group
                    .as_deref()
                    .is_none_or(|group| asset.group == group)
        })
        .collect::<Vec<_>>();
    print_camera_inventory(&cameras);
    if cameras.is_empty() {
        println!(
            "{}",
            "No camera records found. Import inventory metadata for your authorized devices."
                .yellow()
        );
    }
    println!(
        "{}",
        "This view does not scan networks or open camera feeds.".dimmed()
    );
    Ok(())
}

fn menu_ssh_audit() -> Result<()> {
    let path = ui::prompt_text("Path to your local sshd_config file", None)?;
    let findings = assetctl::modules::ssh_config::audit_sshd_config(PathBuf::from(path).as_path())?;
    print_ssh_findings(&findings);
    Ok(())
}

fn menu_archives(app: &AppContext) -> Result<()> {
    let maps = app.saved_maps()?;
    if maps.is_empty() {
        println!("{}", "No saved maps yet.".yellow());
        if ui::prompt_confirm("Save an authorized domain map now?", false)? {
            menu_save_map(app)?;
        }
        return Ok(());
    }
    let mut options = maps
        .iter()
        .map(|map| {
            format!(
                "{} — [{}] {} assets",
                map.id,
                map.group,
                map.state.assets.len()
            )
        })
        .collect::<Vec<_>>();
    options.push("Back".into());
    let selected = ui::prompt_select("Open a saved map", options)?;
    if selected == "Back" {
        return Ok(());
    }
    let id = selected
        .split(" — ")
        .next()
        .context("invalid archive selection")?;
    let map = app.open_saved_map(id)?;
    print_saved_map(&map);
    Ok(())
}

fn menu_scope_group(app: &mut AppContext) -> Result<()> {
    match choose_action(
        "Scopes and groups",
        &[
            "List scopes and groups",
            "Add authorized scope",
            "Remove scope",
            "Back",
        ],
    )?
    .as_str()
    {
        "List scopes and groups" => menu_scopes(app),
        "Add authorized scope" => menu_add_scope(app),
        "Remove scope" => menu_remove_scope(app),
        _ => Ok(()),
    }
}

fn menu_inventory_management(app: &mut AppContext) -> Result<()> {
    match choose_action(
        "Inventory management",
        &[
            "Search inventory",
            "View asset details",
            "Add asset",
            "Remove asset",
            "Import local JSON",
            "Back",
        ],
    )?
    .as_str()
    {
        "Search inventory" => menu_search(app),
        "View asset details" => menu_show_asset(app),
        "Add asset" => menu_add_asset(app),
        "Remove asset" => menu_remove_asset(app),
        "Import local JSON" => menu_import(app),
        _ => Ok(()),
    }
}

fn menu_reports(app: &AppContext) -> Result<()> {
    match choose_action(
        "Reports and local checks",
        &["Run local consistency checks", "Export JSON report", "Back"],
    )?
    .as_str()
    {
        "Run local consistency checks" => menu_check(app),
        "Export JSON report" => menu_report(app),
        _ => Ok(()),
    }
}

fn menu_save_map(app: &AppContext) -> Result<()> {
    if app.state().scopes.is_empty() {
        println!(
            "{}",
            "Add an authorized scope before saving a map.".yellow()
        );
        return Ok(());
    }
    let scopes = app
        .state()
        .scopes
        .iter()
        .map(|scope| format!("[{}] {}", scope.group, scope.target))
        .collect::<Vec<_>>();
    let selected = ui::prompt_select("Select an authorized domain", scopes)?;
    let (group, domain) = selected
        .split_once("] ")
        .map(|(group, domain)| (group.trim_start_matches('['), domain))
        .context("could not parse selected scope")?;
    let name = ui::prompt_text("Archive name", None)?;
    let map = app.save_map(&name, domain, group)?;
    println!(
        "{} Saved map '{}' for {} with {} asset(s).",
        "✔".green().bold(),
        map.id,
        map.target,
        map.state.assets.len()
    );
    Ok(())
}

fn print_saved_map(map: &assetctl::storage::SavedMap) {
    println!(
        "\n{} {} — domain: {} — group: {} — saved: {}",
        "SAVED DIGITAL MAP".cyan().bold(),
        map.id.bold(),
        map.target,
        map.group,
        map.created_at
    );
    println!("Scopes: {}", map.state.scopes.len());
    for scope in &map.state.scopes {
        println!("  {}", scope.target);
    }
    println!("Assets: {}", map.state.assets.len());
    for asset in &map.state.assets {
        print_asset_summary(asset);
    }
}

fn print_camera_inventory(cameras: &[&assetctl::Asset]) {
    if cameras.is_empty() {
        println!("No camera records in local inventory.");
        return;
    }
    println!("\n{} ({})", "CAMERA INVENTORY".cyan().bold(), cameras.len());
    for camera in cameras {
        print_asset_summary(camera);
        if !camera.ip_addresses.is_empty() {
            println!("    addresses: {}", camera.ip_addresses.join(", "));
        }
        if !camera.software_products.is_empty() {
            println!("    products: {}", camera.software_products.join(", "));
        }
    }
}

fn print_ssh_findings(findings: &[Finding]) {
    if findings.is_empty() {
        println!(
            "{} No flagged settings found in the supplied local configuration.",
            "✔".green().bold()
        );
        return;
    }
    println!(
        "{} SSH configuration findings: {}",
        "!".yellow().bold(),
        findings.len()
    );
    for finding in findings {
        println!(
            "  [{}] {}\n    {}\n    Fix: {}",
            finding.severity.as_str().to_uppercase(),
            finding.title,
            finding.details,
            finding.recommendation
        );
    }
    println!(
        "{}",
        "Local config review only; no connection or exploit was attempted.".dimmed()
    );
}

fn menu_search(app: &AppContext) -> Result<()> {
    let criteria = ui::prompt_select(
        "Search by",
        vec![
            "Any text".into(),
            "Inventory group".into(),
            "Asset type".into(),
            "Application".into(),
            "Protocol".into(),
            "Port".into(),
            "Service label".into(),
            "Web label".into(),
            "Network / AS".into(),
            "Location".into(),
            "Software vendor".into(),
            "Software product".into(),
            "Certificate label".into(),
            "Certificate issuer".into(),
        ],
    )?;
    let query = if criteria == "Inventory group" {
        let groups = app
            .state()
            .scopes
            .iter()
            .map(|scope| scope.group.as_str())
            .chain(app.state().assets.iter().map(|asset| asset.group.as_str()))
            .collect::<std::collections::BTreeSet<_>>();
        if groups.is_empty() {
            println!("{}", "No groups have been configured yet.".yellow());
            return Ok(());
        }
        ui::prompt_select(
            "Select inventory group",
            groups.iter().map(|group| (*group).to_owned()).collect(),
        )?
    } else {
        ui::prompt_text("Search value", None)?
    };
    let mut filters = AssetFilters::default();
    let value = query.trim();
    match criteria.as_str() {
        "Any text" => filters.text = Some(value),
        "Inventory group" => filters.group = Some(value),
        "Asset type" => filters.asset_type = Some(value),
        "Application" => filters.application = Some(value),
        "Protocol" => filters.protocol = Some(value),
        "Port" => {
            filters.port = Some(
                value
                    .parse()
                    .context("port must be a number from 0 to 65535")?,
            )
        }
        "Service label" => filters.service_label = Some(value),
        "Web label" => filters.web_label = Some(value),
        "Network / AS" => filters.network = Some(value),
        "Location" => filters.location = Some(value),
        "Software vendor" => filters.software_vendor = Some(value),
        "Software product" => filters.software_product = Some(value),
        "Certificate label" => filters.certificate_label = Some(value),
        "Certificate issuer" => filters.issuer = Some(value),
        _ => bail!("unsupported search criterion"),
    }
    let matches = filter_assets(&app.state().assets, &filters);
    if matches.is_empty() {
        println!("{}", "No matching records in the local inventory.".yellow());
    } else {
        println!("\n{} matching assets\n", matches.len().to_string().bold());
        for asset in matches {
            print_asset_summary(asset);
        }
    }
    Ok(())
}

fn menu_facets(app: &AppContext) -> Result<()> {
    let matches = filter_assets(&app.state().assets, &AssetFilters::default());
    for group in facets(&matches) {
        println!("\n{}", group.name.cyan().bold());
        if group.values.is_empty() {
            println!("  {}", "(no imported values)".dimmed());
        } else {
            for facet in group.values {
                println!("  {:<32} {}", facet.value, facet.count.to_string().dimmed());
            }
        }
    }
    println!("\n{} {}", "Assets:".bold(), matches.len());
    Ok(())
}

fn menu_show_asset(app: &AppContext) -> Result<()> {
    if app.state().assets.is_empty() {
        println!("{}", "Inventory is empty.".yellow());
        return Ok(());
    }
    let options = app
        .state()
        .assets
        .iter()
        .map(|asset| format!("[{}] {} — {}", asset.group, asset.host, asset.label))
        .collect::<Vec<_>>();
    let selected = ui::prompt_select("Select an asset", options)?;
    let asset = app
        .state()
        .assets
        .iter()
        .find(|asset| format!("[{}] {} — {}", asset.group, asset.host, asset.label) == selected)
        .context("selected asset was not found")?;
    println!("{}", serde_json::to_string_pretty(asset)?);
    Ok(())
}

fn menu_add_asset(app: &mut AppContext) -> Result<()> {
    let host = ui::prompt_text("Asset domain or IP", None)?;
    let label = ui::prompt_text("Display label", None)?;
    let group = ui::prompt_text("Authorized group", Some("default"))?;
    let asset_type = ui::prompt_select(
        "Asset type",
        vec![
            "host".into(),
            "certificate".into(),
            "web_property".into(),
            "camera".into(),
        ],
    )?;
    app.add_asset_with_type(&host, &label, &group, &asset_type)?;
    println!(
        "{} Added {} ({}) to {}.",
        "✔".green().bold(),
        host.bold(),
        asset_type,
        group.cyan()
    );
    Ok(())
}

fn menu_remove_asset(app: &mut AppContext) -> Result<()> {
    if app.state().assets.is_empty() {
        println!("{}", "Inventory is empty.".yellow());
        return Ok(());
    }
    let options = app
        .state()
        .assets
        .iter()
        .map(|asset| format!("[{}] {}", asset.group, asset.host))
        .collect::<Vec<_>>();
    let selected = ui::prompt_select("Select an asset to remove", options)?;
    let (group, host) = selected
        .split_once("] ")
        .map(|(group, host)| (group.trim_start_matches('['), host))
        .context("could not parse selected asset")?;
    if ui::prompt_confirm(&format!("Remove {host} from inventory?"), false)? {
        app.remove_asset(host, group)?;
        println!("{} Removed {host}.", "✔".green().bold());
    } else {
        println!("{}", "No changes made.".dimmed());
    }
    Ok(())
}

fn menu_add_scope(app: &mut AppContext) -> Result<()> {
    let target = ui::prompt_text("Domain or IP to authorize", None)?;
    if !ui::prompt_confirm(
        &format!("Confirm that you own or are authorized to manage {target}?"),
        false,
    )? {
        println!("{}", "Scope was not added.".dimmed());
        return Ok(());
    }
    let group = ui::prompt_text("Group", Some("default"))?;
    app.add_scope(&target, &group)?;
    println!(
        "{} Added scope {} to {}.",
        "✔".green().bold(),
        target.bold(),
        group.cyan()
    );
    Ok(())
}

fn menu_remove_scope(app: &mut AppContext) -> Result<()> {
    if app.state().scopes.is_empty() {
        println!("{}", "No authorized scopes configured.".yellow());
        return Ok(());
    }
    let options = app
        .state()
        .scopes
        .iter()
        .map(|scope| format!("[{}] {}", scope.group, scope.target))
        .collect::<Vec<_>>();
    let selected = ui::prompt_select("Select a scope to remove", options)?;
    let (group, target) = selected
        .split_once("] ")
        .map(|(group, target)| (group.trim_start_matches('['), target))
        .context("could not parse selected scope")?;
    if ui::prompt_confirm(&format!("Remove {target} from group {group}?"), false)? {
        app.remove_scope(target, group)?;
        println!("{} Removed scope {target}.", "✔".green().bold());
    } else {
        println!("{}", "No changes made.".dimmed());
    }
    Ok(())
}

fn menu_scopes(app: &AppContext) -> Result<()> {
    if app.state().scopes.is_empty() {
        println!("{}", "No authorized scopes configured.".yellow());
    } else {
        println!("{}", "AUTHORIZED SCOPES".cyan().bold());
        for scope in &app.state().scopes {
            println!("  [{}] {}", scope.group, scope.target);
        }
    }
    let groups = app
        .state()
        .scopes
        .iter()
        .map(|scope| scope.group.as_str())
        .chain(app.state().assets.iter().map(|asset| asset.group.as_str()))
        .collect::<std::collections::BTreeSet<_>>();
    println!(
        "\n{} {}",
        "Groups:".bold(),
        groups.into_iter().collect::<Vec<_>>().join(", ")
    );
    Ok(())
}

fn menu_import(app: &mut AppContext) -> Result<()> {
    let path = ui::prompt_text("Path to local JSON inventory file", None)?;
    let group = ui::prompt_text("Authorized group for imported assets", Some("default"))?;
    let source = JsonFileSource::new(path);
    let assets = source.load_assets()?;
    let count = app.import_assets(&assets, &group)?;
    println!(
        "{} Imported {} assets from {}.",
        "✔".green().bold(),
        count,
        source.name().cyan()
    );
    Ok(())
}

fn menu_check(app: &AppContext) -> Result<()> {
    let findings = app.findings()?;
    if findings.is_empty() {
        println!(
            "{} No local inventory consistency issues found.",
            "✔".green().bold()
        );
    } else {
        println!("{} {} finding(s)", "!".yellow().bold(), findings.len());
        for finding in findings {
            println!(
                "  [{}] {} — {}",
                finding.severity.as_str().to_uppercase(),
                finding.title,
                finding.recommendation
            );
        }
    }
    Ok(())
}

fn menu_report(app: &AppContext) -> Result<()> {
    let findings = app.findings()?;
    println!("{}", render_json_report(app.state(), &findings)?);
    Ok(())
}

fn menu_audit(app: &AppContext) -> Result<()> {
    let log = app.read_audit_log()?;
    if log.is_empty() {
        println!("{}", "Audit log is empty.".yellow());
    } else {
        println!("{}", "LOCAL AUDIT LOG".cyan().bold());
        print!("{log}");
    }
    Ok(())
}

fn menu_modules() -> Result<()> {
    println!("{}", "AVAILABLE MODULES".cyan().bold());
    println!("  scope       Explicitly authorized scope management");
    println!("  inventory   Local inventory search and aggregate facets");
    println!("  digital-map Local domain map built from imported records; save/open snapshots");
    println!("  camera      Browse imported camera asset records; no network discovery");
    println!("  ssh         Offline sshd_config posture review; no connection or exploitation");
    for module in default_modules() {
        println!("  {:<12} {}", module.name(), module.description());
    }
    println!("  import      Import operator-provided local JSON inventory");
    println!("\nNo network scanning, device login, exploitation, or stealth actions.");
    Ok(())
}

fn print_asset_summary(asset: &assetctl::Asset) {
    println!(
        "  {} {} {}",
        format!("[{}]", asset.group).dimmed(),
        asset.host.cyan().bold(),
        asset.label
    );
    println!(
        "    type: {}  ports: {}  apps: {}",
        asset.asset_type,
        if asset.ports.is_empty() {
            "—".into()
        } else {
            asset
                .ports
                .iter()
                .map(u16::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        },
        if asset.applications.is_empty() {
            "—".into()
        } else {
            asset.applications.join(", ")
        }
    );
}
