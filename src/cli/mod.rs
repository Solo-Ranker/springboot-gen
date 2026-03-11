use anyhow::Result;
use clap::{Parser, Subcommand};
use console::style;

use crate::analyzer::ProjectAnalyzer;
use crate::config::ProjectConfig;
use crate::engine::GenerationEngine;

/// SpringbootGen — Boilterplate Spring Boot project generator
#[derive(Parser)]
#[command(
    name = "springboot-gen",
    about = "Generate boilerplate Spring Boot backends with full infrastructure",
    long_about = "SpringbootGen generates opinionated Spring Boot applications with Redis, Kafka, \
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

    /// Add features to an existing SpringbootGen project
    Add(AddArgs),

    /// Analyze an existing Spring Boot project and generate compatible config
    Analyze(AnalyzeArgs),

    /// Import configuration from an existing project and generate support files
    Import(ImportArgs),

    /// List all available features and their descriptions
    Features,

    /// Validate a springboot-gen.toml configuration file
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
    #[arg(long, default_value = "3.5.11")]
    pub boot_version: String,

    /// Java version
    #[arg(long, default_value = "21")]
    pub java_version: u8,

    /// Features to include (comma-separated or repeated flags)
    /// Available: redis, kafka, postgres, mysql, mongodb, elasticsearch,
    ///            security, jwt, oauth2, openapi, actuator, docker, kubernetes,
    ///            tracing, s3, email, websocket, rabbitmq, ibmmq
    #[arg(short, long, value_delimiter = ',', value_name = "FEATURE")]
    pub features: Vec<String>,

    /// Kafka stack options (comma-separated): sasl, ssl
    /// Example: --kafka-stack sasl
    #[arg(long, value_delimiter = ',', value_name = "OPTION")]
    pub kafka_stack: Vec<String>,

    /// Build tool (maven or gradle)
    #[arg(long, value_enum, default_value = "maven")]
    pub build_tool: BuildTool,

    /// Gradle DSL (kotlin or groovy), only used when build_tool is gradle
    #[arg(long, value_enum, default_value = "kotlin")]
    pub gradle_dsl: GradleDsl,

    /// Skip automatic code formatting after generation
    #[arg(long)]
    pub skip_format: bool,

    /// Output directory (defaults to ./<project-name>)
    #[arg(long, value_name = "DIR")]
    pub output: Option<std::path::PathBuf>,

    /// Overwrite existing directory without prompting
    #[arg(long)]
    pub force: bool,

    /// Emit a springboot-gen.toml config alongside the generated project
    #[arg(long, default_value = "true")]
    pub emit_config: bool,
}

#[derive(clap::Args)]
pub struct AddArgs {
    /// Features to add
    #[arg(value_delimiter = ',', value_name = "FEATURE")]
    pub features: Vec<String>,

    /// Path to existing SpringbootGen project (defaults to current directory)
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

    /// Save a springboot-gen.toml based on the analysis
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
    #[arg(long, default_value = "./springboot-gen-out")]
    pub output: std::path::PathBuf,
}

#[derive(clap::Args)]
pub struct ValidateArgs {
    /// Path to springboot-gen.toml
    #[arg(value_name = "CONFIG", default_value = "springboot-gen.toml")]
    pub config: std::path::PathBuf,
}

#[derive(clap::ValueEnum, Clone, Debug, PartialEq)]
pub enum BuildTool {
    Maven,
    Gradle,
}

#[derive(clap::ValueEnum, Clone, Debug, PartialEq)]
pub enum GradleDsl {
    Kotlin,
    Groovy,
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
                let engine = GenerationEngine::new()?;
                engine.generate_project(args, None, false)
            }
            Commands::Preview(args) => {
                let engine = GenerationEngine::new()?;
                engine.generate_project(args, None, true)
            }
            Commands::Init => {
                let (args, config) = interactive_init()?;
                let engine = GenerationEngine::new()?;
                engine.generate_project(args, Some(config), false)
            }
            Commands::Add(args) => {
                let engine = GenerationEngine::new()?;
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
                println!("{} springboot-gen.toml is valid", style("✓").green().bold());
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
  ____             _             _                 _    ____            
 / ___| _ __  _ __(_)_ __   __ _| |__   ___   ___ | |_ / ___| ___ _ __  
 \___ \| '_ \| '__| | '_ \ / _` | '_ \ / _ \ / _ \| __| |  _ / _ \ '_ \ 
  ___) | |_) | |  | | | | | (_| | |_) | (_) | (_) | |_| |_| |  __/ | | |
 |____/| .__/|_|  |_|_| |_|\__, |_.__/ \___/ \___/ \__|\____|\___|_| |_|
       |_|                  |___/
"#
        )
        .cyan()
        .bold()
    );
    println!(
        "  {} Boilerplate Spring Boot generator\n",
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
        ("Messaging", vec!["kafka", "rabbitmq", "ibmmq"]),
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
                    style(&feat.key).green(),
                    style(&feat.description).dim()
                );
            }
        }
        println!();
    }
}

fn interactive_init() -> Result<(NewArgs, ProjectConfig)> {
    use dialoguer::{theme::ColorfulTheme, Confirm};
    use indexmap::IndexMap;

    let theme = ColorfulTheme::default();

    println!("{}", style("Interactive Project Setup").cyan().bold());
    println!("{}\n", style("─".repeat(40)).dim());

    // 1. Project Metadata
    let mut args = prompt_project_meta(&theme)?;
    let mut config = ProjectConfig::from_new_args(&args);

    // 2. Dynamic Features
    let features = crate::features::registry::all_features();
    let mut categories: IndexMap<String, Vec<&crate::features::FeatureSpec>> = IndexMap::new();
    
    for feature in &features {
        categories
            .entry(feature.category.clone())
            .or_default()
            .push(feature);
    }

    use dialoguer::MultiSelect;

    for (cat_name, cat_features) in categories {
        if cat_features.is_empty() {
            continue;
        }

        let labels: Vec<String> = cat_features
            .iter()
            .map(|f| format!("{} ({})", f.name, f.description))
            .collect();

        // Use multiselect for all options for maximum composability
        let selections = MultiSelect::with_theme(&theme)
            .with_prompt(&cat_name)
            .items(&labels)
            .interact()?;

        for idx in selections {
            args.features.push(cat_features[idx].key.clone());
        }
    }

    // 3. Post-feature checks for specific properties
    if args.features.iter().any(|f| f == "postgres" || f == "mysql") {
        config.database.flyway_enabled = Confirm::with_theme(&theme)
            .with_prompt("Enable Flyway migrations?")
            .default(true)
            .interact()?;
    }

    if args.features.contains(&"kafka".to_string()) {
        let stack_options = [("sasl", "SASL Authentication"),
            ("ssl", "SSL/TLS Encryption")];
        let labels: Vec<&str> = stack_options.iter().map(|(_, l)| *l).collect();

        let selections = MultiSelect::with_theme(&theme)
            .with_prompt("Kafka Stack Options (optional)")
            .items(&labels)
            .interact()?;

        for idx in selections {
            args.kafka_stack.push(stack_options[idx].0.to_string());
        }
    }

    let _emit = Confirm::with_theme(&theme)
        .with_prompt("Emit springboot-gen.toml config file?")
        .default(true)
        .interact()?;

    // Sync features back to config
    config.features = args.features.clone();

    Ok((args, config))
}

fn prompt_project_meta(theme: &dialoguer::theme::ColorfulTheme) -> Result<NewArgs> {
    use dialoguer::{Input, Select};

    let name: String = Input::with_theme(theme)
        .with_prompt("Project name")
        .interact_text()?;

    let group: String = Input::with_theme(theme)
        .with_prompt("Group ID (e.g., com.example)")
        .default("com.example".to_string())
        .interact_text()?;

    let boot_versions = vec!["3.5.11", "3.4.3", "3.3.9"];
    let boot_idx = Select::with_theme(theme)
        .with_prompt("Spring Boot version")
        .items(&boot_versions)
        .default(0)
        .interact()?;

    let java_versions = vec!["21", "17 (Not Recommended: Sample code are written with Java 21 syntax - will support that in the future)", "11 (Not Recommended: Sample code are written with Java 21 syntax - will support that in the future)"];
    let java_idx = Select::with_theme(theme)
        .with_prompt("Java version")
        .items(&java_versions)
        .default(0)
        .interact()?;

    let build_tools = vec!["Maven", "Gradle"];
    let build_tool_idx = Select::with_theme(theme)
        .with_prompt("Build tool")
        .items(&build_tools)
        .default(0)
        .interact()?;

    let build_tool = match build_tool_idx {
        0 => BuildTool::Maven,
        1 => BuildTool::Gradle,
        _ => BuildTool::Maven,
    };

    let gradle_dsl = if build_tool == BuildTool::Gradle {
        let dsl_options = vec!["Kotlin (build.gradle.kts)", "Groovy (build.gradle)"];
        let dsl_idx = Select::with_theme(theme)
            .with_prompt("Gradle DSL")
            .items(&dsl_options)
            .default(0)
            .interact()?;
        match dsl_idx {
            0 => GradleDsl::Kotlin,
            1 => GradleDsl::Groovy,
            _ => GradleDsl::Kotlin,
        }
    } else {
        GradleDsl::Kotlin
    };

    Ok(NewArgs {
        name,
        group,
        boot_version: boot_versions[boot_idx].to_string(),
        java_version: java_versions[java_idx].parse()?,
        features: vec![],
        kafka_stack: vec![],
        build_tool,
        gradle_dsl,
        skip_format: false,
        output: None,
        force: false,
        emit_config: true,
    })
}
