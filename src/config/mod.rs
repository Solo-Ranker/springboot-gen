use anyhow::{Context, Result};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Top-level springboot-gen.toml configuration
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

    // ── Cluster-specific ────────────────────────────────
    #[serde(default = "default_cluster_nodes")]
    pub cluster_nodes: Vec<String>,

    // ── Connection pool ─────────────────────────────────
    #[serde(default = "default_pool_max_active")]
    pub pool_max_active: u8,

    #[serde(default = "default_pool_max_idle")]
    pub pool_max_idle: u8,

    #[serde(default)]
    pub pool_min_idle: u8,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            mode: default_redis_mode(),
            host: default_redis_host(),
            port: default_redis_port(),
            password: None,
            database: default_redis_db(),
            ssl: RedisSslConfig::default(),
            sentinel: RedisSentinelConfig::default(),
            cluster_nodes: default_cluster_nodes(),
            pool_max_active: default_pool_max_active(),
            pool_max_idle: default_pool_max_idle(),
            pool_min_idle: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Default for RedisSslConfig {
    fn default() -> Self {
        Self {
            keystore_location: String::new(),
            keystore_password: String::new(),
            truststore_location: String::new(),
            truststore_password: String::new(),
            verify_hostname: default_true(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisSentinelConfig {
    #[serde(default = "default_sentinel_master")]
    pub master: String,

    /// Comma-separated list of sentinel nodes (host:port)
    #[serde(default = "default_sentinel_nodes")]
    pub nodes: Vec<String>,

    #[serde(default)]
    pub sentinel_password: Option<String>,
}

impl Default for RedisSentinelConfig {
    fn default() -> Self {
        Self {
            master: default_sentinel_master(),
            nodes: default_sentinel_nodes(),
            sentinel_password: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Default for KafkaConfig {
    fn default() -> Self {
        Self {
            bootstrap_servers: default_kafka_bootstrap(),
            consumer_group_id: default_consumer_group(),
            auto_offset_reset: default_offset_reset(),
            listener_concurrency: default_listener_concurrency(),
            idempotent_producer: default_true(),
            sasl: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaSaslConfig {
    pub mechanism: String,
    pub username: String,
    pub password: String,
    pub security_protocol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Default for RabbitMqConfig {
    fn default() -> Self {
        Self {
            host: default_rabbitmq_host(),
            port: default_rabbitmq_port(),
            username: default_rabbitmq_user(),
            password: default_rabbitmq_password(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Default for IbmMqConfig {
    fn default() -> Self {
        Self {
            queue_manager: default_ibmmq_qm(),
            channel: default_ibmmq_channel(),
            host: default_ibmmq_host(),
            port: default_ibmmq_port(),
            username: default_ibmmq_user(),
            password: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// SSL configuration for the database connection
    #[serde(default)]
    pub ssl: DatabaseSslConfig,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            host: default_db_host(),
            port: default_db_port(),
            name: default_db_name(),
            username: default_db_user(),
            password: String::new(),
            pool_max_size: default_pool_max_db(),
            flyway_enabled: default_true(),
            ssl: DatabaseSslConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSslConfig {
    /// Enable SSL for the database connection
    #[serde(default)]
    pub enabled: bool,

    /// sslmode for PostgreSQL (disable | allow | prefer | require | verify-ca | verify-full)
    /// or sslMode for MySQL (DISABLED | PREFERRED | REQUIRED | VERIFY_CA | VERIFY_IDENTITY)
    #[serde(default = "default_db_ssl_mode")]
    pub mode: String,

    /// Path to the PKCS12 client keystore (or classpath: reference)
    #[serde(default)]
    pub keystore_location: String,

    #[serde(default = "default_ssl_password")]
    pub keystore_password: String,

    /// Path to the truststore file
    #[serde(default)]
    pub truststore_location: String,

    #[serde(default = "default_ssl_password")]
    pub truststore_password: String,

    /// Whether to verify the server certificate hostname
    #[serde(default = "default_true")]
    pub verify_server_cert: bool,
}

impl Default for DatabaseSslConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: default_db_ssl_mode(),
            keystore_location: String::new(),
            keystore_password: default_ssl_password(),
            truststore_location: String::new(),
            truststore_password: default_ssl_password(),
            verify_server_cert: default_true(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            jwt_secret: None,
            access_token_expiry_ms: default_access_token_ms(),
            refresh_token_expiry_ms: default_refresh_token_ms(),
            oauth2_issuer_uri: None,
            cors_allowed_origins: Vec::new(),
            csrf_disabled: default_true(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerConfig {
    #[serde(default = "default_registry")]
    pub registry: String,

    #[serde(default = "default_base_image")]
    pub base_image: String,

    #[serde(default = "default_app_port")]
    pub app_port: u16,
}

impl Default for DockerConfig {
    fn default() -> Self {
        Self {
            registry: default_registry(),
            base_image: default_base_image(),
            app_port: default_app_port(),
        }
    }
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
fn default_cluster_nodes() -> Vec<String> {
    vec![
        "localhost:7000".into(),
        "localhost:7001".into(),
        "localhost:7002".into(),
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
fn default_db_ssl_mode() -> String {
    "prefer".into()
}
fn default_ssl_password() -> String {
    "changeit".into()
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
                "standalone" | "ssl" | "sentinel" | "ssl-sentinel" | "cluster" => {}
                other => bail!(
                    "redis.mode must be one of: standalone, ssl, sentinel, ssl-sentinel, cluster (got '{}')",
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
                description: format!("{} — generated by SpringbootGen", args.name),
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
