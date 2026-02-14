use anyhow::Result;
use clap::{Parser, Subcommand};
use console::style;

use crate::analyzer::ProjectAnalyzer;
use crate::config::ProjectConfig;
use crate::engine::GenerationEngine;

/// SpringGen — Production-grade Spring Boot project generator
#[derive(Parser)]
#[command(
    name = "springgen",
    about = "Generate production-ready Spring Boot backends with full infrastructure",
    long_about = "SpringGen generates opinionated Spring Boot applications with Redis, Kafka, \
                  security, Docker, and more — fully configured. You write only business logic.",
    version,
    propagate_version = true
)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new Spring Boot project with selected features
    New(NewArgs),

    /// Interactively scaffold a project with guided prompts
    Init,

    /// Add features to an existing SpringGen project
    Add(AddArgs),

    /// Analyze an existing Spring Boot project and generate compatible config
    Analyze(AnalyzeArgs),

    /// Import configuration from an existing project and generate support files
    Import(ImportArgs),

    /// List all available features and their descriptions
    Features,

    /// Validate a springgen.toml configuration file
    Validate(ValidateArgs),

    /// Show the diff of what would be generated without writing files
    Preview(NewArgs),
}

/// Arguments for the `new` command
#[derive(clap::Args, Clone)]
pub struct NewArgs {
    /// Project name (also becomes the root directory)
    #[arg(value_name = "PROJECT_NAME")]
    pub name: String,

    /// Maven group ID (e.g. com.example)
    #[arg(long, default_value = "com.example")]
    pub group: String,

    /// Spring Boot version
    #[arg(long, default_value = "3.2.5")]
    pub boot_version: String,

    /// Java version
    #[arg(long, default_value = "21")]
    pub java_version: u8,

    /// Features to include (comma-separated or repeated flags)
    /// Available: redis, redis-ssl, redis-sentinel, kafka, security, jwt,
    ///            postgres, mysql, mongodb, elasticsearch, openapi, actuator,
    ///            docker, kubernetes, tracing, s3, email, websocket
    #[arg(short, long, value_delimiter = ',', value_name = "FEATURE")]
    pub features: Vec<String>,

    /// Redis connection mode when redis feature is enabled
    #[arg(long, value_enum, default_value = "standalone")]
    pub redis_mode: RedisMode,

    /// Output directory (defaults to ./<project-name>)
    #[arg(long, value_name = "DIR")]
    pub output: Option<std::path::PathBuf>,

    /// Overwrite existing directory without prompting
    #[arg(long)]
    pub force: bool,

    /// Emit a springgen.toml config alongside the generated project
    #[arg(long, default_value = "true")]
    pub emit_config: bool,
}

#[derive(clap::Args)]
pub struct AddArgs {
    /// Features to add
    #[arg(value_delimiter = ',', value_name = "FEATURE")]
    pub features: Vec<String>,

    /// Path to existing SpringGen project (defaults to current directory)
    #[arg(long, default_value = ".")]
    pub path: std::path::PathBuf,
}

#[derive(clap::Args)]
pub struct AnalyzeArgs {
    /// Path to the Spring Boot project to analyze
    #[arg(value_name = "PATH", default_value = ".")]
    pub path: std::path::PathBuf,

    /// Output format for the analysis report
    #[arg(long, value_enum, default_value = "table")]
    pub format: AnalyzeFormat,

    /// Save a springgen.toml based on the analysis
    #[arg(long)]
    pub save: bool,
}

#[derive(clap::Args)]
pub struct ImportArgs {
    /// Path to the existing Spring Boot project
    #[arg(value_name = "PATH", default_value = ".")]
    pub path: std::path::PathBuf,

    /// Features to supplement (auto-detected if not specified)
    #[arg(short, long, value_delimiter = ',')]
    pub features: Vec<String>,

    /// Output directory for generated supplemental files
    #[arg(long, default_value = "./springgen-out")]
    pub output: std::path::PathBuf,
}

#[derive(clap::Args)]
pub struct ValidateArgs {
    /// Path to springgen.toml
    #[arg(value_name = "CONFIG", default_value = "springgen.toml")]
    pub config: std::path::PathBuf,
}

#[derive(clap::ValueEnum, Clone, Debug, PartialEq)]
pub enum RedisMode {
    Standalone,
    Ssl,
    Sentinel,
    Cluster,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum AnalyzeFormat {
    Table,
    Json,
    Toml,
}

impl Cli {
    pub fn run() -> Result<()> {
        let cli = Self::parse();
        print_banner();

        match cli.command {
            Commands::New(args) => {
                let engine = GenerationEngine::new();
                engine.generate_project(args, false)
            }
            Commands::Preview(args) => {
                let engine = GenerationEngine::new();
                engine.generate_project(args, true)
            }
            Commands::Init => {
                let args = interactive_init()?;
                let engine = GenerationEngine::new();
                engine.generate_project(args, false)
            }
            Commands::Add(args) => {
                let engine = GenerationEngine::new();
                engine.add_features(args)
            }
            Commands::Analyze(args) => {
                let analyzer = ProjectAnalyzer::new();
                analyzer.analyze(&args.path, args.format, args.save)
            }
            Commands::Import(args) => {
                let analyzer = ProjectAnalyzer::new();
                analyzer.import_and_supplement(args)
            }
            Commands::Features => {
                print_features();
                Ok(())
            }
            Commands::Validate(args) => {
                let config = ProjectConfig::load(&args.config)?;
                config.validate()?;
                println!("{} springgen.toml is valid", style("✓").green().bold());
                Ok(())
            }
        }
    }
}

fn print_banner() {
    println!(
        "{}",
        style(
            r#"
  ____             _             ____
 / ___| _ __  _ __(_)_ __   __ _/ ___| ___ _ __
 \___ \| '_ \| '__| | '_ \ / _` | |  _ / _ \ '_ \
  ___) | |_) | |  | | | | | (_| | |_| |  __/ | | |
 |____/| .__/|_|  |_|_| |_|\__, |\____|\___|_| |_|
       |_|                  |___/
"#
        )
        .cyan()
        .bold()
    );
    println!(
        "  {} Production-grade Spring Boot generator\n",
        style("→").bold()
    );
}

fn print_features() {
    let features = crate::features::registry::all_features();
    println!("{}\n", style("Available Features").cyan().bold());

    let categories = [
        (
            "Data Layer",
            vec![
                "redis",
                "redis-ssl",
                "redis-sentinel",
                "postgres",
                "mysql",
                "mongodb",
                "elasticsearch",
            ],
        ),
        ("Messaging", vec!["kafka"]),
        ("Security", vec!["security", "jwt", "oauth2"]),
        ("API & Docs", vec!["openapi", "graphql", "websocket"]),
        ("Observability", vec!["actuator", "tracing", "metrics"]),
        ("Infrastructure", vec!["docker", "kubernetes"]),
        ("Integrations", vec!["s3", "email"]),
    ];

    for (cat, keys) in &categories {
        println!("  {}", style(*cat).yellow().bold());
        for key in keys {
            if let Some(feat) = features.iter().find(|f| f.key == *key) {
                println!(
                    "    {} {:<20} {}",
                    style("•").dim(),
                    style(feat.key).green(),
                    style(&feat.description).dim()
                );
            }
        }
        println!();
    }
}

fn interactive_init() -> Result<NewArgs> {
    use dialoguer::{theme::ColorfulTheme, Confirm, Input, MultiSelect, Select};

    let theme = ColorfulTheme::default();

    println!("{}", style("Interactive Project Setup").cyan().bold());
    println!("{}\n", style("─".repeat(40)).dim());

    let name: String = Input::with_theme(&theme)
        .with_prompt("Project name")
        .interact_text()?;

    let group: String = Input::with_theme(&theme)
        .with_prompt("Group ID")
        .default(format!("com.{}", name.to_lowercase().replace('-', "")))
        .interact_text()?;

    let boot_versions = vec!["3.2.5", "3.3.0", "3.1.12"];
    let boot_idx = Select::with_theme(&theme)
        .with_prompt("Spring Boot version")
        .items(&boot_versions)
        .default(0)
        .interact()?;

    let java_versions = vec!["21", "17", "11"];
    let java_idx = Select::with_theme(&theme)
        .with_prompt("Java version")
        .items(&java_versions)
        .default(0)
        .interact()?;

    let feature_options = vec![
        ("redis", "Redis cache & pub/sub (standalone)"),
        ("redis-ssl", "Redis with TLS/SSL encryption"),
        ("redis-sentinel", "Redis with Sentinel high-availability"),
        ("kafka", "Apache Kafka messaging"),
        ("postgres", "PostgreSQL with JPA/Hibernate"),
        ("mysql", "MySQL with JPA/Hibernate"),
        ("mongodb", "MongoDB with Spring Data"),
        ("security", "Spring Security (basic auth + RBAC)"),
        ("jwt", "JWT authentication (requires security)"),
        ("oauth2", "OAuth2 / OIDC resource server"),
        ("openapi", "OpenAPI 3 / Swagger UI"),
        ("actuator", "Spring Actuator health & metrics"),
        ("tracing", "Distributed tracing (Micrometer + Zipkin)"),
        ("docker", "Dockerfile + docker-compose.yml"),
        ("kubernetes", "Kubernetes manifests"),
        ("s3", "AWS S3 / MinIO integration"),
        ("email", "Email with Spring Mail"),
        ("websocket", "WebSocket support"),
        ("elasticsearch", "Elasticsearch integration"),
    ];

    let labels: Vec<&str> = feature_options.iter().map(|(_, label)| *label).collect();
    let selections = MultiSelect::with_theme(&theme)
        .with_prompt("Select features (space to toggle, enter to confirm)")
        .items(&labels)
        .interact()?;

    let features: Vec<String> = selections
        .iter()
        .map(|&i| feature_options[i].0.to_string())
        .collect();

    let redis_mode = if features.iter().any(|f| f.starts_with("redis")) {
        RedisMode::Standalone
    } else {
        RedisMode::Standalone
    };

    let _emit = Confirm::with_theme(&theme)
        .with_prompt("Emit springgen.toml config file?")
        .default(true)
        .interact()?;

    Ok(NewArgs {
        name,
        group,
        boot_version: boot_versions[boot_idx].to_string(),
        java_version: java_versions[java_idx].parse()?,
        features,
        redis_mode,
        output: None,
        force: false,
        emit_config: true,
    })
}
