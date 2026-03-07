use crate::cli::{AddArgs, NewArgs};
use crate::config::ProjectConfig;
use crate::features::{resolve_features, FeatureSpec};
use crate::generators::{
    docker::DockerGenerator, env_file::EnvFileGenerator, java_code::JavaCodeGenerator,
    kubernetes::KubernetesGenerator, pom::PomGenerator, spring_properties::PropertiesGenerator,
};
use anyhow::{Context, Result};
use console::style;
use handlebars::Handlebars;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};

/// The central generation engine — resolves features, then fans out to generators
macro_rules! tpl {
    ($name:literal) => {
        (
            $name,
            include_str!(concat!("../../templates/misc/", $name, ".hbs")),
        )
    };
}

const MISC_TEMPLATES: &[(&str, &str)] = &[
    tpl!("gitignore"),
    tpl!("postgres/V1_0_0__CREATE_TODOS_TABLE.sql"),
    tpl!("mysql/V1_0_0__CREATE_TODOS_TABLE.sql"),
];

pub struct GenerationEngine {
    hb: Handlebars<'static>,
}

impl GenerationEngine {
    pub fn new() -> Result<Self> {
        let mut hb = Handlebars::new();
        hb.set_strict_mode(false);
        hb.register_escape_fn(handlebars::no_escape);
        for (name, src) in MISC_TEMPLATES {
            hb.register_template_string(name, src)?;
        }
        Ok(Self { hb })
    }

    pub fn generate_project(
        &self,
        args: NewArgs,
        config_override: Option<ProjectConfig>,
        preview: bool,
    ) -> Result<()> {
        let features = resolve_features(&args.features)?;
        let config = config_override.unwrap_or_else(|| ProjectConfig::from_new_args(&args));
        let out_dir = args
            .output
            .clone()
            .unwrap_or_else(|| PathBuf::from(&args.name));

        if preview {
            return self.print_preview(&args.name, &features, &config);
        }

        if out_dir.exists() && !args.force {
            anyhow::bail!(
                "Directory '{}' already exists. Use --force to overwrite.",
                out_dir.display()
            );
        }

        let package_path = args.group.replace('.', "/");
        let artifact = to_artifact_id(&args.name);
        self.create_dir_tree(&out_dir, &package_path, &artifact, &features)?;

        let pb = self.progress_bar(7 + features.len() as u64);

        match config.project.build_tool.as_str() {
            "maven" => {
                pb.set_message("Generating pom.xml");
                PomGenerator::new(&config, &features)?.generate(&out_dir)?;
            }
            "gradle" => {
                let filename = if config.project.gradle_dsl == "groovy" {
                    "build.gradle"
                } else {
                    "build.gradle.kts"
                };
                pb.set_message(format!("Generating {filename}"));
                crate::generators::gradle::GradleGenerator::new(&config, &features)?
                    .generate(&out_dir)?;
            }
            _ => anyhow::bail!("Unsupported build tool: {}", config.project.build_tool),
        }
        pb.inc(1);

        pb.set_message("Generating application.yml");
        PropertiesGenerator::new(&config, &features)?.generate(&out_dir)?;
        pb.inc(1);

        pb.set_message("Generating .env / .env.example");
        EnvFileGenerator::new(&config, &features)?.generate(&out_dir)?;
        pb.inc(1);

        if features.iter().any(|f| f.key == "docker") {
            pb.set_message("Generating Dockerfile + feature docker configs");
            DockerGenerator::new(&config, &features)?.generate(&out_dir)?;
        }
        pb.inc(1);

        if features.iter().any(|f| f.key == "kubernetes") {
            pb.set_message("Generating Kubernetes manifests");
            KubernetesGenerator::new(&config, &features)?.generate(&out_dir)?;
        }
        pb.inc(1);

        pb.set_message("Generating Java configuration classes");
        JavaCodeGenerator::new(&config, &features)?.generate(&out_dir, &package_path, &artifact)?;
        pb.inc(1);

        if features
            .iter()
            .any(|f| f.key == "postgres" || f.key == "mysql")
        {
            pb.set_message("Generating Flyway migration");
            self.generate_flyway_migration(&out_dir, &features)?;
        }
        pb.inc(1);

        if args.emit_config {
            pb.set_message("Writing springboot-gen.toml");
            config.save(&out_dir.join("springboot-gen.toml"))?;
        }
        pb.inc(1);

        self.generate_gitignore(&out_dir)?;
        pb.finish_with_message("Done!");

        if !args.skip_format {
            self.run_formatter(&out_dir, &config)?;
        }

        self.print_success(&args.name, &out_dir, &features, &config);
        Ok(())
    }

    pub fn add_features(&self, args: AddArgs) -> Result<()> {
        let config_path = args.path.join("springboot-gen.toml");
        if !config_path.exists() {
            anyhow::bail!(
                "No springboot-gen.toml found in '{}'. Use `springboot-gen analyze` on non-SpringbootGen projects.",
                args.path.display()
            );
        }

        let mut config = ProjectConfig::load(&config_path)?;
        let new_keys: Vec<String> = args
            .features
            .iter()
            .filter(|f| !config.features.contains(f))
            .cloned()
            .collect();

        if new_keys.is_empty() {
            println!(
                "{} All requested features already present",
                style("✓").green()
            );
            return Ok(());
        }

        println!(
            "{} Adding features: {}",
            style("→").cyan(),
            new_keys.join(", ")
        );

        let all_features = {
            let mut combined = config.features.clone();
            combined.extend(new_keys.clone());
            combined
        };
        let features = resolve_features(&all_features)?;

        let pb = self.progress_bar(4);

        pb.set_message("Updating build file");
        match config.project.build_tool.as_str() {
            "maven" => PomGenerator::new(&config, &features)?.generate(&args.path)?,
            "gradle" => crate::generators::gradle::GradleGenerator::new(&config, &features)?
                .generate(&args.path)?,
            _ => {}
        }
        pb.inc(1);

        pb.set_message("Updating application.yml");
        PropertiesGenerator::new(&config, &features)?.generate(&args.path)?;
        pb.inc(1);

        pb.set_message("Updating feature docker configs");
        if features.iter().any(|f| f.key == "docker") {
            DockerGenerator::new(&config, &features)?.generate(&args.path)?;
        }
        pb.inc(1);

        pb.set_message("Generating new Java config classes");
        let package_path = config.project.group.replace('.', "/");
        let artifact = to_artifact_id(&config.project.name);
        JavaCodeGenerator::new(&config, &features)?.generate(
            &args.path,
            &package_path,
            &artifact,
        )?;
        pb.inc(1);

        pb.finish_with_message("Done!");

        config.features = all_features;
        config.save(&config_path)?;

        println!(
            "\n{} Added: {}\n",
            style("✓").green().bold(),
            new_keys
                .iter()
                .map(|k| style(k).cyan().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );

        Ok(())
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    fn create_dir_tree(
        &self,
        out: &Path,
        package_path: &str,
        artifact: &str,
        _features: &[FeatureSpec],
    ) -> Result<()> {
        let src_main = out.join("src/main/java").join(package_path).join(artifact);

        let src_test = out.join("src/test/java").join(package_path).join(artifact);

        let dirs = vec![
            src_main.join("config"),
            src_main.join("controller"),
            src_main.join("service"),
            src_main.join("repository"),
            src_main.join("model"),
            src_main.join("dto"),
            src_main.join("exception"),
            src_main.join("util"),
            src_test.join("integration"),
            src_test.join("unit"),
            out.join("src/main/resources/db/migration"),
            out.join("src/main/resources/templates"),
        ];

        for dir in &dirs {
            std::fs::create_dir_all(dir)
                .with_context(|| format!("Failed to create dir: {}", dir.display()))?;
        }

        Ok(())
    }

    fn generate_gitignore(&self, out: &Path) -> Result<()> {
        let content = self.hb.render("gitignore", &serde_json::json!({}))?;
        std::fs::write(out.join(".gitignore"), content)?;
        Ok(())
    }

    fn generate_flyway_migration(&self, out: &Path, features: &[FeatureSpec]) -> Result<()> {
        let migration_file =
            out.join("src/main/resources/db/migration/V1_0_0__CREATE_TODOS_TABLE.sql");

        if migration_file.exists() {
            return Ok(());
        }

        let template = if features.iter().any(|f| f.key == "mysql") {
            "mysql/V1_0_0__CREATE_TODOS_TABLE.sql"
        } else {
            "postgres/V1_0_0__CREATE_TODOS_TABLE.sql"
        };

        let content = self.hb.render(template, &serde_json::json!({}))?;
        std::fs::write(migration_file, content)?;

        Ok(())
    }

    fn run_formatter(&self, out: &Path, config: &ProjectConfig) -> Result<()> {
        use console::style;
        use std::process::Command;

        println!("\n{} Formatting generated code...", style("→").cyan());

        let (cmd, args) = match config.project.build_tool.as_str() {
            "gradle" => {
                let gradlew = if cfg!(windows) {
                    "gradlew.bat"
                } else {
                    "./gradlew"
                };
                (gradlew.to_string(), vec!["spotlessApply"])
            }
            _ => {
                let mvnw = "mvn";
                (mvnw.to_string(), vec!["spotless:apply"])
            }
        };

        let output = Command::new(&cmd).args(&args).current_dir(out).output();

        match output {
            Ok(result) => {
                if result.status.success() {
                    println!("  {} Code formatted successfully", style("✓").green());
                } else {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    eprintln!(
                        "  {} Formatting failed (non-fatal): {}",
                        style("⚠").yellow(),
                        stderr.lines().next().unwrap_or("unknown error")
                    );
                }
            }
            Err(e) => {
                eprintln!(
                    "  {} Could not run formatter (non-fatal): {}",
                    style("⚠").yellow(),
                    e
                );
            }
        }

        Ok(())
    }

    fn print_preview(
        &self,
        name: &str,
        features: &[FeatureSpec],
        _config: &ProjectConfig,
    ) -> Result<()> {
        println!(
            "\n{} Preview for project: {}\n",
            style("◆").cyan(),
            style(name).bold()
        );
        println!("{}", style("Resolved features:").yellow().bold());
        for f in features {
            println!(
                "  {} {} — {}",
                style("•").dim(),
                style(f.key).green(),
                f.description
            );
        }

        println!(
            "\n{}",
            style("Files that would be generated:").yellow().bold()
        );
        let files = vec![
            "pom.xml",
            "src/main/resources/application.yml",
            "src/main/resources/application-dev.yml",
            ".env.example",
            "docker/",
            "Dockerfile",
        ];
        for f in &files {
            println!("  {} {}", style("•").dim(), f);
        }

        let dep_count: usize = features.iter().map(|f| f.maven_deps.len()).sum();
        let svc_count: usize = features.iter().map(|f| f.docker_services.len()).sum();
        let java_count: usize = features.iter().map(|f| f.java_files.len()).sum();

        println!("\n{}", style("Summary:").yellow().bold());
        println!("  Maven dependencies:  {}", dep_count);
        println!("  Docker services:     {}", svc_count);
        println!("  Java config classes: {}", java_count);

        Ok(())
    }

    fn print_success(
        &self,
        name: &str,
        out: &Path,
        features: &[FeatureSpec],
        config: &ProjectConfig,
    ) {
        println!("\n{}", style("═".repeat(60)).dim());
        println!(
            "  {} Project {} is ready!\n",
            style("🎉").bold(),
            style(name).cyan().bold()
        );
        println!("  {}", style(out.display()).dim());

        println!("\n  {}", style("Features included:").bold());
        for f in features {
            println!("    {} {}", style("✓").green(), f.name);
        }

        println!("\n  {}", style("Next steps:").bold());
        println!("    # Start specific infra: docker compose -f docker/<feature>/docker-compose.yml up -d");

        match config.project.build_tool.as_str() {
            "gradle" => {
                println!("    ./gradlew bootRun        # Start the app");
            }
            _ => {
                println!("    mvn spring-boot:run   # Start the app");
            }
        }

        if let Some(boot) = config.project.boot_version.chars().next() {
            let _ = boot;
        }
        let has_swagger = features.iter().any(|f| f.key == "openapi");
        if has_swagger {
            println!("    open http://localhost:8080/swagger-ui.html");
        }
        let has_kafka_ui = features.iter().any(|f| f.key == "kafka");
        if has_kafka_ui {
            println!("    open http://localhost:8090  # Kafka UI");
        }
        println!("{}\n", style("═".repeat(60)).dim());
    }

    fn progress_bar(&self, len: u64) -> ProgressBar {
        let pb = ProgressBar::new(len);
        pb.set_style(
            ProgressStyle::with_template(
                "  {spinner:.cyan} [{elapsed_precise}] {bar:35.cyan/dim} {pos}/{len} {msg}",
            )
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
        );
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        pb
    }
}

/// Convert a project name to a Java artifact id (lowercase, no special chars)
pub fn to_artifact_id(name: &str) -> String {
    name.to_lowercase()
        .replace(['-', ' ', '.'], "_")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

/// Convert to a Java class name (PascalCase)
pub fn to_class_name(name: &str) -> String {
    name.split(['-', '_', ' '])
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}
