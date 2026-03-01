use anyhow::Result;
use handlebars::Handlebars;
use serde_json::json;
use std::path::Path;

use crate::config::ProjectConfig;
use crate::features::FeatureSpec;

macro_rules! tpl {
    ($name:literal) => {
        (
            $name,
            include_str!(concat!("../../templates/properties/", $name, ".hbs")),
        )
    };
}

const TEMPLATES: &[(&str, &str)] = &[
    tpl!("application-base"),
    tpl!("application-dev"),
    tpl!("application-prod"),
];

pub struct PropertiesGenerator<'a> {
    config: &'a ProjectConfig,
    features: &'a [FeatureSpec],
    hb: Handlebars<'static>,
}

impl<'a> PropertiesGenerator<'a> {
    pub fn new(config: &'a ProjectConfig, features: &'a [FeatureSpec]) -> Result<Self> {
        let mut hb = Handlebars::new();
        hb.set_strict_mode(false);
        for (name, src) in TEMPLATES {
            hb.register_template_string(name, src)?;
        }
        Ok(Self {
            config,
            features,
            hb,
        })
    }

    pub fn generate(&self, out: &Path) -> Result<()> {
        let resources = out.join("src/main/resources");
        std::fs::create_dir_all(&resources)?;

        std::fs::write(resources.join("application.yml"), self.render_base()?)?;
        std::fs::write(resources.join("application-dev.yml"), self.render_dev()?)?;
        std::fs::write(resources.join("application-prod.yml"), self.render_prod()?)?;
        Ok(())
    }

    // ── Base: build one merged Value tree, serialize once ────────────────────

    fn render_base(&self) -> Result<String> {
        let r = &self.config.redis;
        let d = &self.config.database;
        let k = &self.config.kafka;
        let sec = &self.config.security;

        let has_redis = self.has("redis")
            || self.has("redis-ssl")
            || self.has("redis-sentinel")
            || self.has("redis-ssl-sentinel")
            || self.has("redis-cluster");

        let has_db_standalone = self.has("database-standalone");

        let db_is_mysql = self.has("mysql");

        let has_database_replication = self.has("database-replication");

        let data = json!({
            "artifact": crate::engine::to_artifact_id(&self.config.project.name),
            "project": { "name": &self.config.project.name },

            "has_redis":          has_redis,
            "redis": {
                "host":            r.host,
                "port":            r.port,
                "pool_max_active": r.pool_max_active,
                "pool_max_idle":   r.pool_max_idle,
                "pool_min_idle":   r.pool_min_idle,
                "sentinel": {
                    "master": r.sentinel.master,
                    "nodes":  r.sentinel.nodes.join(","),
                },
                "cluster": {
                    "nodes": r.cluster_nodes.join(","),
                }
            },

            "has_kafka": self.has("kafka"),
            "kafka": {
                "bootstrap_servers":    k.bootstrap_servers,
                "consumer_group_id":    k.consumer_group_id,
                "auto_offset_reset":    k.auto_offset_reset,
                "listener_concurrency": k.listener_concurrency,
            },

            "has_db_standalone":  has_db_standalone,
            "db_is_mysql":        db_is_mysql,
            "has_db_replication": has_database_replication,

            "db": {
                "type":          if db_is_mysql { "mysql" } else { "postgres" },
                "host":          d.host,
                "port":          if db_is_mysql { 3306 } else { 5432 },
                "replica_port":  if db_is_mysql { 3307 } else { 5433 },
                "password":      "supersecret",
                "name":          d.name,
                "username":      if db_is_mysql { "root" } else { "postgres" },
                "pool_max_size": d.pool_max_size,
                "flyway_enabled": d.flyway_enabled,
                "ssl": {
                    "mode":                 d.ssl.mode,
                    "keystore_location":    d.ssl.keystore_location,
                    "keystore_password":    d.ssl.keystore_password,
                    "truststore_location":  d.ssl.truststore_location,
                    "truststore_password":  d.ssl.truststore_password,
                }
            },

            "has_mongodb":       self.has("mongodb"),
            "has_elasticsearch": self.has("elasticsearch"),

            "has_jwt":    self.has("jwt"),
            "has_oauth2": self.has("oauth2"),
            "security": {
                "access_token_expiry_ms":  sec.access_token_expiry_ms,
                "refresh_token_expiry_ms": sec.refresh_token_expiry_ms,
                "oauth2_issuer_uri": sec.oauth2_issuer_uri
                    .as_deref()
                    .unwrap_or("http://localhost:8180/realms/app"),
            },

            "has_openapi":   self.has("openapi"),
            "has_actuator":  self.has("actuator"),
            "has_tracing":   self.has("tracing"),
            "has_email":     self.has("email"),
            "has_s3":        self.has("s3"),
            "has_websocket": self.has("websocket"),
        });

        Ok(self.hb.render("application-base", &data)?)
    }

    fn render_dev(&self) -> Result<String> {
        Ok(self.hb.render(
            "application-dev",
            &json!({ "has_openapi": self.has("openapi")}),
        )?)
    }

    fn render_prod(&self) -> Result<String> {
        Ok(self.hb.render("application-prod", &json!({}))?)
    }

    fn has(&self, key: &str) -> bool {
        self.features.iter().any(|f| f.key == key)
    }
}
