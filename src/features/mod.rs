pub mod registry;
pub mod stack;

pub use registry::{
    all_features, find_feature, resolve_features, DockerService, FeatureSpec, MavenDep,
};
pub use stack::{FeatureStack, ResolvedFeature, StackOption};
