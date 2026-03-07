use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureSpec {
    pub key: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub maven_deps: Vec<MavenDep>,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub docker_services: Vec<DockerService>,
    #[serde(default)]
    pub env_vars: IndexMap<String, String>,
    #[serde(default)]
    pub java_files: IndexMap<String, String>,
    #[serde(default)]
    pub conflicts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MavenDep {
    pub group_id: String,
    pub artifact_id: String,
    pub version: Option<String>,
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerService {
    pub name: String,
    pub image: String,
    #[serde(default)]
    pub ports: Vec<String>,
    #[serde(default)]
    pub environment: IndexMap<String, String>,
    #[serde(default)]
    pub volumes: Vec<String>,
    pub healthcheck: Option<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
}

use include_dir::{include_dir, Dir};

static PLUGINS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/plugins");

// ── Registry ──────────────────────────────────────────────────────────────────

pub fn all_features() -> Vec<FeatureSpec> {
    let mut features = Vec::new();
    for file in PLUGINS_DIR.files() {
        if let Some(ext) = file.path().extension() {
            if ext == "toml" {
                if let Some(contents) = file.contents_utf8() {
                    match toml::from_str::<FeatureSpec>(contents) {
                        Ok(f) => features.push(f),
                        Err(e) => eprintln!("Warning: Failed to parse plugin {:?}: {}", file.path(), e),
                    }
                }
            }
        }
    }
    features
}

pub fn resolve_features(keys: &[String]) -> anyhow::Result<Vec<FeatureSpec>> {
    let all = all_features();
    let mut resolved_map: IndexMap<String, FeatureSpec> = IndexMap::new();
    let mut queue = keys.to_vec();

    while let Some(key) = queue.pop() {
        if resolved_map.contains_key(&key) {
            continue;
        }

        let spec = all
            .iter()
            .find(|f| f.key == key)
            .ok_or_else(|| {
                anyhow::anyhow!(
                "Unknown feature: '{}'. Run `springboot-gen features` to list available features.",
                key
            )
            })?
            .clone();

        for dep in &spec.requires {
            if !resolved_map.contains_key(dep) {
                queue.push(dep.to_string());
            }
        }

        resolved_map.insert(key.clone(), spec);
    }

    // Check conflicts
    for (key, spec) in &resolved_map {
        for conflict in &spec.conflicts {
            if resolved_map.contains_key(conflict) {
                anyhow::bail!(
                    "Feature conflict: '{}' and '{}' cannot be used together",
                    key,
                    conflict
                );
            }
        }
    }

    // Topological sort (parents first, dependencies last) so parents can overwrite dependency templates
    let mut sorted = Vec::new();
    let mut visited = std::collections::HashSet::new();

    fn visit(
        node: &str,
        map: &IndexMap<String, FeatureSpec>,
        visited: &mut std::collections::HashSet<String>,
        sorted: &mut Vec<FeatureSpec>,
    ) {
        if visited.contains(node) {
            return;
        }
        visited.insert(node.to_string());
        
        if let Some(spec) = map.get(node) {
            for dep in &spec.requires {
                visit(dep, map, visited, sorted);
            }
            sorted.push(spec.clone());
        }
    }

    for key in resolved_map.keys() {
        visit(key, &resolved_map, &mut visited, &mut sorted);
    }

    // Reverse to get parents first
    sorted.reverse();

    Ok(sorted)
}
