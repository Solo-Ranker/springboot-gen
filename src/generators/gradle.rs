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
            include_str!(concat!("../../templates/gradle/", $name, ".hbs")),
        )
    };
}

const TEMPLATES: &[(&str, &str)] = &[
    tpl!("build.gradle"),
    tpl!("build.gradle.kts"),
    tpl!("settings.gradle"),
    tpl!("settings.gradle.kts"),
    tpl!("gradle.properties"),
];

pub struct GradleGenerator<'a> {
    config: &'a ProjectConfig,
    features: &'a [FeatureSpec],
    hb: Handlebars<'static>,
}

impl<'a> GradleGenerator<'a> {
    pub fn new(config: &'a ProjectConfig, features: &'a [FeatureSpec]) -> Result<Self> {
        let mut hb = Handlebars::new();
        hb.set_strict_mode(false);
        // Prevent Handlebars from escaping characters in code templates
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
        self.generate_build_gradle(out)?;
        self.generate_settings_gradle(out)?;
        self.generate_gradle_properties(out)?;
        self.generate_gradle_wrapper(out)?;
        Ok(())
    }

    fn generate_build_gradle(&self, out: &Path) -> Result<()> {
        let is_kotlin_dsl = self.config.project.gradle_dsl == "kotlin";
        let (tpl_name, filename) = if is_kotlin_dsl {
            ("build.gradle.kts", "build.gradle.kts")
        } else {
            ("build.gradle", "build.gradle")
        };

        let data = json!({
            "project": {
                "boot_version":  self.config.project.boot_version,
                "group":         self.config.project.group,
                "version":       self.config.project.version,
                "java_version":  self.config.project.java_version,
            },
            "deps": self.collect_deps(is_kotlin_dsl),
        });

        let content = self.hb.render(tpl_name, &data)?;
        std::fs::write(out.join(filename), content)?;
        Ok(())
    }

    fn generate_settings_gradle(&self, out: &Path) -> Result<()> {
        let is_kotlin_dsl = self.config.project.gradle_dsl == "kotlin";
        let (tpl_name, filename) = if is_kotlin_dsl {
            ("settings.gradle.kts", "settings.gradle.kts")
        } else {
            ("settings.gradle", "settings.gradle")
        };

        let data = json!({ "project": { "name": self.config.project.name } });
        let content = self.hb.render(tpl_name, &data)?;
        std::fs::write(out.join(filename), content)?;
        Ok(())
    }

    fn generate_gradle_properties(&self, out: &Path) -> Result<()> {
        let content = self.hb.render("gradle.properties", &json!({}))?;
        std::fs::write(out.join("gradle.properties"), content)?;
        Ok(())
    }

    // ── Dependency collection ─────────────────────────────────────────────────

    fn collect_deps(&self, kotlin_dsl: bool) -> Vec<String> {
        // Core
        let mut deps: Vec<String> = vec![
            "// Core Dependencies".to_string(),
            self.dep(
                kotlin_dsl,
                "implementation",
                "org.springframework.boot",
                "spring-boot-starter-web",
                None,
            ),
            self.dep(
                kotlin_dsl,
                "implementation",
                "org.springframework.boot",
                "spring-boot-starter-validation",
                None,
            ),
            self.dep(
                kotlin_dsl,
                "compileOnly",
                "org.projectlombok",
                "lombok",
                None,
            ),
            self.dep(
                kotlin_dsl,
                "annotationProcessor",
                "org.projectlombok",
                "lombok",
                None,
            ),
            self.dep(
                kotlin_dsl,
                "testImplementation",
                "org.springframework.boot",
                "spring-boot-starter-test",
                None,
            ),
            self.dep(
                kotlin_dsl,
                "testImplementation",
                "org.testcontainers",
                "junit-jupiter",
                None,
            ),
        ];

        // Feature deps
        for feature in self.features {
            if feature.maven_deps.is_empty() {
                continue;
            }
            let mut feature_deps = Vec::new();
            for dep in feature.maven_deps {
                let scope = match dep.scope {
                    Some("test") => "testImplementation",
                    Some("provided") => "compileOnly",
                    Some("runtime") => "runtimeOnly",
                    _ => "implementation",
                };
                let d = self.dep(
                    kotlin_dsl,
                    scope,
                    dep.group_id,
                    dep.artifact_id,
                    dep.version,
                );
                if !deps.contains(&d) {
                    feature_deps.push(d);
                }
            }
            if !feature_deps.is_empty() {
                deps.push(String::new());
                deps.push(format!("// {} Dependencies", feature.name));
                deps.extend(feature_deps);
            }
        }

        // Feature-specific test deps
        let mut test_deps = Vec::new();
        if self.has("kafka") {
            test_deps.push(self.dep(
                kotlin_dsl,
                "testImplementation",
                "org.springframework.kafka",
                "spring-kafka-test",
                None,
            ));
        }
        if self.has("postgres") {
            test_deps.push(self.dep(
                kotlin_dsl,
                "testImplementation",
                "org.testcontainers",
                "postgresql",
                None,
            ));
        }
        if !test_deps.is_empty() {
            deps.push(String::new());
            deps.push("// Additional Test Dependencies".to_string());
            deps.extend(test_deps);
        }

        deps
    }

    fn dep(
        &self,
        kotlin_dsl: bool,
        scope: &str,
        group: &str,
        artifact: &str,
        version: Option<&str>,
    ) -> String {
        let coord = match version {
            Some(v) => format!("{}:{}:{}", group, artifact, v),
            None => format!("{}:{}", group, artifact),
        };
        if kotlin_dsl {
            format!("{}(\"{}\")", scope, coord)
        } else {
            format!("{} '{}'", scope, coord)
        }
    }

    fn has(&self, key: &str) -> bool {
        self.features.iter().any(|f| f.key == key)
    }

    fn generate_gradle_wrapper(&self, out: &Path) -> Result<()> {
        let wrapper_dir = out.join("gradle/wrapper");
        std::fs::create_dir_all(&wrapper_dir)?;

        std::fs::write(
            wrapper_dir.join("gradle-wrapper.properties"),
            include_str!("../../templates/gradle/gradle-wrapper.properties"),
        )?;

        std::fs::write(
            wrapper_dir.join("gradle-wrapper.jar"),
            include_bytes!("../../templates/gradle/gradle-wrapper.jar"),
        )?;

        std::fs::write(
            out.join("gradlew"),
            include_str!("../../templates/gradle/gradlew"),
        )?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(out.join("gradlew"))?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(out.join("gradlew"), perms)?;
        }

        std::fs::write(
            out.join("gradlew.bat"),
            include_str!("../../templates/gradle/gradlew.bat"),
        )?;

        Ok(())
    }
}
