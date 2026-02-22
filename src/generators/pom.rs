use anyhow::Result;
use handlebars::Handlebars;
use serde_json::{json, Value};
use std::path::Path;

use crate::config::ProjectConfig;
use crate::features::FeatureSpec;

pub struct PomGenerator<'a> {
    config: &'a ProjectConfig,
    features: &'a [FeatureSpec],
    hb: Handlebars<'static>,
}

impl<'a> PomGenerator<'a> {
    pub fn new(config: &'a ProjectConfig, features: &'a [FeatureSpec]) -> Result<Self> {
        let mut hb = Handlebars::new();
        hb.set_strict_mode(false);
        hb.register_escape_fn(handlebars::no_escape);
        hb.register_template_string("pom.xml", include_str!("../../templates/pom/pom.xml.hbs"))?;
        Ok(Self {
            config,
            features,
            hb,
        })
    }

    pub fn generate(&self, out: &Path) -> Result<()> {
        let content = self.hb.render("pom.xml", &self.build_context())?;
        std::fs::write(out.join("pom.xml"), content)?;
        Ok(())
    }

    fn build_context(&self) -> Value {
        let meta = &self.config.project;
        let artifact = crate::engine::to_artifact_id(&meta.name);

        json!({
            "project": {
                "boot_version": meta.boot_version,
                "group":        meta.group,
                "artifact":     artifact,
                "version":      meta.version,
                "name":         meta.name,
                "description":  meta.description,
                "java_version": meta.java_version,
            },
            "deps": self.collect_deps(),
        })
    }

    fn collect_deps(&self) -> Vec<Value> {
        let mut deps: Vec<Value> = Vec::new();
        let mut seen: Vec<(&str, &str)> = Vec::new();

        let mut push =
            |group_id: &str, artifact_id: &str, version: Option<&str>, scope: Option<&str>| {
                deps.push(json!({
                    "group_id":    group_id,
                    "artifact_id": artifact_id,
                    "version":     version,
                    "scope":       scope,
                }));
            };

        // Core
        push(
            "org.springframework.boot",
            "spring-boot-starter-web",
            None,
            None,
        );
        push(
            "org.springframework.boot",
            "spring-boot-starter-validation",
            None,
            None,
        );
        push("org.projectlombok", "lombok", None, Some("provided"));
        push(
            "org.springframework.boot",
            "spring-boot-starter-test",
            None,
            Some("test"),
        );

        // Feature deps
        for feature in self.features {
            for dep in feature.maven_deps {
                let key = (dep.group_id, dep.artifact_id);
                if !seen.contains(&key) {
                    seen.push(key);
                    push(dep.group_id, dep.artifact_id, dep.version, dep.scope);
                }
            }
        }

        // Test deps
        push("org.testcontainers", "junit-jupiter", None, Some("test"));

        if self.has("kafka") {
            push(
                "org.springframework.kafka",
                "spring-kafka-test",
                None,
                Some("test"),
            );
        }
        if self.has("postgres") {
            push("org.testcontainers", "postgresql", None, Some("test"));
        }

        deps
    }

    fn has(&self, key: &str) -> bool {
        self.features.iter().any(|f| f.key == key)
    }
}
