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
    tpl!("docker-compose.yml"),
    tpl!("docker-compose.override.yml"),
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
            out.join("docker-compose.yml"),
            self.hb
                .render("docker-compose.yml", &self.compose_context(&artifact))?,
        )?;

        std::fs::write(
            out.join("docker-compose.override.yml"),
            self.hb.render(
                "docker-compose.override.yml",
                &json!({ "artifact": artifact }),
            )?,
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

    fn compose_context(&self, artifact: &str) -> Value {
        let port = self.config.docker.app_port;

        let infra_services: Vec<Value> = self
            .features
            .iter()
            .flat_map(|f| f.docker_services.iter().map(|s| self.service_value(s)))
            .collect();

        let app_depends_on: Vec<&str> = self
            .features
            .iter()
            .flat_map(|f| f.docker_services.iter().map(|s| s.name))
            .collect();

        let app_env: Vec<String> = self
            .features
            .iter()
            .flat_map(|f| f.env_vars.iter().map(|(k, _)| format!("{k}: ${{{k}}}")))
            .collect();

        let volumes =
            self.collect_volumes(self.features.iter().flat_map(|f| f.docker_services.iter()));

        json!({
            "artifact":       artifact,
            "port":           port,
            "infra_services": infra_services,
            "app_depends_on": app_depends_on,
            "app_env":        app_env,
            "volumes":        volumes,
        })
    }

    fn component_compose_context(&self, feature: &FeatureSpec) -> Value {
        let infra_services: Vec<Value> = feature
            .docker_services
            .iter()
            .map(|s| self.service_value(s))
            .collect();

        let volumes = self.collect_volumes(feature.docker_services.iter());

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
}
