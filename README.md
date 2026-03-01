# SpringbootGen

> Production-grade Spring Boot project generator — you write business logic, SpringbootGen writes the infrastructure.

```
  ____             _             _                 _    ____            
 / ___| _ __  _ __(_)_ __   __ _| |__   ___   ___ | |_ / ___| ___ _ __  
 \___ \| '_ \| '__| | '_ \ / _` | '_ \ / _ \ / _ \| __| |  _ / _ \ '_ \ 
  ___) | |_) | |  | | | | | (_| | |_) | (_) | (_) | |_| |_| |  __/ | | |
 |____/| .__/|_|  |_|_| |_|\__, |_.__/ \___/ \___/ \__|\____|\___|_| |_|
       |_|                  |___/
```

## What it does

SpringbootGen generates a **complete, production-ready** Spring Boot project from a single command.
Everything is wired up — you immediately write business logic.

| What SpringbootGen generates | What you write |
|--------------------------|----------------|
| `pom.xml` with all dependencies | Domain entities |
| `application.yml` (dev/prod/test) | Repository interfaces |
| `docker-compose.yml` + `Dockerfile` | Service business logic |
| Redis/Kafka/DB Java config classes | REST controllers |
| JWT auth filter + SecurityConfig | DTOs specific to your domain |
| `GlobalExceptionHandler` | Custom exception types |
| `.env.example` + K8s manifests | Tests |

---

## Installation

```bash
# Build from source
cargo build --release

# Move to PATH
sudo mv target/release/springboot-gen /usr/local/bin/

# Verify
springboot-gen --version
```

---

## Quick Start

```bash
# New project with PostgreSQL, Redis, Kafka, JWT auth, OpenAPI
springboot-gen new my-service \
  --group com.acme \
  --features postgres,redis,kafka,jwt,openapi,actuator,docker

cd my-service
docker-compose up -d
mvn spring-boot:run
```

---

## Commands

### `springboot-gen new <NAME>`

Create a brand-new project.

```bash
springboot-gen new payment-service \
  --group com.acme.payments \
  --boot-version 3.2.5 \
  --java-version 21 \
  --features postgres,redis,kafka,jwt,openapi,actuator,docker,kubernetes
```

| Flag | Default | Description |
|------|---------|-------------|
| `--group` | `com.example` | Maven group ID |
| `--boot-version` | `3.2.5` | Spring Boot version |
| `--java-version` | `21` | Java version |
| `--features` | *(none)* | Comma-separated feature list |
| `--redis-mode` | `standalone` | `standalone` / `ssl` / `sentinel` / `cluster` |
| `--output` | `./<name>` | Output directory |
| `--force` | false | Overwrite existing directory |
| `--emit-config` | true | Write `springboot-gen.toml` |

---

### `springboot-gen init`

Interactive guided setup (TUI with feature checkboxes).

```bash
springboot-gen init
```

---

### `springboot-gen add <FEATURES>`

Add features to an **existing** SpringbootGen project.

```bash
springboot-gen add elasticsearch,tracing --path ./my-service
```

This updates `pom.xml`, `application.yml`, `docker-compose.yml`, generates new Java config
classes, and updates `springboot-gen.toml`.

---

### `springboot-gen analyze <PATH>`

**Analyze an existing Spring Boot project** — detect its features and configuration.

```bash
# Print a table report
springboot-gen analyze ./legacy-service

# Output as TOML (for use as springboot-gen.toml)
springboot-gen analyze ./legacy-service --format toml

# Output as JSON
springboot-gen analyze ./legacy-service --format json

# Detect and save springboot-gen.toml
springboot-gen analyze ./legacy-service --save
```

**What it detects:**
- Spring Boot version and Java version (from pom.xml)
- Redis mode (standalone, SSL, Sentinel — by inspecting config + Java source)
- Kafka bootstrap servers and consumer group
- Database type (PostgreSQL, MySQL, MongoDB)
- Security mode (Basic, JWT, OAuth2)
- Presence of Actuator, OpenAPI, Tracing, Elasticsearch, S3, Email, WebSocket
- Docker/docker-compose presence
- Configuration warnings (weak JWT secrets, dangerous DDL-auto settings)

---

### `springboot-gen import <PATH>`

Like `analyze`, but also **generates supplemental files** — updated `application.yml`,
`.env.example`, `docker-compose.yml`, and K8s manifests, output to `./springboot-gen-out/`.

```bash
# Auto-detect everything
springboot-gen import ./legacy-service --output ./springboot-gen-out

# Auto-detect + add extra features not present
springboot-gen import ./legacy-service --features tracing,kubernetes --output ./infra
```

---

### `springboot-gen features`

List all available features.

```bash
springboot-gen features
```

---

### `springboot-gen preview`

Show what would be generated **without writing any files** (dry-run).

```bash
springboot-gen preview my-service --features postgres,redis,kafka,jwt
```

---

### `springboot-gen validate`

Validate a `springboot-gen.toml` configuration file.

```bash
springboot-gen validate springboot-gen.toml
```

---

## Feature Reference

### Data Layer

| Key | Description |
|-----|-------------|
| `redis` | Redis standalone (Lettuce, connection pool, `RedisTemplate`) |
| `redis-ssl` | Redis with TLS — generates SSL bundle config, `RedisSslConfig.java` |
| `redis-sentinel` | Redis Sentinel HA — `RedisSentinelConfig.java`, 3 Sentinel Docker services |
| `postgres` | PostgreSQL + JPA + Flyway + HikariCP |
| `mysql` | MySQL 8 + JPA + Flyway |
| `mongodb` | MongoDB + Spring Data |
| `elasticsearch` | Elasticsearch 8.x |

### Messaging

| Key | Description |
|-----|-------------|
| `kafka` | Kafka producer/consumer, retry topics, DLQ, Kafka UI in Docker |

### Security

| Key | Description | Requires |
|-----|-------------|---------|
| `security` | Spring Security with CORS, CSRF disabled | — |
| `jwt` | Stateless JWT (JJWT), access + refresh tokens, `AuthController` | `security` |
| `oauth2` | OAuth2 resource server with JWKS validation | `security` |

> `jwt` and `oauth2` are mutually exclusive.

### Redis Modes

#### Standalone (default)
```bash
springboot-gen new svc --features redis
```
Generates `RedisConfig.java` with Lettuce pool.

#### SSL / TLS
```bash
springboot-gen new svc --features redis-ssl
# or
springboot-gen new svc --features redis --redis-mode ssl
```
Generates `RedisSslConfig.java` with Spring Boot 3 SSL bundle integration:
```yaml
spring:
  data.redis:
    ssl.enabled: true
    ssl.bundle: redis-ssl
  ssl.bundle.jks.redis-ssl:
    keystore.location: ${REDIS_SSL_KEYSTORE:classpath:ssl/redis-client.p12}
    truststore.location: ${REDIS_SSL_TRUSTSTORE:classpath:ssl/redis-truststore.p12}
```

Cert generation helper:
```bash
keytool -genkeypair -alias redis-client -keyalg RSA -keysize 2048 \
  -storetype PKCS12 \
  -keystore src/main/resources/ssl/redis-client.p12 \
  -validity 365 -storepass changeit
```

#### Sentinel HA
```bash
springboot-gen new svc --features redis-sentinel
# or
springboot-gen new svc --features redis --redis-mode sentinel
```
Generates `RedisSentinelConfig.java`:
```java
sentinelConfig.sentinel("sentinel-host-1", 26379);
sentinelConfig.sentinel("sentinel-host-2", 26379);
sentinelConfig.sentinel("sentinel-host-3", 26379);
```
And a Docker Compose setup with `redis-master`, `redis-replica-1`, and `redis-sentinel-1`.

Environment variables:
```env
REDIS_SENTINEL_MASTER=mymaster
REDIS_SENTINEL_NODES=localhost:26379,localhost:26380,localhost:26381
REDIS_SENTINEL_PASSWORD=
REDIS_PASSWORD=
```

---

## `springboot-gen.toml` Reference

The config file (auto-generated, optional) allows fine-grained control:

```toml
[project]
name = "my-service"
group = "com.acme"
version = "0.0.1-SNAPSHOT"
description = "My Service"
java_version = 21
boot_version = "3.2.5"

features = ["postgres", "redis-sentinel", "kafka", "jwt", "openapi", "actuator", "docker"]

[redis]
mode = "sentinel"               # standalone | ssl | sentinel | cluster

[redis.sentinel]
master = "mymaster"
nodes = ["redis-1:26379", "redis-2:26379", "redis-3:26379"]
sentinel_password = ""          # optional

[redis.ssl]
keystore_location = "classpath:ssl/redis-client.p12"
keystore_password = "changeit"
truststore_location = "classpath:ssl/redis-truststore.p12"
truststore_password = "changeit"
verify_hostname = true

[kafka]
bootstrap_servers = "kafka-1:9092,kafka-2:9092"
consumer_group_id = "my-service"
auto_offset_reset = "earliest"
listener_concurrency = 3
idempotent_producer = true

[database]
host = "postgres"
port = 5432
name = "appdb"
username = "postgres"
pool_max_size = 10
flyway_enabled = true

[security]
access_token_expiry_ms = 900000
refresh_token_expiry_ms = 604800000
cors_allowed_origins = ["https://app.example.com"]

[docker]
registry = "registry.example.com"
base_image = "eclipse-temurin:21-jre-alpine"
app_port = 8080
```

---

## Supporting an Existing Project

### Scenario 1: You have a Spring Boot project with no SpringbootGen config

```bash
# Step 1: Analyze what's already there
springboot-gen analyze ./my-existing-service --format table

# Step 2: Save the detected config
springboot-gen analyze ./my-existing-service --save

# Step 3: Generate supplemental infrastructure files
springboot-gen import ./my-existing-service --output ./infra-supplement

# Step 4: Manually merge generated files into your project
# The CLI prints an integration guide at the end
```

### Scenario 2: Add missing infrastructure to an existing SpringbootGen project

```bash
# Add Redis Sentinel + Distributed Tracing
springboot-gen add redis-sentinel,tracing --path ./my-service
```

### Scenario 3: Detect Redis mode in a legacy project

The analyzer inspects:
1. `pom.xml` — dependency presence
2. `application.yml/properties` — `sentinel.master`, `ssl.enabled`, `cluster.nodes`
3. Java source — `RedisSentinelConfiguration`, `SslOptions`, `RedisClusterConfiguration`

Detection confidence levels:
- **HIGH** — dependency + configuration both detected
- **MEDIUM** — only in pom.xml or only in config
- **LOW** — inferred from naming patterns

---

## Generated File Overview

```
my-service/
├── pom.xml                                  # All deps, profiles, plugins
├── springboot-gen.toml                           # Reproducible config
├── Dockerfile                               # Multi-stage, non-root
├── docker-compose.yml                       # All infra services + app
├── docker-compose.override.yml              # Dev overrides (.env, hot reload)
├── .dockerignore
├── .env                                     # Local dev (gitignored)
├── .env.example                             # Committed safe version
├── .gitignore
├── k8s/                                     # When kubernetes feature active
│   ├── namespace.yaml
│   ├── configmap.yaml
│   ├── secret.yaml
│   ├── deployment.yaml                      # With liveness/readiness probes
│   ├── service.yaml
│   ├── hpa.yaml                             # CPU/memory autoscaling
│   ├── ingress.yaml
│   └── kustomization.yaml
└── src/
    ├── main/
    │   ├── java/com/acme/myservice/
    │   │   ├── MyServiceApplication.java    # @SpringBootApplication + auditing
    │   │   ├── config/
    │   │   │   ├── RedisConfig.java         # (or RedisSslConfig / RedisSentinelConfig)
    │   │   │   ├── CacheConfig.java         # @EnableCaching, per-cache TTL
    │   │   │   ├── KafkaConfig.java         # Retry + error handler
    │   │   │   ├── KafkaTopicConfig.java    # Topic definitions
    │   │   │   ├── SecurityConfig.java      # CORS, CSRF, stateless JWT
    │   │   │   ├── JwtProperties.java       # @ConfigurationProperties
    │   │   │   ├── JwtAuthenticationFilter.java
    │   │   │   └── OpenApiConfig.java
    │   │   ├── controller/
    │   │   │   ├── AuthController.java      # POST /api/v1/auth/login
    │   │   │   └── HealthController.java    # GET /api/v1/ping
    │   │   ├── service/
    │   │   │   ├── JwtService.java          # Token generation & validation
    │   │   │   ├── KafkaProducerService.java
    │   │   │   └── KafkaConsumerService.java
    │   │   ├── dto/
    │   │   │   ├── ApiResponse.java         # Generic response envelope
    │   │   │   ├── TokenResponse.java
    │   │   │   └── LoginRequest.java
    │   │   └── exception/
    │   │       └── GlobalExceptionHandler.java  # ProblemDetail RFC 9457
    │   └── resources/
    │       ├── application.yml              # Base config
    │       ├── application-dev.yml          # Dev overrides (debug logging)
    │       ├── application-prod.yml         # Prod overrides (swagger off)
    │       ├── ssl/                         # When redis-ssl active
    │       └── db/migration/
    │           └── V1__init_schema.sql      # Flyway baseline
    └── test/
        ├── java/.../integration/
        └── java/.../unit/
```

---

## Architecture

```
springboot-gen/
├── src/
│   ├── cli/          # Command parsing (clap), interactive init (dialoguer)
│   ├── config/       # ProjectConfig — springboot-gen.toml schema (serde)
│   ├── features/     # Feature registry with deps, properties, Docker services
│   ├── engine/       # GenerationEngine — orchestrates all generators
│   ├── generators/
│   │   ├── pom           # Maven pom.xml
│   │   ├── spring_properties  # application.yml (base/dev/prod)
│   │   ├── docker        # Dockerfile + docker-compose.yml
│   │   ├── java_code     # All Java configuration/boilerplate
│   │   ├── env_file      # .env.example
│   │   └── kubernetes    # K8s manifests
│   └── analyzer/     # Project scanner — detects features from existing projects
```

### Feature composition engine

Each `FeatureSpec` in `features/registry.rs` declares:

```rust
FeatureSpec {
    key: "redis-sentinel",
    maven_deps: &[...],         // → pom.xml
    requires: &[""],            // → transitive resolution
    properties: &[...],         // → application.yml
    docker_services: &[...],    // → docker-compose.yml
    env_vars: &[...],           // → .env.example
    java_files: &[...],         // → generated Java classes
    conflicts: &["redis", "redis-ssl"],
}
```

The engine resolves transitive dependencies, checks conflicts, then fans out to
each generator.

---

## Security Hardening

Generated code follows security best practices:

- **Non-root Docker user**: `addgroup spring && adduser spring`
- **Stateless JWT**: `SessionCreationPolicy.STATELESS`
- **BCrypt passwords**: `BCryptPasswordEncoder` (cost factor 10)
- **CSRF disabled** (stateless APIs don't need it; re-enable for SSR)
- **CORS configured** with `allowedOriginPatterns`
- **Actuator secured**: only `health`, `info`, `prometheus` exposed by default
- **Hikari pool limits**: prevents DB connection exhaustion
- **Idempotent Kafka producer**: `enable.idempotence=true`
- **K8s security context**: `runAsNonRoot: true`, `runAsUser: 1000`

---

## License

MIT
