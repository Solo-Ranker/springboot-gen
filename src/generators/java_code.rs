use anyhow::Result;
use handlebars::Handlebars;
use serde_json::json;
use std::path::Path;

use crate::config::ProjectConfig;
use crate::engine::to_class_name;
use crate::features::FeatureSpec;

macro_rules! tpl {
    ($name:literal) => {
        (
            $name,
            include_str!(concat!("../../templates/java/", $name, ".java.hbs")),
        )
    };
}

const TEMPLATES: &[(&str, &str)] = &[
    // Core
    tpl!("core/Application"),
    tpl!("core/ApiResponse"),
    tpl!("core/GlobalExceptionHandler"),
    tpl!("core/HealthController"),
    tpl!("core/NotFoundException"),
    // Redis
    tpl!("redis/RedisProperties"),
    tpl!("redis/RedisConfig"),
    tpl!("redis/CacheConfig"),
    // Kafka
    tpl!("kafka/KafkaConfig"),
    tpl!("kafka/KafkaTopicConfig"),
    tpl!("kafka/KafkaProducerService"),
    tpl!("kafka/KafkaConsumerService"),
    // Security
    tpl!("security/BasicSecurityConfig"),
    tpl!("security/JwtSecurityConfig"),
    tpl!("security/OAuth2SecurityConfig"),
    tpl!("security/JwtProperties"),
    tpl!("security/JwtService"),
    tpl!("security/JwtAuthenticationFilter"),
    tpl!("security/AuthController"),
    tpl!("security/TokenResponse"),
    tpl!("security/LoginRequest"),
    // Integrations
    tpl!("integrations/OpenApiConfig"),
    tpl!("integrations/S3Config"),
    tpl!("integrations/S3Service"),
    tpl!("integrations/EmailConfig"),
    tpl!("integrations/EmailService"),
    tpl!("integrations/WebSocketConfig"),
    // Database
    tpl!("database/JpaConfig"),
    tpl!("todo/TodoEntity"),
    tpl!("todo/TodoFilter"),
    tpl!("todo/TodoSpecification"),
    tpl!("todo/TodoRepository"),
    tpl!("todo/Todo"),
    tpl!("todo/TodoService"),
    tpl!("todo/TodoController"),
    // Database — standalone
    tpl!("database/standalone/DatabaseProperties"),
    tpl!("database/standalone/JdbcUrlBuilder"),
    tpl!("database/standalone/DatabaseConfig"),
    tpl!("database/standalone/DatabaseType"),
    // Database — replication
    tpl!("database/replication/DatabaseType"),
    tpl!("database/replication/DataSourceType"),
    tpl!("database/replication/DatabaseProperties"),
    tpl!("database/replication/JdbcUrlBuilder"),
    tpl!("database/replication/HikariDataSourceFactory"),
    tpl!("database/replication/DatabaseConfig"),
    tpl!("database/replication/LoadBalanceRoutingDataSource"),
    tpl!("database/replication/RoutingDataSourceContext"),
    tpl!("database/replication/TransactionRoutingAspect"),
];

pub struct JavaCodeGenerator<'a> {
    config: &'a ProjectConfig,
    features: &'a [FeatureSpec],
    hb: Handlebars<'static>,
}

impl<'a> JavaCodeGenerator<'a> {
    pub fn new(config: &'a ProjectConfig, features: &'a [FeatureSpec]) -> Result<Self> {
        let mut hb = Handlebars::new();
        hb.set_strict_mode(false);
        hb.register_escape_fn(handlebars::no_escape);
        for (name, src) in TEMPLATES {
            hb.register_template_string(name, src)?;
        }
        Ok(Self {
            config,
            features,
            hb,
        })
    }

    pub fn generate(&self, out: &Path, package_path: &str, artifact: &str) -> Result<()> {
        let group = &self.config.project.group;
        let package = format!("{}.{}", group, artifact);

        let base = out.join("src/main/java").join(package_path).join(artifact);

        // ── Core ─────────────────────────────────────────────────────────────
        self.write(
            &base,
            &format!("{}Application.java", to_class_name(artifact)),
            "core/Application",
            &json!({ "package": package, "className": to_class_name(artifact) }),
        )?;

        self.write(
            &base.join("exception"),
            "GlobalExceptionHandler.java",
            "core/GlobalExceptionHandler",
            &json!({ "package": package }),
        )?;

        self.write(
            &base.join("exception"),
            "NotFoundException.java",
            "core/NotFoundException",
            &json!({ "package": package }),
        )?;

        self.write(
            &base.join("dto"),
            "ApiResponse.java",
            "core/ApiResponse",
            &json!({ "package": package }),
        )?;

        self.write(
            &base.join("controller"),
            "HealthController.java",
            "core/HealthController",
            &json!({ "package": package }),
        )?;

        // ── Dynamic Feature Templates ──────────────────────────────────────────
        let ctx = json!({ "package": package, "project_name": self.config.project.name, "className": to_class_name(artifact) });

        for feature in self.features {
            for (tpl_key, dest_path) in &feature.java_files {
                let parts: Vec<&str> = dest_path.split('/').collect();
                let class_name = parts.last().unwrap();
                
                let mut dest_dir = base.clone();
                for folder in &parts[..parts.len() - 1] {
                    dest_dir = dest_dir.join(folder);
                }

                // Verify the template exists in the macro array
                let tpl_exists = TEMPLATES.iter().any(|(k, _)| k == tpl_key);
                if tpl_exists {
                    self.write(&dest_dir, &format!("{}.java", class_name), tpl_key, &ctx)?;
                } else {
                    eprintln!("Warning: Template '{}' defined in feature '{}' not found in TEMPLATES registry.", tpl_key, feature.key);
                }
            }
        }

        Ok(())
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn write(&self, dir: &Path, filename: &str, tpl: &str, data: &serde_json::Value) -> Result<()> {
        std::fs::create_dir_all(dir)?;
        let path = dir.join(filename);
        if !path.exists() {
            std::fs::write(&path, self.hb.render(tpl, data)?)?;
        }
        Ok(())
    }
}
