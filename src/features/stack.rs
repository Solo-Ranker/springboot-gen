use serde::Serialize;

use super::registry::{DockerService, MavenDep};

/// Represents a feature with composable stack options
/// Example: Redis can have SSL, Sentinel, or Cluster options
#[derive(Debug, Clone, Serialize)]
pub struct FeatureStack {
    /// Base feature key (e.g., "redis", "kafka", "postgres")
    pub key: &'static str,

    /// Human-readable name
    pub name: &'static str,

    /// Description of the base feature
    pub description: &'static str,

    /// Base dependencies required for this feature (always included)
    pub base_maven_deps: &'static [MavenDep],

    /// Base properties for application.yml (always included)
    pub base_properties: &'static [(&'static str, &'static str)],

    /// Base Docker services (always included)
    pub base_docker_services: &'static [DockerService],

    /// Base environment variables (always included)
    pub base_env_vars: &'static [(&'static str, &'static str)],

    /// Base Java files to generate (always included)
    pub base_java_files: &'static [&'static str],

    /// Available stack options for this feature
    pub options: &'static [StackOption],

    /// Features this base feature requires
    pub requires: &'static [&'static str],

    /// Features this base feature conflicts with
    pub conflicts: &'static [&'static str],
}

/// A composable option that can be added to a base feature
/// Example: SSL option for Redis, SASL option for Kafka
#[derive(Debug, Clone, Serialize)]
pub struct StackOption {
    /// Option key (e.g., "ssl", "sentinel", "cluster")
    pub key: &'static str,

    /// Human-readable name
    pub name: &'static str,

    /// Description of this option
    pub description: &'static str,

    /// Additional Maven dependencies for this option
    pub maven_deps: &'static [MavenDep],

    /// Additional properties for this option
    pub properties: &'static [(&'static str, &'static str)],

    /// Additional Docker services for this option
    pub docker_services: &'static [DockerService],

    /// Additional environment variables for this option
    pub env_vars: &'static [(&'static str, &'static str)],

    /// Additional Java files to generate for this option
    pub java_files: &'static [&'static str],

    /// Options this option conflicts with (within the same feature)
    pub conflicts: &'static [&'static str],

    /// Options this option requires (within the same feature)
    pub requires: &'static [&'static str],
}

/// Resolved feature with selected stack options
#[derive(Debug, Clone)]
pub struct ResolvedFeature {
    pub key: String,
    pub selected_options: Vec<String>,
    pub maven_deps: Vec<MavenDep>,
    pub properties: Vec<(String, String)>,
    pub docker_services: Vec<DockerService>,
    pub env_vars: Vec<(String, String)>,
    pub java_files: Vec<String>,
}

impl FeatureStack {
    /// Resolve this feature with the given stack options
    pub fn resolve(&self, selected_options: &[String]) -> Result<ResolvedFeature, String> {
        // Validate options exist
        for opt_key in selected_options {
            if !self.options.iter().any(|o| o.key == opt_key) {
                return Err(format!(
                    "Unknown stack option '{}' for feature '{}'",
                    opt_key, self.key
                ));
            }
        }

        // Check for conflicts between options
        for opt_key in selected_options {
            let option = self.options.iter().find(|o| o.key == opt_key).unwrap();
            for conflict in option.conflicts {
                if selected_options.iter().any(|o| o == conflict) {
                    return Err(format!(
                        "Stack options '{}' and '{}' conflict in feature '{}'",
                        opt_key, conflict, self.key
                    ));
                }
            }
        }

        // Check for required options
        for opt_key in selected_options {
            let option = self.options.iter().find(|o| o.key == opt_key).unwrap();
            for required in option.requires {
                if !selected_options.iter().any(|o| o == required) {
                    return Err(format!(
                        "Stack option '{}' requires '{}' in feature '{}'",
                        opt_key, required, self.key
                    ));
                }
            }
        }

        // Combine base + selected options
        let mut resolved = ResolvedFeature {
            key: self.key.to_string(),
            selected_options: selected_options.to_vec(),
            maven_deps: self.base_maven_deps.to_vec(),
            properties: self
                .base_properties
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            docker_services: self.base_docker_services.to_vec(),
            env_vars: self
                .base_env_vars
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            java_files: self.base_java_files.iter().map(|s| s.to_string()).collect(),
        };

        // Add each selected option's contributions
        for opt_key in selected_options {
            let option = self.options.iter().find(|o| o.key == opt_key).unwrap();

            resolved.maven_deps.extend_from_slice(option.maven_deps);
            resolved.properties.extend(
                option
                    .properties
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string())),
            );
            resolved
                .docker_services
                .extend_from_slice(option.docker_services);
            resolved.env_vars.extend(
                option
                    .env_vars
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string())),
            );
            resolved
                .java_files
                .extend(option.java_files.iter().map(|s| s.to_string()));
        }

        Ok(resolved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_base_only() {
        let stack = FeatureStack {
            key: "redis",
            name: "Redis",
            description: "Redis cache",
            base_maven_deps: &[],
            base_properties: &[("redis.host", "localhost")],
            base_docker_services: &[],
            base_env_vars: &[],
            base_java_files: &["RedisConfig"],
            options: &[],
            requires: &[],
            conflicts: &[],
        };

        let resolved = stack.resolve(&[]).unwrap();
        assert_eq!(resolved.key, "redis");
        assert_eq!(resolved.properties.len(), 1);
        assert_eq!(resolved.java_files.len(), 1);
    }

    #[test]
    fn test_resolve_with_option() {
        let stack = FeatureStack {
            key: "redis",
            name: "Redis",
            description: "Redis cache",
            base_maven_deps: &[],
            base_properties: &[("redis.host", "localhost")],
            base_docker_services: &[],
            base_env_vars: &[],
            base_java_files: &["RedisConfig"],
            options: &[StackOption {
                key: "ssl",
                name: "SSL/TLS",
                description: "Enable SSL",
                maven_deps: &[],
                properties: &[("redis.ssl.enabled", "true")],
                docker_services: &[],
                env_vars: &[],
                java_files: &["RedisSslConfig"],
                conflicts: &[],
                requires: &[],
            }],
            requires: &[],
            conflicts: &[],
        };

        let resolved = stack.resolve(&["ssl".to_string()]).unwrap();
        assert_eq!(resolved.properties.len(), 2);
        assert_eq!(resolved.java_files.len(), 2);
    }

    #[test]
    fn test_conflict_detection() {
        let stack = FeatureStack {
            key: "redis",
            name: "Redis",
            description: "Redis cache",
            base_maven_deps: &[],
            base_properties: &[],
            base_docker_services: &[],
            base_env_vars: &[],
            base_java_files: &[],
            options: &[
                StackOption {
                    key: "ssl",
                    name: "SSL",
                    description: "SSL",
                    maven_deps: &[],
                    properties: &[],
                    docker_services: &[],
                    env_vars: &[],
                    java_files: &[],
                    conflicts: &["sentinel"],
                    requires: &[],
                },
                StackOption {
                    key: "sentinel",
                    name: "Sentinel",
                    description: "Sentinel",
                    maven_deps: &[],
                    properties: &[],
                    docker_services: &[],
                    env_vars: &[],
                    java_files: &[],
                    conflicts: &[],
                    requires: &[],
                },
            ],
            requires: &[],
            conflicts: &[],
        };

        let result = stack.resolve(&["ssl".to_string(), "sentinel".to_string()]);
        assert!(result.is_err());
    }
}
