use anyhow::Result;
use handlebars::Handlebars;
use indexmap::IndexMap;
use serde_json::{json, Map, Value};
use std::path::Path;

use crate::config::ProjectConfig;
use crate::features::FeatureSpec;

// Keys that belong to each named section, in order
const SECTIONS: &[(&str, &[&str])] = &[
    ("application", &["SPRING_PROFILES_ACTIVE", "SERVER_PORT"]),
    (
        "database",
        &[
            "DB_HOST",
            "DB_PORT",
            "DB_NAME",
            "DB_USER",
            "DB_PASSWORD",
            "DB_POOL_MAX",
        ],
    ),
    (
        "redis",
        &[
            "REDIS_HOST",
            "REDIS_PORT",
            "REDIS_PASSWORD",
            "REDIS_DB",
            "REDIS_SENTINEL_MASTER",
            "REDIS_SENTINEL_NODES",
            "REDIS_SENTINEL_PASSWORD",
            "REDIS_SSL_KEYSTORE",
            "REDIS_SSL_KEYSTORE_PASSWORD",
            "REDIS_SSL_TRUSTSTORE",
            "REDIS_SSL_TRUSTSTORE_PASSWORD",
        ],
    ),
    (
        "kafka",
        &["KAFKA_BOOTSTRAP_SERVERS", "KAFKA_LISTENER_CONCURRENCY"],
    ),
    (
        "mongodb",
        &[
            "MONGO_HOST",
            "MONGO_PORT",
            "MONGO_DB",
            "MONGO_USER",
            "MONGO_PASSWORD",
        ],
    ),
    (
        "security",
        &[
            "JWT_SECRET",
            "JWT_ACCESS_EXPIRY_MS",
            "JWT_REFRESH_EXPIRY_MS",
            "SECURITY_USER",
            "SECURITY_PASSWORD",
            "OAUTH2_ISSUER_URI",
            "OAUTH2_JWK_URI",
        ],
    ),
    (
        "s3",
        &[
            "S3_BUCKET",
            "AWS_REGION",
            "S3_ENDPOINT",
            "S3_PATH_STYLE",
            "AWS_ACCESS_KEY_ID",
            "AWS_SECRET_ACCESS_KEY",
        ],
    ),
    (
        "mail",
        &[
            "MAIL_HOST",
            "MAIL_PORT",
            "MAIL_USER",
            "MAIL_PASSWORD",
            "MAIL_FROM",
        ],
    ),
    ("tracing", &["ZIPKIN_ENDPOINT", "TRACING_SAMPLE_RATE"]),
    (
        "elasticsearch",
        &[
            "ELASTICSEARCH_URIS",
            "ELASTICSEARCH_USER",
            "ELASTICSEARCH_PASSWORD",
        ],
    ),
    ("websocket", &["WS_ALLOWED_ORIGINS"]),
];

pub struct EnvFileGenerator<'a> {
    config: &'a ProjectConfig,
    features: &'a [FeatureSpec],
    hb: Handlebars<'static>,
}

impl<'a> EnvFileGenerator<'a> {
    pub fn new(config: &'a ProjectConfig, features: &'a [FeatureSpec]) -> Result<Self> {
        let mut hb = Handlebars::new();
        hb.set_strict_mode(false);
        hb.register_escape_fn(handlebars::no_escape);
        hb.register_template_string("env", include_str!("../../templates/env/.env.hbs"))?;
        Ok(Self {
            config,
            features,
            hb,
        })
    }

    pub fn generate(&self, out: &Path) -> Result<()> {
        let content = self.hb.render("env", &self.build_context())?;
        std::fs::write(out.join(".env.example"), &content)?;
        if !out.join(".env").exists() {
            std::fs::write(out.join(".env"), &content)?;
        }
        Ok(())
    }

    fn build_context(&self) -> Value {
        // Collect all vars (deduped, ordered)
        let mut vars: IndexMap<String, String> = IndexMap::new();
        vars.insert("SPRING_PROFILES_ACTIVE".into(), "dev".into());
        vars.insert("SERVER_PORT".into(), "8080".into());
        for feature in self.features {
            for (key, val) in &feature.env_vars {
                vars.entry(key.to_string()).or_insert(val.to_string());
            }
        }

        // Distribute vars into sections
        let mut sections: Map<String, Value> = Map::new();
        let mut written: Vec<&str> = Vec::new();

        for (section_key, keys) in SECTIONS {
            let mut section_map: Map<String, Value> = Map::new();
            for key in *keys {
                if let Some(val) = vars.get(*key) {
                    section_map.insert((*key).to_string(), json!(val));
                    written.push(key);
                }
            }
            if !section_map.is_empty() {
                sections.insert((*section_key).to_string(), Value::Object(section_map));
            }
        }

        // Anything not in a known section goes to "other"
        let mut other: Map<String, Value> = Map::new();
        for (key, val) in &vars {
            if !written.contains(&key.as_str()) {
                other.insert(key.clone(), json!(val));
            }
        }
        if !other.is_empty() {
            sections.insert("other".to_string(), Value::Object(other));
        }

        json!({
            "project_name": self.config.project.name,
            "sections":     sections,
        })
    }
}
