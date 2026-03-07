use anyhow::Result;
use console::style;
use indexmap::IndexMap;
use regex::Regex;
use std::path::Path;
use walkdir::WalkDir;

use crate::cli::{AnalyzeFormat, ImportArgs};
use crate::config::{
    DatabaseConfig, DockerConfig, IbmMqConfig, KafkaConfig, ProjectConfig, ProjectMeta,
    RabbitMqConfig, RedisConfig, RedisSentinelConfig, RedisSslConfig, SecurityConfig,
};

/// Inspection result from scanning an existing project
#[derive(Debug, Clone)]
pub struct AnalysisReport {
    pub project_name: String,
    pub group_id: String,
    pub boot_version: String,
    pub java_version: u8,
    pub detected_features: Vec<DetectedFeature>,
    pub redis_mode: Option<RedisMode>,
    pub kafka_config: Option<KafkaDetails>,
    pub db_type: Option<DatabaseType>,
    pub warnings: Vec<String>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DetectedFeature {
    pub key: String,
    pub confidence: Confidence,
    pub source: String, // where it was detected (pom.xml / application.yml / etc.)
}

#[derive(Debug, Clone, PartialEq)]
pub enum Confidence {
    High,   // Dependency + configuration both present
    Medium, // Only in pom.xml or only in config
    Low,    // Inferred from patterns
}

#[derive(Debug, Clone)]
pub enum RedisMode {
    Standalone,
    Ssl,
    Sentinel,
    Cluster,
}

#[derive(Debug, Clone)]
pub struct KafkaDetails {
    pub bootstrap_servers: String,
    pub consumer_group: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DatabaseType {
    Postgres,
    Mysql,
    MongoDB,
    H2,
}

pub struct ProjectAnalyzer;

impl ProjectAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Analyze a project directory
    pub fn analyze(&self, path: &Path, format: AnalyzeFormat, save: bool) -> Result<()> {
        println!(
            "\n{} Analyzing project at: {}\n",
            style("🔍").bold(),
            style(path.display()).cyan()
        );

        let report = self.scan(path)?;

        match format {
            AnalyzeFormat::Table => self.print_table(&report),
            AnalyzeFormat::Json => {
                let config = self.report_to_config(&report);
                println!("{}", serde_json::to_string_pretty(&config)?);
            }
            AnalyzeFormat::Toml => {
                let config = self.report_to_config(&report);
                println!("{}", toml::to_string_pretty(&config)?);
            }
        }

        if save {
            let config = self.report_to_config(&report);
            let out = path.join("springboot-gen.toml");
            config.save(&out)?;
            println!(
                "\n{} Saved config to: {}",
                style("✓").green().bold(),
                style(out.display()).cyan()
            );
        }

        Ok(())
    }

    /// Analyze an existing project and generate supplemental SpringbootGen files
    pub fn import_and_supplement(&self, args: ImportArgs) -> Result<()> {
        println!(
            "\n{} Importing project: {}\n",
            style("📥").bold(),
            style(args.path.display()).cyan()
        );

        let mut report = self.scan(&args.path)?;

        // Allow manual feature override/supplement
        if !args.features.is_empty() {
            println!(
                "  {} Supplementing with extra features: {}",
                style("→").cyan(),
                args.features.join(", ")
            );
            for feat in &args.features {
                if !report.detected_features.iter().any(|f| &f.key == feat) {
                    report.detected_features.push(DetectedFeature {
                        key: feat.clone(),
                        confidence: Confidence::High,
                        source: "manual".to_string(),
                    });
                }
            }
        }

        let config = self.report_to_config(&report);
        let feature_keys: Vec<String> = report
            .detected_features
            .iter()
            .map(|f| f.key.clone())
            .collect();
        let features = crate::features::resolve_features(&feature_keys)?;

        std::fs::create_dir_all(&args.output)?;

        // Generate only the supplement files (don't overwrite Java code)
        println!(
            "{}",
            style("Generating supplemental files:").yellow().bold()
        );

        crate::generators::spring_properties::PropertiesGenerator::new(&config, &features)?
            .generate(&args.output)?;
        println!(
            "  {} application.yml (updated with detected config)",
            style("✓").green()
        );

        crate::generators::env_file::EnvFileGenerator::new(&config, &features)?
            .generate(&args.output)?;
        println!("  {} .env.example", style("✓").green());

        if features.iter().any(|f| f.key == "docker") {
            crate::generators::docker::DockerGenerator::new(&config, &features)?
                .generate(&args.output)?;
            println!(
                "  {} feature docker folders + Dockerfile",
                style("✓").green()
            );
        }

        if features.iter().any(|f| f.key == "kubernetes") {
            crate::generators::kubernetes::KubernetesGenerator::new(&config, &features)?
                .generate(&args.output)?;
            println!("  {} k8s/ manifests", style("✓").green());
        }

        config.save(&args.output.join("springboot-gen.toml"))?;
        println!("  {} springboot-gen.toml", style("✓").green());

        self.print_import_guide(&report, &args.output);

        Ok(())
    }

    // ── Scanning engine ────────────────────────────────────────────────────────

    fn scan(&self, root: &Path) -> Result<AnalysisReport> {
        // ── Read pom.xml ─────────────────────────────────────────────────────
        let pom_path = root.join("pom.xml");
        let pom = if pom_path.exists() {
            std::fs::read_to_string(&pom_path)?
        } else {
            // Also check for build.gradle
            let gradle_path = root.join("build.gradle");
            if gradle_path.exists() {
                std::fs::read_to_string(&gradle_path)?
            } else {
                anyhow::bail!(
                    "No pom.xml or build.gradle found in '{}'. Is this a Spring Boot project?",
                    root.display()
                );
            }
        };

        // ── Read application.yml / application.properties ────────────────────
        let mut yml_content = String::new();
        let yml_paths = [
            "src/main/resources/application.yml",
            "src/main/resources/application.yaml",
            "src/main/resources/application.properties",
        ];
        for p in &yml_paths {
            let full = root.join(p);
            if full.exists() {
                yml_content.push_str(&std::fs::read_to_string(full)?);
                yml_content.push('\n');
            }
        }

        // ── Scan Java source files for annotations ───────────────────────────
        let mut java_annotations: Vec<String> = Vec::new();
        let src_root = root.join("src/main/java");
        if src_root.exists() {
            for entry in WalkDir::new(&src_root)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().is_some_and(|ext| ext == "java"))
            {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    java_annotations.push(content);
                }
            }
        }
        let java_source = java_annotations.join("\n");

        // ── Extract project metadata ─────────────────────────────────────────
        let project_name = self
            .extract_pom_value(&pom, "artifactId")
            .unwrap_or_else(|| "unknown".to_string());
        let group_id = self
            .extract_pom_value(&pom, "groupId")
            .unwrap_or_else(|| "com.example".to_string());
        let boot_version = self
            .extract_boot_version(&pom)
            .unwrap_or_else(|| "3.2.5".to_string());
        let java_version = self.extract_java_version(&pom, &yml_content).unwrap_or(21);

        // ── Detect features ──────────────────────────────────────────────────
        let mut features: Vec<DetectedFeature> = Vec::new();
        let mut warnings: Vec<String> = Vec::new();
        let mut recommendations: Vec<String> = Vec::new();

        // ── Redis detection ──────────────────────────────────────────────────
        let has_redis_dep = pom.contains("spring-boot-starter-data-redis");
        let has_lettuce = pom.contains("lettuce-core");
        let has_jedis = pom.contains("jedis");

        let redis_mode = if has_redis_dep || has_lettuce {
            let mode = self.detect_redis_mode(&yml_content, &java_source);
            let feat_key = match mode {
                RedisMode::Ssl => "redis-ssl",
                RedisMode::Sentinel => "redis-sentinel",
                RedisMode::Cluster => "redis-ssl", // map cluster to itself, but we don't have a cluster feature key yet
                RedisMode::Standalone => "redis",
            };
            features.push(DetectedFeature {
                key: feat_key.to_string(),
                confidence: if has_redis_dep && yml_content.contains("redis") {
                    Confidence::High
                } else {
                    Confidence::Medium
                },
                source: "pom.xml + application.yml".to_string(),
            });
            if has_jedis && !has_lettuce {
                warnings.push(
                    "Jedis client detected. SpringbootGen uses Lettuce — migration may be needed."
                        .to_string(),
                );
            }
            Some(mode)
        } else {
            None
        };

        // ── Kafka detection ──────────────────────────────────────────────────
        let has_kafka = pom.contains("spring-kafka");
        let kafka_details = if has_kafka {
            let bootstrap = self
                .extract_yml_value(&yml_content, "bootstrap-servers")
                .unwrap_or_else(|| "${KAFKA_BOOTSTRAP_SERVERS:localhost:9092}".to_string());
            let group = self.extract_yml_value(&yml_content, "group-id");

            features.push(DetectedFeature {
                key: "kafka".to_string(),
                confidence: if yml_content.contains("kafka") {
                    Confidence::High
                } else {
                    Confidence::Medium
                },
                source: "pom.xml".to_string(),
            });
            Some(KafkaDetails {
                bootstrap_servers: bootstrap,
                consumer_group: group,
            })
        } else {
            None
        };

        // ── Database detection ───────────────────────────────────────────────
        let db_type = if pom.contains("postgresql")
            || pom.contains("postgresql")
            || yml_content.contains("postgresql")
        {
            features.push(DetectedFeature {
                key: "postgres".to_string(),
                confidence: Confidence::High,
                source: "pom.xml".to_string(),
            });
            Some(DatabaseType::Postgres)
        } else if pom.contains("mysql-connector") || yml_content.contains("mysql") {
            features.push(DetectedFeature {
                key: "mysql".to_string(),
                confidence: Confidence::High,
                source: "pom.xml".to_string(),
            });
            Some(DatabaseType::Mysql)
        } else if pom.contains("spring-boot-starter-data-mongodb") {
            features.push(DetectedFeature {
                key: "mongodb".to_string(),
                confidence: Confidence::High,
                source: "pom.xml".to_string(),
            });
            Some(DatabaseType::MongoDB)
        } else if pom.contains("h2") {
            warnings.push(
                "H2 in-memory database detected. For production, switch to postgres or mysql."
                    .to_string(),
            );
            Some(DatabaseType::H2)
        } else {
            None
        };

        // ── Security detection ───────────────────────────────────────────────
        if pom.contains("spring-boot-starter-security") {
            let is_jwt = pom.contains("jjwt")
                || java_source.contains("JwtService")
                || java_source.contains("JwtUtil");
            let is_oauth2 = pom.contains("oauth2-resource-server") || pom.contains("oauth2-client");

            if is_jwt {
                features.push(DetectedFeature {
                    key: "security".to_string(),
                    confidence: Confidence::High,
                    source: "pom.xml".to_string(),
                });
                features.push(DetectedFeature {
                    key: "jwt".to_string(),
                    confidence: Confidence::High,
                    source: "pom.xml + Java source".to_string(),
                });
            } else if is_oauth2 {
                features.push(DetectedFeature {
                    key: "security".to_string(),
                    confidence: Confidence::High,
                    source: "pom.xml".to_string(),
                });
                features.push(DetectedFeature {
                    key: "oauth2".to_string(),
                    confidence: Confidence::High,
                    source: "pom.xml".to_string(),
                });
            } else {
                features.push(DetectedFeature {
                    key: "security".to_string(),
                    confidence: Confidence::High,
                    source: "pom.xml".to_string(),
                });
            }
        }

        // ── Actuator ─────────────────────────────────────────────────────────
        if pom.contains("spring-boot-starter-actuator") {
            features.push(DetectedFeature {
                key: "actuator".to_string(),
                confidence: Confidence::High,
                source: "pom.xml".to_string(),
            });
        } else {
            recommendations.push(
                "Add 'actuator' feature for health checks and Prometheus metrics.".to_string(),
            );
        }

        // ── OpenAPI ──────────────────────────────────────────────────────────
        if pom.contains("springdoc-openapi") {
            features.push(DetectedFeature {
                key: "openapi".to_string(),
                confidence: Confidence::High,
                source: "pom.xml".to_string(),
            });
        }

        // ── Tracing ──────────────────────────────────────────────────────────
        if pom.contains("micrometer-tracing") || pom.contains("zipkin") {
            features.push(DetectedFeature {
                key: "tracing".to_string(),
                confidence: Confidence::High,
                source: "pom.xml".to_string(),
            });
        }

        // ── Elasticsearch ────────────────────────────────────────────────────
        if pom.contains("spring-boot-starter-data-elasticsearch") {
            features.push(DetectedFeature {
                key: "elasticsearch".to_string(),
                confidence: Confidence::High,
                source: "pom.xml".to_string(),
            });
        }

        // ── S3 ───────────────────────────────────────────────────────────────
        if pom.contains("software.amazon.awssdk") || pom.contains("s3") {
            features.push(DetectedFeature {
                key: "s3".to_string(),
                confidence: Confidence::Medium,
                source: "pom.xml".to_string(),
            });
        }

        // ── Email ────────────────────────────────────────────────────────────
        if pom.contains("spring-boot-starter-mail") {
            features.push(DetectedFeature {
                key: "email".to_string(),
                confidence: Confidence::High,
                source: "pom.xml".to_string(),
            });
        }

        // ── WebSocket ────────────────────────────────────────────────────────
        if pom.contains("spring-boot-starter-websocket") {
            features.push(DetectedFeature {
                key: "websocket".to_string(),
                confidence: Confidence::High,
                source: "pom.xml".to_string(),
            });
        }

        // ── Docker check ─────────────────────────────────────────────────────
        if root.join("Dockerfile").exists() || root.join("docker").exists() {
            features.push(DetectedFeature {
                key: "docker".to_string(),
                confidence: Confidence::High,
                source: "Dockerfile/docker folder".to_string(),
            });
        } else {
            recommendations.push("Add 'docker' feature for containerization support.".to_string());
        }

        // ── JWT secret strength check ────────────────────────────────────────
        if features.iter().any(|f| f.key == "jwt") {
            let jwt_secret = self
                .extract_yml_value(&yml_content, "jwt.secret")
                .or_else(|| self.extract_yml_value(&yml_content, "jwt-secret"));
            if let Some(secret) = jwt_secret {
                if secret.len() < 32 || secret.contains("secret") || secret.contains("password") {
                    warnings.push(
                        "JWT secret appears weak. Use a random 256-bit key in production."
                            .to_string(),
                    );
                }
            }
        }

        // ── DDL auto check ───────────────────────────────────────────────────
        if yml_content.contains("ddl-auto: create") || yml_content.contains("ddl-auto: create-drop")
        {
            warnings.push("spring.jpa.hibernate.ddl-auto is 'create' or 'create-drop' — dangerous in production! Use 'validate' with Flyway.".to_string());
        }

        Ok(AnalysisReport {
            project_name,
            group_id,
            boot_version,
            java_version,
            detected_features: features,
            redis_mode,
            kafka_config: kafka_details,
            db_type,
            warnings,
            recommendations,
        })
    }

    // ── Detection helpers ─────────────────────────────────────────────────────

    fn detect_redis_mode(&self, yml: &str, java: &str) -> RedisMode {
        if yml.contains("sentinel.master")
            || yml.contains("sentinel.nodes")
            || java.contains("RedisSentinelConfiguration")
            || java.contains("SentinelConfiguration")
        {
            return RedisMode::Sentinel;
        }
        if yml.contains("ssl.enabled: true")
            || yml.contains("ssl:\n    enabled: true")
            || yml.contains("useSsl")
            || yml.contains("SslOptions")
            || java.contains("useSsl()")
            || java.contains("RedisSslConfig")
        {
            return RedisMode::Ssl;
        }
        if yml.contains("cluster.nodes") || java.contains("RedisClusterConfiguration") {
            return RedisMode::Cluster;
        }
        RedisMode::Standalone
    }

    fn extract_pom_value(&self, pom: &str, tag: &str) -> Option<String> {
        let pattern = format!("<{}>(.*?)</{}>", tag, tag);
        let re = Regex::new(&pattern).ok()?;
        re.captures(pom)?
            .get(1)
            .map(|m| m.as_str().trim().to_string())
    }

    fn extract_boot_version(&self, pom: &str) -> Option<String> {
        // Look for spring-boot parent version
        let re = Regex::new(r#"spring-boot-starter-parent.*?<version>([\d.]+(?:-\w+)?)</version>"#)
            .ok()?;
        re.captures(pom)?
            .get(1)
            .map(|m| m.as_str().trim().to_string())
    }

    fn extract_java_version(&self, pom: &str, _yml: &str) -> Option<u8> {
        let patterns = [
            r#"<java.version>(\d+)</java.version>"#,
            r#"<source>(\d+)</source>"#,
        ];
        for pattern in &patterns {
            if let Ok(re) = Regex::new(pattern) {
                if let Some(cap) = re.captures(pom) {
                    if let Ok(v) = cap[1].parse::<u8>() {
                        return Some(v);
                    }
                }
            }
        }
        None
    }

    fn extract_yml_value(&self, yml: &str, key: &str) -> Option<String> {
        let pattern = format!(r#"{}:\s*(.+)"#, regex::escape(key));
        let re = Regex::new(&pattern).ok()?;
        re.captures(yml)?.get(1).map(|m| {
            m.as_str()
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string()
        })
    }

    // ── Report rendering ──────────────────────────────────────────────────────

    fn report_to_config(&self, report: &AnalysisReport) -> ProjectConfig {
        let features: Vec<String> = report
            .detected_features
            .iter()
            .map(|f| f.key.clone())
            .collect();

        let redis = match &report.redis_mode {
            Some(RedisMode::Sentinel) => RedisConfig {
                mode: "sentinel".into(),
                sentinel: RedisSentinelConfig {
                    master: "mymaster".into(),
                    nodes: vec![
                        "localhost:26379".into(),
                        "localhost:26380".into(),
                        "localhost:26381".into(),
                    ],
                    sentinel_password: None,
                },
                ..Default::default()
            },
            Some(RedisMode::Ssl) => RedisConfig {
                mode: "ssl".into(),
                port: 6380,
                ssl: RedisSslConfig {
                    keystore_location: "classpath:ssl/redis-client.p12".into(),
                    keystore_password: "changeit".into(),
                    truststore_location: "classpath:ssl/redis-truststore.p12".into(),
                    truststore_password: "changeit".into(),
                    verify_hostname: true,
                },
                ..Default::default()
            },
            _ => RedisConfig::default(),
        };

        let kafka = if let Some(k) = &report.kafka_config {
            KafkaConfig {
                bootstrap_servers: k.bootstrap_servers.clone(),
                consumer_group_id: k
                    .consumer_group
                    .clone()
                    .unwrap_or_else(|| "app-group".into()),
                ..Default::default()
            }
        } else {
            KafkaConfig::default()
        };

        let db_port = match report.db_type {
            Some(DatabaseType::Mysql) => 3306u16,
            _ => 5432u16,
        };

        ProjectConfig {
            project: ProjectMeta {
                name: report.project_name.clone(),
                group: report.group_id.clone(),
                version: "0.0.1-SNAPSHOT".into(),
                description: format!("{} — imported by SpringbootGen", report.project_name),
                java_version: report.java_version,
                boot_version: report.boot_version.clone(),
                build_tool: "maven".into(), // Default to Maven for imported projects
                gradle_dsl: "kotlin".into(), // Default to Kotlin DSL
            },
            features,
            redis,
            kafka,
            database: DatabaseConfig {
                port: db_port,
                ..Default::default()
            },
            rabbitmq: RabbitMqConfig::default(),
            ibmmq: IbmMqConfig::default(),
            security: SecurityConfig::default(),
            docker: DockerConfig::default(),
            extra_properties: IndexMap::new(),
        }
    }

    fn print_table(&self, report: &AnalysisReport) {
        println!("{}", style("Project Analysis Report").cyan().bold());
        println!("{}", style("─".repeat(60)).dim());
        println!("  {:<25} {}", style("Name:").bold(), report.project_name);
        println!("  {:<25} {}", style("Group:").bold(), report.group_id);
        println!(
            "  {:<25} {}",
            style("Boot Version:").bold(),
            report.boot_version
        );
        println!(
            "  {:<25} {}",
            style("Java Version:").bold(),
            report.java_version
        );

        println!("\n{}", style("Detected Features:").yellow().bold());
        for feat in &report.detected_features {
            let conf_badge = match feat.confidence {
                Confidence::High => style("HIGH  ").green(),
                Confidence::Medium => style("MEDIUM").yellow(),
                Confidence::Low => style("LOW   ").red(),
            };
            println!(
                "  {} [{conf}] {key:<20} via {source}",
                style("•").dim(),
                conf = conf_badge,
                key = style(&feat.key).cyan(),
                source = style(&feat.source).dim()
            );
        }

        if !report.warnings.is_empty() {
            println!("\n{}", style("Warnings:").red().bold());
            for w in &report.warnings {
                println!("  {} {}", style("⚠").yellow(), w);
            }
        }

        if !report.recommendations.is_empty() {
            println!("\n{}", style("Recommendations:").blue().bold());
            for r in &report.recommendations {
                println!("  {} {}", style("→").cyan(), r);
            }
        }

        println!();
    }

    fn print_import_guide(&self, report: &AnalysisReport, out: &Path) {
        println!("\n{}", style("═".repeat(60)).dim());
        println!("{}", style("Import complete!").cyan().bold());
        println!("\nGenerated files are in: {}", style(out.display()).cyan());
        println!(
            "\n{}",
            style("To integrate these into your project:").bold()
        );
        println!(
            "  1. Merge {} into your application.yml",
            out.join("src/main/resources/application.yml").display()
        );
        println!(
            "  2. Copy {} to your project root",
            out.join(".env.example").display()
        );
        if out.join("docker").exists() {
            println!("  3. Review and copy docker folder to your project root");
        } else {
            println!("  3. Review generated code");
        }
        println!("  4. Run: springboot-gen add <feature> --path <your-project>");

        if !report.warnings.is_empty() {
            println!("\n{}", style("Review these warnings:").red().bold());
            for w in &report.warnings {
                println!("  {} {}", style("⚠").yellow(), w);
            }
        }
        println!("{}\n", style("═".repeat(60)).dim());
    }
}
