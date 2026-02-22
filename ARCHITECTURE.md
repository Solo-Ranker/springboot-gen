# Architecture & Design Guide

This document explains the internal structure of `SpringbootGen` and the reasoning behind its architectural decisions. It is intended for contributors who want to understand how the tool works under the hood.

## 1. System Overview

SpringbootGen is a **CLI tool written in Rust** that generates boilerplate Spring Boot applications. Unlike simple archetypes, it uses a **compositional engine** to dynamically assemble a project based on requested features (e.g., Redis, Kafka, Security).

### Why Rust?
- **Single Binary**: Easy distribution (`curl | bash` style) without requiring a JVM on the host machine to *generate* the project.
- **Speed**: Instant startup and generation, critical for CLI UX.
- **Type Safety**: The feature dependency graph and configuration are strictly typed, preventing invalid combinations at compile/runtime.

## 2. Project Structure

The source code is organized by **domain responsibility**, not just technical layer.

```
src/
├── main.rs            # Entry point (thin wrapper around `cli`)
├── cli/               # Command-line interface definition
│   └── mod.rs         # Uses `clap` for parsing args (new, init, add)
├── config/            # Configuration structs
│   └── mod.rs         # `ProjectConfig` struct (maps to springboot-gen.toml)
├── features/          # The "Brain" of the system
│   ├── mod.rs         # Feature resolution logic
│   └── registry.rs    # THE BIG FILE: Defines every available feature
├── engine/            # Orchestration
│   └── mod.rs         # `GenerationEngine`: coordinates all generators
├── generators/        # The "Workers"
│   ├── docker.rs      # Generates `docker-compose.yml` & `Dockerfile`
│   ├── java_code.rs   # Generates Java classes (Spring configs, etc.)
│   ├── pom.rs         # Generates `pom.xml`
│   ├── spring_properties.rs # Generates `application.yml`
│   └── ...            # specific generators for k8s, env files, etc.
└── analyzer/          # Analyzing existing projects
    └── mod.rs         # Heuristics to detect features in legacy code
```

## 3. Core Concepts

### 3.1. The Feature Registry (`src/features/registry.rs`)

This is the most important file in the codebase. It uses a **Declarative** approach to define what a "Feature" is.

Instead of writing `if (redis) { addDep(); addConfig(); }` scattered everywhere, we define a feature statically:

```rust
FeatureSpec {
    key: "redis",
    maven_deps: &[ /* ... */ ],
    properties: &[ ("spring.data.redis.host", "${REDIS_HOST}") ],
    docker_services: &[ /* ... */ ],
    java_files: &["RedisConfig"], // references template names
    requires: &[],
    conflicts: &["redis-ssl"],
}
```

**Why this design?**
- **Centralized Logic**: Adding a new feature usually only requires editing this one file.
- **Composition**: The engine can genericallly "sum up" all dependencies and Docker services from selected features.
- **Validation**: Conflicts and dependencies (e.g., `jwt` requires `security`) are resolved automatically by the engine before generation starts.

### 3.2. Generation Engine (`src/engine/mod.rs`)

The `GenerationEngine` is the conductor. Its job is:
1.  **Resolve**: Expand the user's requested features (e.g., `jwt` -> adds `security`).
2.  **Verify**: Check for conflicts (e.g., `jwt` vs `oauth2`).
3.  **Fan-out**: Pass the *final* list of properties, dependencies, and services to each specific `Generator`.

### 3.3. Generators (`src/generators/`)

Each generator focuses on one output type. They all share the same interface pattern of `new(config, features) -> Generator`.

- **`DockerGenerator`**: Iterates through all features, collects their `docker_services` structs, and renders a single `docker-compose.yml`.
- **`PomGenerator`**: Collects all `maven_deps` and renders the distinct list into `pom.xml`.
- **`JavaCodeGenerator`**: This is more complex (see below).

## 4. Key Implementation Details

### 4.1. Java Code Generation (`src/generators/java_code.rs`)

We use a **Hybrid Approach** for generating Java code:

1.  **Handlebars Templates**: Complex classes (like `Application.java`, `GlobalExceptionHandler.java`) are stored as `.hbs` files in `templates/java/`. These are compiled into the binary or loaded at runtime.
    *   *Why?* Separates complex Java syntax from Rust code.
2.  **Raw Strings (Legacy)**: Some simpler configs currently use `format!` macros directly in Rust.
    *   *Direction*: We are actively migrating these to Handlebars templates to improve maintainability.

### 4.2. File Output Strategy

- **Atomic Writes**: Generators prepare content in memory and write to disk.
- **Idempotency**: `springboot-gen` tries to be safe. It won't overwrite an existing folder unless `--force` is used.
- **`springboot-gen.toml`**: This file persists the configuration, allowing `springboot-gen add <feature>` to know what was previously installed (to handle merges perfectly).

## 5. Adding a New Feature

To add a new feature (e.g., "RabbitMQ"):

1.  **Open `src/features/registry.rs`**.
2.  **Add `FeatureSpec`**:
    *   Define `maven_deps` (Spring Boot starts, etc.).
    *   Define `properties` (application.yml keys).
    *   Define `docker_services` (the RabbitMQ image, ports, env vars).
    *   List `java_files` needed (e.g., `RabbitMqConfig`).
3.  **Add Java Template**:
    *   Create `templates/java/RabbitMqConfig.java.hbs`.
    *   Register it in `src/generators/java_code.rs`.
    *   Add logic in `JavaCodeGenerator::generate` to render it when the feature is present.

## 6. How to Contribute

- **Code Style**: Run `cargo fmt` before committing.
- **Testing**:
    - We currently lack extensive unit tests for the generators.
    - **Recommended Test Workflow**:
        1.  Run `cargo run -- new test-app --features <your-feature>`
        2.  Inspect the generated `test-app/` folder.
        3.  Run `mvn spring-boot:run` in that folder to verify it actually works.
