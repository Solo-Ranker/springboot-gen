use anyhow::Result;
use handlebars::Handlebars;
use serde_json::{json, Map, Value};
use std::path::Path;

use crate::config::ProjectConfig;
use crate::features::{DockerService, FeatureSpec};

macro_rules! tpl {
    ($name:literal) => {
        (
            $name,
            include_str!(concat!("../../templates/docker/", $name, ".hbs")),
        )
    };
}

const TEMPLATES: &[(&str, &str)] = &[
    tpl!("Dockerfile"),
    tpl!("component-compose.yml"),
    tpl!(".dockerignore"),
];

pub struct DockerGenerator<'a> {
    config: &'a ProjectConfig,
    features: &'a [FeatureSpec],
    hb: Handlebars<'static>,
}

impl<'a> DockerGenerator<'a> {
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

    pub fn generate(&self, out: &Path) -> Result<()> {
        let artifact = crate::engine::to_artifact_id(&self.config.project.name);

        std::fs::write(
            out.join("Dockerfile"),
            self.hb
                .render("Dockerfile", &self.dockerfile_context(&artifact))?,
        )?;

        std::fs::write(
            out.join(".dockerignore"),
            self.hb.render(".dockerignore", &json!({}))?,
        )?;

        // Per-feature component compose files
        let docker_dir = out.join("docker");
        for feature in self.features {
            if !feature.docker_services.is_empty() {
                let feat_dir = docker_dir.join(feature.key);
                std::fs::create_dir_all(&feat_dir)?;
                let ctx = self.component_compose_context(feature);
                std::fs::write(
                    feat_dir.join("docker-compose.yml"),
                    self.hb.render("component-compose.yml", &ctx)?,
                )?;
            }
        }

        Ok(())
    }

    // ── Context builders ──────────────────────────────────────────────────────

    fn dockerfile_context(&self, artifact: &str) -> Value {
        json!({
            "artifact":     artifact,
            "version":      self.config.project.version,
            "java_version": self.config.project.java_version,
            "base_image":   self.config.docker.base_image,
            "port":         self.config.docker.app_port,
        })
    }

    fn component_compose_context(&self, feature: &FeatureSpec) -> Value {
        let is_db_repl = self.has("database-replication");
        let mut infra_services: Vec<Value> = Vec::new();
        let mut volumes = self.collect_volumes(feature.docker_services.iter());

        if is_db_repl && feature.key == "postgres" {
            volumes = vec![
                "postgres_master_data".to_string(),
                "postgres_slave_data".to_string(),
            ];
            infra_services.push(json!({
                "name": "postgresql-master",
                "image": "bitnami/postgresql:latest",
                "ports": ["5432:5432"],
                "environment": {
                    "POSTGRESQL_REPLICATION_MODE": "master",
                    "POSTGRESQL_USERNAME": "postgres",
                    "POSTGRESQL_PASSWORD": "supersecret",
                    "POSTGRESQL_DATABASE": "sample",
                    "POSTGRESQL_REPLICATION_USER": "repl_user",
                    "POSTGRESQL_REPLICATION_PASSWORD": "repl_password"
                },
                "volumes": ["postgres_master_data:/bitnami/postgresql"],
                "healthcheck": "pg_isready -U postgres",
                "networks": ["app-tier"]
            }));
            infra_services.push(json!({
                "name": "postgresql-slave",
                "image": "bitnami/postgresql:latest",
                "depends_on": ["postgresql-master"],
                "ports": ["5433:5432"],
                "environment": {
                    "POSTGRESQL_REPLICATION_MODE": "slave",
                    "POSTGRESQL_MASTER_HOST": "postgresql-master",
                    "POSTGRESQL_MASTER_PORT_NUMBER": "5432",
                    "POSTGRESQL_REPLICATION_USER": "repl_user",
                    "POSTGRESQL_REPLICATION_PASSWORD": "repl_password",
                    "POSTGRESQL_USERNAME": "postgres",
                    "POSTGRESQL_PASSWORD": "supersecret",
                    "POSTGRESQL_DATABASE": "sample"
                },
                "volumes": ["postgres_slave_data:/bitnami/postgresql"],
                "networks": ["app-tier"]
            }));
        } else if is_db_repl && feature.key == "mysql" {
            volumes = vec![
                "mysql_master_data".to_string(),
                "mysql_slave_data".to_string(),
            ];
            infra_services.push(json!({
                "name": "mysql-master",
                "image": "bitnamilegacy/mysql:8.0",
                "ports": ["3306:3306"],
                "environment": {
                    "MYSQL_REPLICATION_MODE": "master",
                    "MYSQL_REPLICATION_USER": "repl_user",
                    "MYSQL_REPLICATION_PASSWORD": "repl_password",
                    "MYSQL_ROOT_PASSWORD": "supersecret",
                    "MYSQL_DATABASE": "sample"
                },
                "volumes": ["mysql_master_data:/bitnami/mysql"],
                "healthcheck": "mysqladmin ping -h localhost -psupersecret",
                "networks": ["app-tier"]
            }));
            infra_services.push(json!({
                "name": "mysql-slave",
                "image": "bitnamilegacy/mysql:8.0",
                "depends_on": ["mysql-master"],
                "ports": ["3307:3306"],
                "environment": {
                    "MYSQL_REPLICATION_MODE": "slave",
                    "MYSQL_MASTER_HOST": "mysql-master",
                    "MYSQL_MASTER_PORT_NUMBER": "3306",
                    "MYSQL_MASTER_ROOT_PASSWORD": "supersecret",
                    "MYSQL_REPLICATION_USER": "repl_user",
                    "MYSQL_REPLICATION_PASSWORD": "repl_password",
                    "MYSQL_ROOT_PASSWORD": "supersecret",
                    "MYSQL_DATABASE": "sample"
                },
                "volumes": ["mysql_slave_data:/bitnami/mysql"],
                "networks": ["app-tier"]
            }));
        } else {
            infra_services = feature
                .docker_services
                .iter()
                .map(|s| self.service_value(s))
                .collect();
        }

        json!({
            "feature_name":   feature.name,
            "feature_key":    feature.key,
            "infra_services": infra_services,
            "volumes":        volumes,
        })
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn service_value(&self, svc: &DockerService) -> Value {
        let environment: Map<String, Value> = svc
            .environment
            .iter()
            .map(|(k, v)| (k.to_string(), json!(v)))
            .collect();

        json!({
            "name":        svc.name,
            "image":       svc.image,
            "ports":       svc.ports,
            "environment": environment,
            "volumes":     svc.volumes,
            "healthcheck": svc.healthcheck,
            "depends_on":  svc.depends_on,
        })
    }

    fn collect_volumes<'b, I>(&self, services: I) -> Vec<String>
    where
        I: Iterator<Item = &'b DockerService>,
    {
        let mut volumes: Vec<String> = Vec::new();
        for svc in services {
            for vol in svc.volumes {
                if let Some(name) = vol.split(':').next() {
                    if !name.starts_with('.') && !name.starts_with('/') {
                        let name = name.to_string();
                        if !volumes.contains(&name) {
                            volumes.push(name);
                        }
                    }
                }
            }
        }
        volumes
    }

    fn has(&self, key: &str) -> bool {
        self.features.iter().any(|f| f.key == key)
    }
}
