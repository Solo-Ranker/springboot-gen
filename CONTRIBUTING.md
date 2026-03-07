# Contributing to SpringbootGen

First off, thank you for considering contributing to SpringbootGen! It's people like you that make SpringbootGen such a great tool.

## 1. Local Development Setup

SpringbootGen is written in **Rust**. To develop locally:

1.  **Install Rust**: If you haven't already, install Rust using [rustup](https://rustup.rs/).
2.  **Clone the repository**:
    ```bash
    git clone https://github.com/Solo-Ranker/springboot-gen.git
    cd springboot-gen
    ```
3.  **Build the project**:
    ```bash
    cargo build
    ```
4.  **Run tests**:
    ```bash
    cargo test
    ```
5.  **Run formatting and linting**:
    ```bash
    cargo fmt
    cargo clippy --all-targets --all-features
    ```

## 2. Project Architecture

Before contributing, please read the [`ARCHITECTURE.md`](ARCHITECTURE.md) document to understand how the feature composition engine and handlebars templates work.

## 3. Adding a New Feature

If you want to add support for a new infrastructure tool (e.g., "RabbitMQ"):

1.  **Open `src/features/registry.rs`**.
2.  **Add a `FeatureSpec`**:
    *   Define `maven_deps` (Spring Boot starters, etc.).
    *   Define `properties` (application.yml keys).
    *   Define `docker_services` (the Docker Compose image, ports, env vars).
    *   List `java_files` needed (e.g., `RabbitMqConfig`).
3.  **Add Java Templates**:
    *   Create `templates/handle/your_template.hbs` or modify existing code generators inside `src/generators/`.
    *   Register it in the corresponding generator code (e.g., `src/generators/java_code.rs`).

## 4. Submitting a Pull Request

1.  **Fork** the repository and create your branch from `main`.
2.  **Write Tests** (if applicable) for the generated output.
3.  Ensure the code generator successfully runs your test project.
    *   `cargo run -- new test-app --features <your-feature>`
    *   `cd test-app && mvn spring-boot:run`
4.  Make sure your code passes formatting guidelines (`cargo fmt`) and `cargo clippy`.
5.  Open a Pull Request with a clear description of the feature or bugfix.

## 5. Code of Conduct

By participating in this project, you are expected to uphold a welcoming, inclusive, and professional environment. Treat everyone with respect.
