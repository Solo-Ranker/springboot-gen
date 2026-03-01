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
        let config_dir = base.join("config");
        let controller_dir = base.join("controller");
        let service_dir = base.join("service");
        let dto_dir = base.join("dto");
        let exception_dir = base.join("exception");

        // ── Core ─────────────────────────────────────────────────────────────
        self.write(
            &base,
            &format!("{}Application.java", to_class_name(artifact)),
            "core/Application",
            &json!({ "package": package, "className": to_class_name(artifact) }),
        )?;

        self.write(
            &exception_dir,
            "GlobalExceptionHandler.java",
            "core/GlobalExceptionHandler",
            &json!({ "package": package }),
        )?;

        self.write(
            &exception_dir,
            "NotFoundException.java",
            "core/NotFoundException",
            &json!({ "package": package }),
        )?;

        self.write(
            &dto_dir,
            "ApiResponse.java",
            "core/ApiResponse",
            &json!({ "package": package }),
        )?;

        self.write(
            &controller_dir,
            "HealthController.java",
            "core/HealthController",
            &json!({ "package": package }),
        )?;

        // ── Redis ─────────────────────────────────────────────────────────────
        // All redis variants use the same unified RedisConfig + RedisProperties.
        // Mode (standalone/sentinel/cluster) and SSL are controlled via app.redis.* in YAML.
        let props_dir = base.join("config/properties");
        let ctx = json!({ "package": package });

        if self.has("redis")
            || self.has("redis-ssl")
            || self.has("redis-sentinel")
            || self.has("redis-ssl-sentinel")
            || self.has("redis-cluster")
        {
            self.write(
                &props_dir,
                "RedisProperties.java",
                "redis/RedisProperties",
                &ctx,
            )?;
            self.write(&config_dir, "RedisConfig.java", "redis/RedisConfig", &ctx)?;
            self.write(&config_dir, "CacheConfig.java", "redis/CacheConfig", &ctx)?;
        }

        // ── Kafka ─────────────────────────────────────────────────────────────
        if self.has("kafka") {
            self.write(&config_dir, "KafkaConfig.java", "kafka/KafkaConfig", &ctx)?;
            self.write(
                &config_dir,
                "KafkaTopicConfig.java",
                "kafka/KafkaTopicConfig",
                &ctx,
            )?;
            self.write(
                &service_dir,
                "KafkaProducerService.java",
                "kafka/KafkaProducerService",
                &ctx,
            )?;
            self.write(
                &service_dir,
                "KafkaConsumerService.java",
                "kafka/KafkaConsumerService",
                &ctx,
            )?;
        }

        // ── Security ──────────────────────────────────────────────────────────
        if self.has("security") && !self.has("jwt") && !self.has("oauth2") {
            self.write(
                &config_dir,
                "SecurityConfig.java",
                "security/BasicSecurityConfig",
                &ctx,
            )?;
        }
        if self.has("jwt") {
            self.write(
                &config_dir,
                "SecurityConfig.java",
                "security/JwtSecurityConfig",
                &ctx,
            )?;
            self.write(
                &config_dir,
                "JwtProperties.java",
                "security/JwtProperties",
                &ctx,
            )?;
            self.write(&service_dir, "JwtService.java", "security/JwtService", &ctx)?;
            self.write(
                &config_dir,
                "JwtAuthenticationFilter.java",
                "security/JwtAuthenticationFilter",
                &ctx,
            )?;
            self.write(
                &controller_dir,
                "AuthController.java",
                "security/AuthController",
                &ctx,
            )?;
            self.write(
                &dto_dir,
                "TokenResponse.java",
                "security/TokenResponse",
                &ctx,
            )?;
            self.write(&dto_dir, "LoginRequest.java", "security/LoginRequest", &ctx)?;
        }
        if self.has("oauth2") {
            self.write(
                &config_dir,
                "SecurityConfig.java",
                "security/OAuth2SecurityConfig",
                &ctx,
            )?;
        }

        // ── Integrations ──────────────────────────────────────────────────────
        if self.has("openapi") {
            let ctx = json!({ "package": package, "project_name": self.config.project.name });
            self.write(
                &config_dir,
                "OpenApiConfig.java",
                "integrations/OpenApiConfig",
                &ctx,
            )?;
        }
        if self.has("s3") {
            self.write(&config_dir, "S3Config.java", "integrations/S3Config", &ctx)?;
            self.write(
                &service_dir,
                "S3Service.java",
                "integrations/S3Service",
                &ctx,
            )?;
        }
        if self.has("email") {
            self.write(
                &config_dir,
                "EmailConfig.java",
                "integrations/EmailConfig",
                &ctx,
            )?;
            self.write(
                &service_dir,
                "EmailService.java",
                "integrations/EmailService",
                &ctx,
            )?;
        }
        if self.has("websocket") {
            self.write(
                &config_dir,
                "WebSocketConfig.java",
                "integrations/WebSocketConfig",
                &ctx,
            )?;
        }

        // ── Database ──────────────────────────────────────────────────────────
        let ctx = json!({ "package": package });
        let builder_dir = base.join("common/builder");
        let enums_dir = base.join("common/enums");
        let routing_dir = config_dir.join("routing");
        let props_dir = base.join("config/properties");
        let model_dir = base.join("model");
        let specification_dir = base.join("common/specification");
        let repository_dir = base.join("repository");
        let service_dir = base.join("service");
        let controller_dir = base.join("controller");

        // DB - we will add to the sample crud api
        if self.has("postgres") | self.has("mysql") {
            self.write(&model_dir, "TodoEntity.java", "todo/TodoEntity", &ctx)?;
            self.write(&dto_dir, "TodoFilter.java", "todo/TodoFilter", &ctx)?;
            self.write(
                &specification_dir,
                "TodoSpecification.java",
                "todo/TodoSpecification",
                &ctx,
            )?;
            self.write(
                &repository_dir,
                "TodoRepository.java",
                "todo/TodoRepository",
                &ctx,
            )?;
            self.write(&dto_dir, "Todo.java", "todo/Todo", &ctx)?;
            self.write(&service_dir, "TodoService.java", "todo/TodoService", &ctx)?;
            self.write(
                &controller_dir,
                "TodoController.java",
                "todo/TodoController",
                &ctx,
            )?;
        }

        // Standalone DB: postgres | mysql
        if self.has("database-standalone") {
            self.write(&config_dir, "JpaConfig.java", "database/JpaConfig", &ctx)?;
            self.write(
                &enums_dir,
                "DatabaseType.java",
                "database/standalone/DatabaseType",
                &ctx,
            )?;
            self.write(
                &props_dir,
                "DatabaseProperties.java",
                "database/standalone/DatabaseProperties",
                &ctx,
            )?;
            self.write(
                &builder_dir,
                "JdbcUrlBuilder.java",
                "database/standalone/JdbcUrlBuilder",
                &ctx,
            )?;
            self.write(
                &config_dir,
                "DatabaseConfig.java",
                "database/standalone/DatabaseConfig",
                &ctx,
            )?;
        }

        // Replication DB: master-slave setup
        if self.has("database-replication") {
            self.write(&config_dir, "JpaConfig.java", "database/JpaConfig", &ctx)?;
            // Enums
            self.write(
                &enums_dir,
                "DatabaseType.java",
                "database/replication/DatabaseType",
                &ctx,
            )?;
            self.write(
                &enums_dir,
                "DataSourceType.java",
                "database/replication/DataSourceType",
                &ctx,
            )?;
            // Properties
            self.write(
                &props_dir,
                "DatabaseProperties.java",
                "database/replication/DatabaseProperties",
                &ctx,
            )?;
            // Builders & factories
            self.write(
                &builder_dir,
                "JdbcUrlBuilder.java",
                "database/replication/JdbcUrlBuilder",
                &ctx,
            )?;
            self.write(
                &config_dir,
                "HikariDataSourceFactory.java",
                "database/replication/HikariDataSourceFactory",
                &ctx,
            )?;
            // Config
            self.write(
                &config_dir,
                "DatabaseConfig.java",
                "database/replication/DatabaseConfig",
                &ctx,
            )?;
            // Routing
            self.write(
                &routing_dir,
                "RoutingDataSourceContext.java",
                "database/replication/RoutingDataSourceContext",
                &ctx,
            )?;
            self.write(
                &routing_dir,
                "LoadBalanceRoutingDataSource.java",
                "database/replication/LoadBalanceRoutingDataSource",
                &ctx,
            )?;
            self.write(
                &routing_dir,
                "TransactionRoutingAspect.java",
                "database/replication/TransactionRoutingAspect",
                &ctx,
            )?;
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

    fn has(&self, key: &str) -> bool {
        self.features.iter().any(|f| f.key == key)
    }
}
