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
            for dep in &feature.maven_deps {
                let key = (dep.group_id.as_str(), dep.artifact_id.as_str());
                if !seen.contains(&key) {
                    seen.push(key);
                    push(&dep.group_id, &dep.artifact_id, dep.version.as_deref(), dep.scope.as_deref());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::NewArgs;
    use crate::config::ProjectConfig;
    use crate::features::resolve_features;

    fn get_test_config() -> ProjectConfig {
        let args = NewArgs {
            name: "test-svc".to_string(),
            group: "com.test".to_string(),
            boot_version: "3.2.5".to_string(),
            java_version: 21,
            features: vec![],
            output: None,
            force: false,
            emit_config: false,
            build_tool: crate::cli::BuildTool::Maven,
            gradle_dsl: crate::cli::GradleDsl::Kotlin,
            skip_format: false,
            kafka_stack: vec![],
        };
        ProjectConfig::from_new_args(&args)
    }

    #[test]
    fn test_pom_context_no_features() {
        let config = get_test_config();
        let features = vec![];
        let generator = PomGenerator::new(&config, &features).unwrap();
        
        let ctx = generator.build_context();
        assert_eq!(ctx["project"]["artifact"], "test_svc");
        
        let deps = ctx["deps"].as_array().unwrap();
        // Should have web, validation, lombok(provided), test, junit-jupiter(test)
        assert_eq!(deps.len(), 5);
        
        let web_dep = deps.iter().find(|d| d["artifact_id"] == "spring-boot-starter-web");
        assert!(web_dep.is_some());
    }

    #[test]
    fn test_pom_context_with_features() {
        let config = get_test_config();
        
        // Let's resolve 'redis' which pulls in its dependencies
        let features = resolve_features(&["redis".to_string()]).unwrap();
        
        let generator = PomGenerator::new(&config, &features).unwrap();
        
        let ctx = generator.build_context();
        let deps = ctx["deps"].as_array().unwrap();
        
        // web(1), val(1), lombok(1), test(1), junit(1) + redis(3) = 8
        assert_eq!(deps.len(), 8);
        
        let redis_dep = deps.iter().find(|d| d["artifact_id"] == "spring-boot-starter-data-redis");
        assert!(redis_dep.is_some());
    }
}
