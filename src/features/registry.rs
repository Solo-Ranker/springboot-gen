use indexmap::IndexMap;
use serde::Serialize;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct FeatureSpec {
    pub key: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub maven_deps: &'static [MavenDep],
    pub requires: &'static [&'static str],
    pub docker_services: &'static [DockerService],
    pub env_vars: &'static [(&'static str, &'static str)],
    pub java_files: &'static [&'static str],
    pub conflicts: &'static [&'static str],
}

#[derive(Debug, Clone, Serialize)]
pub struct MavenDep {
    pub group_id: &'static str,
    pub artifact_id: &'static str,
    pub version: Option<&'static str>,
    pub scope: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DockerService {
    pub name: &'static str,
    pub image: &'static str,
    pub ports: &'static [&'static str],
    pub environment: &'static [(&'static str, &'static str)],
    pub volumes: &'static [&'static str],
    pub healthcheck: Option<&'static str>,
    pub depends_on: &'static [&'static str],
}

// ── Shorthand macros ──────────────────────────────────────────────────────────

/// dep!("group", "artifact")
/// dep!("group", "artifact", version: "1.0")
/// dep!("group", "artifact", scope: "test")
/// dep!("group", "artifact", version: "1.0", scope: "runtime")
macro_rules! dep {
    ($g:literal, $a:literal) => {
        MavenDep {
            group_id: $g,
            artifact_id: $a,
            version: None,
            scope: None,
        }
    };
    ($g:literal, $a:literal, version: $v:literal) => {
        MavenDep {
            group_id: $g,
            artifact_id: $a,
            version: Some($v),
            scope: None,
        }
    };
    ($g:literal, $a:literal, scope: $s:literal) => {
        MavenDep {
            group_id: $g,
            artifact_id: $a,
            version: None,
            scope: Some($s),
        }
    };
    ($g:literal, $a:literal, version: $v:literal, scope: $s:literal) => {
        MavenDep {
            group_id: $g,
            artifact_id: $a,
            version: Some($v),
            scope: Some($s),
        }
    };
}

/// svc!("name", "image", ports: [...], env: [...], volumes: [...], health: "cmd", depends: [...])
/// All fields after image are optional and can be omitted or reordered.
macro_rules! svc {
    ($name:literal, $image:literal
     $(, ports:   $ports:expr)?
     $(, env:     $env:expr)?
     $(, volumes: $vols:expr)?
     $(, health:  $hc:literal)?
     $(, depends: $deps:expr)?
     $(,)?
    ) => {
        DockerService {
            name:        $name,
            image:       $image,
            ports:       svc!(@opt_slice $($ports)?),
            environment: svc!(@opt_slice $($env)?),
            volumes:     svc!(@opt_slice $($vols)?),
            healthcheck: svc!(@opt_str $($hc)?),
            depends_on:  svc!(@opt_slice $($deps)?),
        }
    };
    (@opt_slice)         => { &[] };
    (@opt_slice $e:expr) => { $e };
    (@opt_str)           => { None };
    (@opt_str $s:literal) => { Some($s) };
}

/// feature!(...) — build a FeatureSpec with all fields
macro_rules! feature {
    (
        key:         $key:literal,
        name:        $name:literal,
        description: $desc:literal,
        deps:        [$($dep:expr),* $(,)?],
        requires:    [$($req:literal),* $(,)?],
        services:    [$($svc:expr),* $(,)?],
        env:         [$($ek:literal => $ev:literal),* $(,)?],
        java:        [$($jf:literal),* $(,)?],
        conflicts:   [$($cf:literal),* $(,)?],
    ) => {
        FeatureSpec {
            key:             $key,
            name:            $name,
            description:     $desc,
            maven_deps:      &[$($dep),*],
            requires:        &[$($req),*],
            docker_services: &[$($svc),*],
            env_vars:        &[$(($ek, $ev)),*],
            java_files:      &[$($jf),*],
            conflicts:       &[$($cf),*],
        }
    };
}

// ── Registry ──────────────────────────────────────────────────────────────────

pub fn all_features() -> Vec<FeatureSpec> {
    vec![
        feature!(
            key:         "redis",
            name:        "Redis (Standalone)",
            description: "Redis cache, pub/sub, and session store",
            deps: [
                dep!("org.springframework.boot", "spring-boot-starter-data-redis"),
                dep!("io.lettuce", "lettuce-core"),
                dep!("org.apache.commons", "commons-pool2"),
            ],
            requires:  [],
            services:  [svc!("redis", "redis:7.2-alpine",
                ports:   &["6379:6379"],
                volumes: &["redis_data:/data"],
                health:  "redis-cli ping",
            )],
            env: [
                "REDIS_HOST" => "localhost",
                "REDIS_PORT" => "6379",
                "REDIS_PASSWORD" => "",
                "REDIS_DB" => "0",
            ],
            java:      ["RedisConfig", "CacheConfig"],
            conflicts: ["redis-ssl", "redis-sentinel"],
        ),
        feature!(
            key:         "redis-ssl",
            name:        "Redis (SSL/TLS)",
            description: "Redis with TLS encryption",
            deps: [
                dep!("org.springframework.boot", "spring-boot-starter-data-redis"),
                dep!("io.lettuce", "lettuce-core"),
                dep!("org.apache.commons", "commons-pool2"),
            ],
            requires:  [],
            services:  [svc!("redis", "redis:7.2-alpine",
                ports:   &["6380:6380"],
                volumes: &["redis_data:/data", "./ssl/redis:/tls:ro"],
                health:  "redis-cli --tls ping",
            )],
            env: [
                "REDIS_HOST" => "localhost",
                "REDIS_PORT" => "6380",
                "REDIS_PASSWORD" => "",
                "REDIS_SSL_KEYSTORE" => "classpath:ssl/redis-client.p12",
                "REDIS_SSL_KEYSTORE_PASSWORD" => "changeit",
                "REDIS_SSL_TRUSTSTORE" => "classpath:ssl/redis-truststore.p12",
                "REDIS_SSL_TRUSTSTORE_PASSWORD" => "changeit",
            ],
            java:      ["RedisSslConfig", "CacheConfig"],
            conflicts: ["redis", "redis-sentinel"],
        ),
        feature!(
            key:         "redis-sentinel",
            name:        "Redis (Sentinel HA)",
            description: "Redis with Sentinel for automatic failover",
            deps: [
                dep!("org.springframework.boot", "spring-boot-starter-data-redis"),
                dep!("io.lettuce", "lettuce-core"),
                dep!("org.apache.commons", "commons-pool2"),
            ],
            requires:  [],
            services:  [
                svc!("redis-master", "redis:7.2-alpine",
                    ports:   &["6379:6379"],
                    volumes: &["redis_master_data:/data"],
                    health:  "redis-cli ping",
                ),
                svc!("redis-replica-1", "redis:7.2-alpine",
                    ports:   &["6380:6379"],
                    volumes: &["redis_replica1_data:/data"],
                    health:  "redis-cli ping",
                    depends: &["redis-master"],
                ),
                svc!("redis-sentinel-1", "redis:7.2-alpine",
                    ports:   &["26379:26379"],
                    health:  "redis-cli -p 26379 ping",
                    depends: &["redis-master", "redis-replica-1"],
                ),
            ],
            env: [
                "REDIS_SENTINEL_MASTER" => "mymaster",
                "REDIS_SENTINEL_NODES" => "localhost:26379,localhost:26380,localhost:26381",
                "REDIS_SENTINEL_PASSWORD" => "",
                "REDIS_PASSWORD" => "",
                "REDIS_DB" => "0",
            ],
            java:      ["RedisSentinelConfig", "CacheConfig"],
            conflicts: ["redis", "redis-ssl"],
        ),
        feature!(
            key:         "kafka",
            name:        "Apache Kafka",
            description: "Kafka producer/consumer with schema registry support",
            deps: [
                dep!("org.springframework.kafka", "spring-kafka"),
                dep!("org.apache.kafka", "kafka-clients"),
            ],
            requires:  [],
            services:  [
                svc!("zookeeper", "confluentinc/cp-zookeeper:7.6.0",
                    ports: &["2181:2181"],
                    env:   &[("ZOOKEEPER_CLIENT_PORT", "2181"), ("ZOOKEEPER_TICK_TIME", "2000")],
                ),
                svc!("kafka", "confluentinc/cp-kafka:7.6.0",
                    ports:   &["9092:9092", "9101:9101"],
                    env:     &[
                        ("KAFKA_BROKER_ID", "1"),
                        ("KAFKA_ZOOKEEPER_CONNECT", "zookeeper:2181"),
                        ("KAFKA_ADVERTISED_LISTENERS", "PLAINTEXT://localhost:9092"),
                        ("KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR", "1"),
                        ("KAFKA_AUTO_CREATE_TOPICS_ENABLE", "true"),
                    ],
                    health:  "kafka-broker-api-versions --bootstrap-server localhost:9092",
                    depends: &["zookeeper"],
                ),
                svc!("kafka-ui", "provectuslabs/kafka-ui:latest",
                    ports:   &["8090:8080"],
                    env:     &[
                        ("KAFKA_CLUSTERS_0_NAME", "local"),
                        ("KAFKA_CLUSTERS_0_BOOTSTRAPSERVERS", "kafka:9092"),
                    ],
                    depends: &["kafka"],
                ),
            ],
            env: [
                "KAFKA_BOOTSTRAP_SERVERS" => "localhost:9092",
                "KAFKA_LISTENER_CONCURRENCY" => "3",
            ],
            java:      ["KafkaConfig", "KafkaProducerService", "KafkaConsumerService", "KafkaTopicConfig"],
            conflicts: ["rabbitmq", "ibmmq"],
        ),
        feature!(
            key:         "rabbitmq",
            name:        "RabbitMQ",
            description: "RabbitMQ messaging with Spring AMQP",
            deps: [dep!("org.springframework.boot", "spring-boot-starter-amqp")],
            requires:  [],
            services:  [svc!("rabbitmq", "rabbitmq:3-management",
                ports:   &["5672:5672", "15672:15672"],
                env:     &[
                    ("RABBITMQ_DEFAULT_USER", "${RABBITMQ_USER:-guest}"),
                    ("RABBITMQ_DEFAULT_PASS", "${RABBITMQ_PASSWORD:-guest}"),
                ],
                volumes: &["rabbitmq_data:/var/lib/rabbitmq"],
                health:  "rabbitmq-diagnostics -q ping",
            )],
            env: [
                "RABBITMQ_HOST" => "localhost",
                "RABBITMQ_PORT" => "5672",
                "RABBITMQ_USER" => "guest",
                "RABBITMQ_PASSWORD" => "guest",
            ],
            java:      ["RabbitMqConfig"],
            conflicts: ["kafka", "ibmmq"],
        ),
        feature!(
            key:         "ibmmq",
            name:        "IBM MQ",
            description: "IBM MQ (JMS) integration",
            deps: [dep!("com.ibm.mq", "mq-jms-spring-boot-starter", version: "3.3.4")],
            requires:  [],
            services:  [svc!("ibmmq", "ibmcom/mq:latest",
                ports:   &["1414:1414", "9443:9443"],
                env:     &[("LICENSE", "accept"), ("MQ_QMGR_NAME", "${IBM_MQ_QM:-QM1}")],
                volumes: &["ibmmq_data:/mnt/mqm"],
            )],
            env: [
                "IBM_MQ_QM" => "QM1",
                "IBM_MQ_CHANNEL" => "DEV.APP.SVRCONN",
                "IBM_MQ_HOST" => "localhost",
                "IBM_MQ_PORT" => "1414",
                "IBM_MQ_USER" => "app",
                "IBM_MQ_PASSWORD" => "",
            ],
            java:      ["IbmMqConfig"],
            conflicts: ["kafka", "rabbitmq"],
        ),
        feature!(
            key:         "postgres",
            name:        "PostgreSQL",
            description: "PostgreSQL with Spring Data JPA, Flyway, and HikariCP",
            deps: [
                dep!("org.springframework.boot", "spring-boot-starter-data-jpa"),
                dep!("org.postgresql", "postgresql"),
                dep!("org.flywaydb", "flyway-core"),
                dep!("com.zaxxer", "HikariCP"),
            ],
            requires:  [],
            services:  [svc!("postgres", "postgres:16-alpine",
                ports:   &["5432:5432"],
                env:     &[
                    ("POSTGRES_DB",       "${DB_NAME:-appdb}"),
                    ("POSTGRES_USER",     "${DB_USER:-postgres}"),
                    ("POSTGRES_PASSWORD", "${DB_PASSWORD:-postgres}"),
                ],
                volumes: &["postgres_data:/var/lib/postgresql/data"],
                health:  "pg_isready -U ${DB_USER:-postgres}",
            )],
            env: [
                "DB_HOST" => "localhost",
                "DB_PORT" => "5432",
                "DB_NAME" => "appdb",
                "DB_USER" => "postgres",
                "DB_PASSWORD" => "postgres",
                "DB_POOL_MAX" => "10",
            ],
            java:      ["JpaConfig"],
            conflicts: ["mysql"],
        ),
        feature!(
            key:         "mysql",
            name:        "MySQL",
            description: "MySQL 8 with Spring Data JPA and Flyway",
            deps: [
                dep!("org.springframework.boot", "spring-boot-starter-data-jpa"),
                dep!("com.mysql", "mysql-connector-j"),
                dep!("org.flywaydb", "flyway-mysql"),
                dep!("com.zaxxer", "HikariCP"),
            ],
            requires:  [],
            services:  [svc!("mysql", "mysql:8.3",
                ports:   &["3306:3306"],
                env:     &[
                    ("MYSQL_ROOT_PASSWORD", "${DB_PASSWORD:-root}"),
                    ("MYSQL_DATABASE",      "${DB_NAME:-appdb}"),
                ],
                volumes: &["mysql_data:/var/lib/mysql"],
                health:  "mysqladmin ping -h localhost",
            )],
            env: [
                "DB_HOST" => "localhost",
                "DB_PORT" => "3306",
                "DB_NAME" => "appdb",
                "DB_USER" => "root",
                "DB_PASSWORD" => "root",
            ],
            java:      ["JpaConfig"],
            conflicts: ["postgres"],
        ),
        feature!(
            key:         "mongodb",
            name:        "MongoDB",
            description: "MongoDB with Spring Data",
            deps: [dep!("org.springframework.boot", "spring-boot-starter-data-mongodb")],
            requires:  [],
            services:  [svc!("mongodb", "mongo:7.0",
                ports:   &["27017:27017"],
                env:     &[
                    ("MONGO_INITDB_ROOT_USERNAME", "${MONGO_USER:-root}"),
                    ("MONGO_INITDB_ROOT_PASSWORD", "${MONGO_PASSWORD:-root}"),
                    ("MONGO_INITDB_DATABASE",      "${MONGO_DB:-appdb}"),
                ],
                volumes: &["mongo_data:/data/db"],
                health:  "mongosh --eval \"db.adminCommand('ping')\"",
            )],
            env: [
                "MONGO_HOST" => "localhost",
                "MONGO_PORT" => "27017",
                "MONGO_DB" => "appdb",
                "MONGO_USER" => "root",
                "MONGO_PASSWORD" => "root",
            ],
            java:      ["MongoConfig"],
            conflicts: [],
        ),
        feature!(
            key:         "security",
            name:        "Spring Security",
            description: "Spring Security with RBAC, CORS, and CSRF configuration",
            deps: [dep!("org.springframework.boot", "spring-boot-starter-security")],
            requires:  [],
            services:  [],
            env: [
                "SECURITY_USER" => "admin",
                "SECURITY_PASSWORD" => "changeit",
            ],
            java:      ["SecurityConfig", "UserDetailsServiceImpl"],
            conflicts: [],
        ),
        feature!(
            key:         "jwt",
            name:        "JWT Authentication",
            description: "Stateless JWT auth with access/refresh token rotation",
            deps: [
                dep!("io.jsonwebtoken", "jjwt-api",     version: "0.12.5"),
                dep!("io.jsonwebtoken", "jjwt-impl",    version: "0.12.5", scope: "runtime"),
                dep!("io.jsonwebtoken", "jjwt-jackson", version: "0.12.5", scope: "runtime"),
            ],
            requires:  ["security"],
            services:  [],
            env: [
                "JWT_SECRET" => "change-me-in-production-with-256bit-key",
                "JWT_ACCESS_EXPIRY_MS" => "900000",
                "JWT_REFRESH_EXPIRY_MS" => "604800000",
            ],
            java:      ["JwtService", "JwtAuthenticationFilter", "JwtProperties", "AuthController", "TokenResponse"],
            conflicts: ["oauth2"],
        ),
        feature!(
            key:         "oauth2",
            name:        "OAuth2 Resource Server",
            description: "OAuth2/OIDC resource server with JWT validation",
            deps: [
                dep!("org.springframework.boot", "spring-boot-starter-oauth2-resource-server"),
                dep!("org.springframework.boot", "spring-boot-starter-oauth2-client"),
            ],
            requires:  ["security"],
            services:  [svc!("keycloak", "quay.io/keycloak/keycloak:24.0",
                ports:   &["8180:8080"],
                env:     &[("KEYCLOAK_ADMIN", "admin"), ("KEYCLOAK_ADMIN_PASSWORD", "admin")],
                health:  "curl -f http://localhost:8080/health/ready",
            )],
            env: [
                "OAUTH2_ISSUER_URI" => "http://localhost:8180/realms/app",
                "OAUTH2_JWK_URI" => "",
            ],
            java:      ["OAuth2SecurityConfig", "JwtConverterConfig"],
            conflicts: ["jwt"],
        ),
        feature!(
            key:         "openapi",
            name:        "OpenAPI / Swagger",
            description: "OpenAPI 3 documentation with Swagger UI",
            deps: [dep!("org.springdoc", "springdoc-openapi-starter-webmvc-ui", version: "2.5.0")],
            requires:  [],
            services:  [],
            env:       [],
            java:      ["OpenApiConfig"],
            conflicts: [],
        ),
        feature!(
            key:         "actuator",
            name:        "Spring Actuator",
            description: "Health checks, metrics, and management endpoints",
            deps: [
                dep!("org.springframework.boot", "spring-boot-starter-actuator"),
                dep!("io.micrometer", "micrometer-registry-prometheus"),
            ],
            requires:  [],
            services:  [],
            env:       [],
            java:      [],
            conflicts: [],
        ),
        feature!(
            key:         "tracing",
            name:        "Distributed Tracing",
            description: "Micrometer Tracing with Zipkin/Jaeger exporter",
            deps: [
                dep!("io.micrometer", "micrometer-tracing-bridge-brave"),
                dep!("io.zipkin.reporter2", "zipkin-reporter-brave"),
                dep!("com.github.loki4j", "loki-logback-appender", version: "1.5.2"),
            ],
            requires:  ["actuator"],
            services:  [svc!("zipkin", "openzipkin/zipkin:latest",
                ports:  &["9411:9411"],
                health: "wget -qO- http://localhost:9411/health",
            )],
            env: [
                "ZIPKIN_ENDPOINT" => "http://localhost:9411/api/v2/spans",
                "TRACING_SAMPLE_RATE" => "1.0",
            ],
            java:      [],
            conflicts: [],
        ),
        feature!(
            key:         "elasticsearch",
            name:        "Elasticsearch",
            description: "Spring Data Elasticsearch with Java client",
            deps: [dep!("org.springframework.boot", "spring-boot-starter-data-elasticsearch")],
            requires:  [],
            services:  [svc!("elasticsearch", "elasticsearch:8.12.0",
                ports:   &["9200:9200", "9300:9300"],
                env:     &[
                    ("discovery.type",        "single-node"),
                    ("xpack.security.enabled","false"),
                    ("ES_JAVA_OPTS",          "-Xms512m -Xmx512m"),
                ],
                volumes: &["es_data:/usr/share/elasticsearch/data"],
                health:  "curl -f http://localhost:9200/_cluster/health",
            )],
            env: [
                "ELASTICSEARCH_URIS" => "http://localhost:9200",
                "ELASTICSEARCH_USER" => "elastic",
                "ELASTICSEARCH_PASSWORD" => "elastic",
            ],
            java:      ["ElasticsearchConfig"],
            conflicts: [],
        ),
        feature!(
            key:         "s3",
            name:        "AWS S3 / MinIO",
            description: "S3 object storage with MinIO in local dev",
            deps: [
                dep!("software.amazon.awssdk", "s3",   version: "2.25.0"),
                dep!("software.amazon.awssdk", "auth", version: "2.25.0"),
            ],
            requires:  [],
            services:  [svc!("minio", "minio/minio:latest",
                ports:   &["9000:9000", "9001:9001"],
                env:     &[
                    ("MINIO_ROOT_USER",     "${MINIO_USER:-minioadmin}"),
                    ("MINIO_ROOT_PASSWORD", "${MINIO_PASSWORD:-minioadmin}"),
                ],
                volumes: &["minio_data:/data"],
                health:  "curl -f http://localhost:9000/minio/health/live",
            )],
            env: [
                "S3_BUCKET" => "app-uploads",
                "AWS_REGION" => "us-east-1",
                "S3_ENDPOINT" => "http://localhost:9000",
                "S3_PATH_STYLE" => "true",
                "AWS_ACCESS_KEY_ID" => "minioadmin",
                "AWS_SECRET_ACCESS_KEY" => "minioadmin",
            ],
            java:      ["S3Config", "S3Service"],
            conflicts: [],
        ),
        feature!(
            key:         "email",
            name:        "Email (Spring Mail)",
            description: "Email sending with Thymeleaf templates and MailHog dev server",
            deps: [
                dep!("org.springframework.boot", "spring-boot-starter-mail"),
                dep!("org.springframework.boot", "spring-boot-starter-thymeleaf"),
            ],
            requires:  [],
            services:  [svc!("mailhog", "mailhog/mailhog:latest",
                ports: &["1025:1025", "8025:8025"],
            )],
            env: [
                "MAIL_HOST" => "localhost",
                "MAIL_PORT" => "1025",
                "MAIL_USER" => "",
                "MAIL_PASSWORD" => "",
                "MAIL_FROM" => "noreply@example.com",
            ],
            java:      ["EmailConfig", "EmailService"],
            conflicts: [],
        ),
        feature!(
            key:         "websocket",
            name:        "WebSocket",
            description: "STOMP WebSocket with SockJS fallback",
            deps: [dep!("org.springframework.boot", "spring-boot-starter-websocket")],
            requires:  [],
            services:  [],
            env: ["WS_ALLOWED_ORIGINS" => "*"],
            java:      ["WebSocketConfig", "WebSocketSecurityConfig"],
            conflicts: [],
        ),
        feature!(
            key:         "docker",
            name:        "Docker",
            description: "Multi-stage Dockerfile and docker-compose.yml",
            deps:      [],
            requires:  [],
            services:  [],
            env:       [],
            java:      [],
            conflicts: [],
        ),
        feature!(
            key:         "kubernetes",
            name:        "Kubernetes",
            description: "Kubernetes manifests (Deployment, Service, ConfigMap, HPA)",
            deps:      [],
            requires:  ["actuator"],
            services:  [],
            env:       [],
            java:      [],
            conflicts: [],
        ),
    ]
}

// ── Lookup / resolution ───────────────────────────────────────────────────────

pub fn find_feature(key: &str) -> Option<FeatureSpec> {
    all_features().into_iter().find(|f| f.key == key)
}

pub fn resolve_features(keys: &[String]) -> anyhow::Result<Vec<FeatureSpec>> {
    let all = all_features();
    let mut resolved: IndexMap<String, FeatureSpec> = IndexMap::new();
    let mut queue = keys.to_vec();

    while let Some(key) = queue.pop() {
        if resolved.contains_key(&key) {
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

        for dep in spec.requires {
            if !resolved.contains_key(*dep) {
                queue.push(dep.to_string());
            }
        }

        resolved.insert(key, spec);
    }

    for (key, spec) in &resolved {
        for conflict in spec.conflicts {
            if resolved.contains_key(*conflict) {
                anyhow::bail!(
                    "Feature conflict: '{}' and '{}' cannot be used together",
                    key,
                    conflict
                );
            }
        }
    }

    Ok(resolved.into_values().collect())
}
