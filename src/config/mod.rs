use anyhow::{Context, Result};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Top-level springgen.toml configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub project: ProjectMeta,
    pub features: Vec<String>,

    #[serde(default)]
    pub redis: RedisConfig,

    #[serde(default)]
    pub kafka: KafkaConfig,

    #[serde(default)]
    pub rabbitmq: RabbitMqConfig,

    #[serde(default)]
    pub ibmmq: IbmMqConfig,

    #[serde(default)]
    pub database: DatabaseConfig,

    #[serde(default)]
    pub security: SecurityConfig,

    #[serde(default)]
    pub docker: DockerConfig,

    #[serde(default)]
    pub extra_properties: IndexMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMeta {
    pub name: String,
    pub group: String,
    pub version: String,
    pub description: String,
    pub java_version: u8,
    pub boot_version: String,
    #[serde(default = "default_build_tool")]
    pub build_tool: String,
    #[serde(default = "default_gradle_dsl")]
    pub gradle_dsl: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RedisConfig {
    /// standalone | ssl | sentinel | cluster
    #[serde(default = "default_redis_mode")]
    pub mode: String,

    // ── Standalone / SSL fields ─────────────────────────
    #[serde(default = "default_redis_host")]
    pub host: String,

    #[serde(default = "default_redis_port")]
    pub port: u16,

    #[serde(default)]
    pub password: Option<String>,

    #[serde(default = "default_redis_db")]
    pub database: u8,

    // ── SSL-specific ────────────────────────────────────
    #[serde(default)]
    pub ssl: RedisSslConfig,

    // ── Sentinel-specific ───────────────────────────────
    #[serde(default)]
    pub sentinel: RedisSentinelConfig,

    // ── Connection pool ─────────────────────────────────
    #[serde(default = "default_pool_max_active")]
    pub pool_max_active: u8,

    #[serde(default = "default_pool_max_idle")]
    pub pool_max_idle: u8,

    #[serde(default)]
    pub pool_min_idle: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RedisSslConfig {
    /// Path to the JKS/P12 keystore file (or classpath: reference)
    #[serde(default)]
    pub keystore_location: String,

    #[serde(default)]
    pub keystore_password: String,

    /// Path to the truststore file
    #[serde(default)]
    pub truststore_location: String,

    #[serde(default)]
    pub truststore_password: String,

    /// Whether to verify the server hostname
    #[serde(default = "default_true")]
    pub verify_hostname: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RedisSentinelConfig {
    #[serde(default = "default_sentinel_master")]
    pub master: String,

    /// Comma-separated list of sentinel nodes (host:port)
    #[serde(default = "default_sentinel_nodes")]
    pub nodes: Vec<String>,

    #[serde(default)]
    pub sentinel_password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KafkaConfig {
    #[serde(default = "default_kafka_bootstrap")]
    pub bootstrap_servers: String,

    #[serde(default = "default_consumer_group")]
    pub consumer_group_id: String,

    #[serde(default = "default_offset_reset")]
    pub auto_offset_reset: String,

    #[serde(default = "default_listener_concurrency")]
    pub listener_concurrency: u8,

    #[serde(default = "default_true")]
    pub idempotent_producer: bool,

    #[serde(default)]
    pub sasl: Option<KafkaSaslConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaSaslConfig {
    pub mechanism: String,
    pub username: String,
    pub password: String,
    pub security_protocol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RabbitMqConfig {
    #[serde(default = "default_rabbitmq_host")]
    pub host: String,

    #[serde(default = "default_rabbitmq_port")]
    pub port: u16,

    #[serde(default = "default_rabbitmq_user")]
    pub username: String,

    #[serde(default = "default_rabbitmq_password")]
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IbmMqConfig {
    #[serde(default = "default_ibmmq_qm")]
    pub queue_manager: String,

    #[serde(default = "default_ibmmq_channel")]
    pub channel: String,

    #[serde(default = "default_ibmmq_host")]
    pub host: String,

    #[serde(default = "default_ibmmq_port")]
    pub port: u16,

    #[serde(default = "default_ibmmq_user")]
    pub username: String,

    #[serde(default)]
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DatabaseConfig {
    #[serde(default = "default_db_host")]
    pub host: String,

    #[serde(default = "default_db_port")]
    pub port: u16,

    #[serde(default = "default_db_name")]
    pub name: String,

    #[serde(default = "default_db_user")]
    pub username: String,

    #[serde(default)]
    pub password: String,

    #[serde(default = "default_pool_max_db")]
    pub pool_max_size: u8,

    #[serde(default = "default_true")]
    pub flyway_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecurityConfig {
    #[serde(default)]
    pub jwt_secret: Option<String>,

    #[serde(default = "default_access_token_ms")]
    pub access_token_expiry_ms: u64,

    #[serde(default = "default_refresh_token_ms")]
    pub refresh_token_expiry_ms: u64,

    #[serde(default)]
    pub oauth2_issuer_uri: Option<String>,

    #[serde(default)]
    pub cors_allowed_origins: Vec<String>,

    #[serde(default = "default_true")]
    pub csrf_disabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DockerConfig {
    #[serde(default = "default_registry")]
    pub registry: String,

    #[serde(default = "default_base_image")]
    pub base_image: String,

    #[serde(default = "default_app_port")]
    pub app_port: u16,
}

// ── Default value helpers ────────────────────────────────────────────────────

fn default_redis_mode() -> String {
    "standalone".into()
}
fn default_redis_host() -> String {
    "localhost".into()
}
fn default_redis_port() -> u16 {
    6379
}
fn default_redis_db() -> u8 {
    0
}
fn default_sentinel_master() -> String {
    "mymaster".into()
}
fn default_sentinel_nodes() -> Vec<String> {
    vec![
        "localhost:26379".into(),
        "localhost:26380".into(),
        "localhost:26381".into(),
    ]
}
fn default_pool_max_active() -> u8 {
    8
}
fn default_pool_max_idle() -> u8 {
    8
}
fn default_kafka_bootstrap() -> String {
    "localhost:9092".into()
}
fn default_consumer_group() -> String {
    "app-group".into()
}
fn default_offset_reset() -> String {
    "earliest".into()
}
fn default_listener_concurrency() -> u8 {
    3
}
fn default_rabbitmq_host() -> String {
    "localhost".into()
}
fn default_rabbitmq_port() -> u16 {
    5672
}
fn default_rabbitmq_user() -> String {
    "guest".into()
}
fn default_rabbitmq_password() -> String {
    "guest".into()
}
fn default_ibmmq_qm() -> String {
    "QM1".into()
}
fn default_ibmmq_channel() -> String {
    "DEV.APP.SVRCONN".into()
}
fn default_ibmmq_host() -> String {
    "localhost".into()
}
fn default_ibmmq_port() -> u16 {
    1414
}
fn default_ibmmq_user() -> String {
    "app".into()
}
fn default_db_host() -> String {
    "localhost".into()
}
fn default_db_port() -> u16 {
    5432
}
fn default_db_name() -> String {
    "appdb".into()
}
fn default_db_user() -> String {
    "postgres".into()
}
fn default_pool_max_db() -> u8 {
    10
}
fn default_access_token_ms() -> u64 {
    900_000
}
fn default_refresh_token_ms() -> u64 {
    604_800_000
}
fn default_true() -> bool {
    true
}
fn default_registry() -> String {
    "docker.io".into()
}
fn default_base_image() -> String {
    "eclipse-temurin:21-jre-alpine".into()
}
fn default_app_port() -> u16 {
    8080
}
fn default_build_tool() -> String {
    "maven".into()
}
fn default_gradle_dsl() -> String {
    "kotlin".into()
}

// ── Config loading / persistence ─────────────────────────────────────────────

impl ProjectConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Cannot read config file: {}", path.display()))?;
        let config: Self = toml::from_str(&content)
            .with_context(|| format!("Invalid TOML in: {}", path.display()))?;
        Ok(config)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;
        std::fs::write(path, content)
            .with_context(|| format!("Cannot write config file: {}", path.display()))?;
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        use anyhow::bail;

        if self.project.name.is_empty() {
            bail!("project.name cannot be empty");
        }
        if self.project.group.is_empty() {
            bail!("project.group cannot be empty");
        }
        if self.project.java_version < 11 {
            bail!("project.java_version must be >= 11");
        }

        // Validate build tool
        match self.project.build_tool.as_str() {
            "maven" | "gradle" => {}
            other => bail!(
                "project.build_tool must be 'maven' or 'gradle' (got '{}')",
                other
            ),
        }

        // Validate Gradle DSL
        match self.project.gradle_dsl.as_str() {
            "kotlin" | "groovy" => {}
            other => bail!(
                "project.gradle_dsl must be 'kotlin' or 'groovy' (got '{}')",
                other
            ),
        }

        // Validate Redis
        let has_redis = self.features.iter().any(|f| f.starts_with("redis"));
        if has_redis {
            match self.redis.mode.as_str() {
                "standalone" | "ssl" | "sentinel" | "cluster" => {}
                other => bail!(
                    "redis.mode must be one of: standalone, ssl, sentinel, cluster (got '{}')",
                    other
                ),
            }
        }

        // Check conflicting features
        crate::features::resolve_features(&self.features)?;

        Ok(())
    }

    /// Build a default config from new args
    pub fn from_new_args(args: &crate::cli::NewArgs) -> Self {
        use crate::cli::{BuildTool, GradleDsl};

        let redis_mode = if !args.redis_stack.is_empty() {
            args.redis_stack.join(",")
        } else {
            "standalone".to_string()
        };

        let build_tool = match &args.build_tool {
            BuildTool::Maven => "maven",
            BuildTool::Gradle => "gradle",
        }
        .to_string();

        let gradle_dsl = match &args.gradle_dsl {
            GradleDsl::Kotlin => "kotlin",
            GradleDsl::Groovy => "groovy",
        }
        .to_string();

        ProjectConfig {
            project: ProjectMeta {
                name: args.name.clone(),
                group: args.group.clone(),
                version: "0.0.1-SNAPSHOT".into(),
                description: format!("{} — generated by SpringGen", args.name),
                java_version: args.java_version,
                boot_version: args.boot_version.clone(),
                build_tool,
                gradle_dsl,
            },
            features: args.features.clone(),
            redis: RedisConfig {
                mode: redis_mode,
                host: "localhost".into(),
                port: 6379,
                ..Default::default()
            },
            kafka: KafkaConfig::default(),
            rabbitmq: RabbitMqConfig::default(),
            ibmmq: IbmMqConfig::default(),
            database: DatabaseConfig::default(),
            security: SecurityConfig::default(),
            docker: DockerConfig::default(),
            extra_properties: IndexMap::new(),
        }
    }
}
