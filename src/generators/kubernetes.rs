use anyhow::Result;
use handlebars::Handlebars;
use serde_json::{json, Value};
use std::path::Path;

use crate::config::ProjectConfig;
use crate::features::FeatureSpec;

macro_rules! tpl {
    ($name:literal) => {
        (
            $name,
            include_str!(concat!("../../templates/kubernetes/", $name, ".hbs")),
        )
    };
}

const TEMPLATES: &[(&str, &str)] = &[
    tpl!("namespace.yaml"),
    tpl!("configmap.yaml"),
    tpl!("secret.yaml"),
    tpl!("deployment.yaml"),
    tpl!("service.yaml"),
    tpl!("hpa.yaml"),
    tpl!("ingress.yaml"),
    tpl!("kustomization.yaml"),
];

pub struct KubernetesGenerator<'a> {
    config: &'a ProjectConfig,
    features: &'a [FeatureSpec],
    hb: Handlebars<'static>,
}

impl<'a> KubernetesGenerator<'a> {
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
        let k8s_dir = out.join("k8s");
        std::fs::create_dir_all(&k8s_dir)?;

        let ctx = self.build_context();

        for (name, _) in TEMPLATES {
            std::fs::write(k8s_dir.join(name), self.hb.render(name, &ctx)?)?;
        }

        Ok(())
    }

    fn build_context(&self) -> Value {
        let artifact = crate::engine::to_artifact_id(&self.config.project.name);
        let port = self.config.docker.app_port;

        json!({
            "artifact":          artifact,
            "port":              port,
            "has_redis":         self.has("redis") || self.has("redis-ssl"),
            "has_redis_sentinel": self.has("redis-sentinel"),
            "has_kafka":         self.has("kafka"),
            "has_postgres":      self.has("postgres"),
            "has_tracing":       self.has("tracing"),
        })
    }

    fn has(&self, key: &str) -> bool {
        self.features.iter().any(|f| f.key == key)
    }
}
