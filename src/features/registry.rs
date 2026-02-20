use indexmap::IndexMap;
use serde::Serialize;

/// A single feature's full specification
#[derive(Debug, Clone, Serialize)]
pub struct FeatureSpec {
    /// Unique key used in CLI flags and config
    pub key: &'static str,
    /// Human-readable name
    pub name: &'static str,
    /// Short description shown in `features` list
    pub description: &'static str,
    /// Maven/Gradle dependencies added when this feature is active
    pub maven_deps: &'static [MavenDep],
    /// Other feature keys this feature requires (automatically added)
    pub requires: &'static [&'static str],
    /// Spring properties this feature adds to application.yml
    pub properties: &'static [(&'static str, &'static str)],
    /// Docker services to add to docker-compose.yml
    pub docker_services: &'static [DockerService],
    /// Environment variables for .env and docker-compose
    pub env_vars: &'static [(&'static str, &'static str)],
    /// Java source files to generate
    pub java_files: &'static [&'static str],
    /// Whether the feature conflicts with another
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

/// Returns the complete registry of all available features
pub fn all_features() -> Vec<FeatureSpec> {
    vec![
        // ─────────────────────────────────────────────────────────
        // Redis Standalone
        // ─────────────────────────────────────────────────────────
        FeatureSpec {
            key: "redis",
            name: "Redis (Standalone)",
            description: "Redis cache, pub/sub, and session store",
            maven_deps: &[
                MavenDep { group_id: "org.springframework.boot", artifact_id: "spring-boot-starter-data-redis", version: None, scope: None },
                MavenDep { group_id: "io.lettuce", artifact_id: "lettuce-core", version: None, scope: None },
                MavenDep { group_id: "org.apache.commons", artifact_id: "commons-pool2", version: None, scope: None },
            ],
            requires: &[],
            properties: &[
                ("spring.data.redis.host", "${REDIS_HOST:localhost}"),
                ("spring.data.redis.port", "${REDIS_PORT:6379}"),
                ("spring.data.redis.password", "${REDIS_PASSWORD:}"),
                ("spring.data.redis.database", "${REDIS_DB:0}"),
                ("spring.data.redis.timeout", "2000ms"),
                ("spring.data.redis.lettuce.pool.max-active", "8"),
                ("spring.data.redis.lettuce.pool.max-idle", "8"),
                ("spring.data.redis.lettuce.pool.min-idle", "0"),
            ],
            docker_services: &[DockerService {
                name: "redis",
                image: "redis:7.2-alpine",
                ports: &["6379:6379"],
                environment: &[],
                volumes: &["redis_data:/data"],
                healthcheck: Some("redis-cli ping"),
                depends_on: &[],
            }],
            env_vars: &[
                ("REDIS_HOST", "localhost"),
                ("REDIS_PORT", "6379"),
                ("REDIS_PASSWORD", ""),
                ("REDIS_DB", "0"),
            ],
            java_files: &["RedisConfig", "CacheConfig"],
            conflicts: &["redis-ssl", "redis-sentinel"],
        },

        // ─────────────────────────────────────────────────────────
        // Redis SSL/TLS
        // ─────────────────────────────────────────────────────────
        FeatureSpec {
            key: "redis-ssl",
            name: "Redis (SSL/TLS)",
            description: "Redis with TLS encryption — uses Lettuce with SSL context",
            maven_deps: &[
                MavenDep { group_id: "org.springframework.boot", artifact_id: "spring-boot-starter-data-redis", version: None, scope: None },
                MavenDep { group_id: "io.lettuce", artifact_id: "lettuce-core", version: None, scope: None },
                MavenDep { group_id: "org.apache.commons", artifact_id: "commons-pool2", version: None, scope: None },
            ],
            requires: &[],
            properties: &[
                ("spring.data.redis.host", "${REDIS_HOST:localhost}"),
                ("spring.data.redis.port", "${REDIS_PORT:6380}"),
                ("spring.data.redis.password", "${REDIS_PASSWORD:}"),
                ("spring.data.redis.ssl.enabled", "true"),
                ("spring.data.redis.ssl.bundle", "redis-ssl"),
                ("spring.ssl.bundle.jks.redis-ssl.keystore.location", "${REDIS_SSL_KEYSTORE:classpath:ssl/redis-client.p12}"),
                ("spring.ssl.bundle.jks.redis-ssl.keystore.password", "${REDIS_SSL_KEYSTORE_PASSWORD:changeit}"),
                ("spring.ssl.bundle.jks.redis-ssl.truststore.location", "${REDIS_SSL_TRUSTSTORE:classpath:ssl/redis-truststore.p12}"),
                ("spring.ssl.bundle.jks.redis-ssl.truststore.password", "${REDIS_SSL_TRUSTSTORE_PASSWORD:changeit}"),
                ("spring.data.redis.lettuce.pool.max-active", "8"),
                ("spring.data.redis.lettuce.pool.max-idle", "8"),
                ("spring.data.redis.lettuce.pool.min-idle", "0"),
            ],
            docker_services: &[DockerService {
                name: "redis",
                image: "redis:7.2-alpine",
                ports: &["6380:6380"],
                environment: &[],
                volumes: &["redis_data:/data", "./ssl/redis:/tls:ro"],
                healthcheck: Some("redis-cli --tls ping"),
                depends_on: &[],
            }],
            env_vars: &[
                ("REDIS_HOST", "localhost"),
                ("REDIS_PORT", "6380"),
                ("REDIS_PASSWORD", ""),
                ("REDIS_SSL_KEYSTORE", "classpath:ssl/redis-client.p12"),
                ("REDIS_SSL_KEYSTORE_PASSWORD", "changeit"),
                ("REDIS_SSL_TRUSTSTORE", "classpath:ssl/redis-truststore.p12"),
                ("REDIS_SSL_TRUSTSTORE_PASSWORD", "changeit"),
            ],
            java_files: &["RedisSslConfig", "CacheConfig"],
            conflicts: &["redis", "redis-sentinel"],
        },

        // ─────────────────────────────────────────────────────────
        // Redis Sentinel
        // ─────────────────────────────────────────────────────────
        FeatureSpec {
            key: "redis-sentinel",
            name: "Redis (Sentinel HA)",
            description: "Redis with Sentinel for automatic failover and high availability",
            maven_deps: &[
                MavenDep { group_id: "org.springframework.boot", artifact_id: "spring-boot-starter-data-redis", version: None, scope: None },
                MavenDep { group_id: "io.lettuce", artifact_id: "lettuce-core", version: None, scope: None },
                MavenDep { group_id: "org.apache.commons", artifact_id: "commons-pool2", version: None, scope: None },
            ],
            requires: &[],
            properties: &[
                ("spring.data.redis.sentinel.master", "${REDIS_SENTINEL_MASTER:mymaster}"),
                ("spring.data.redis.sentinel.nodes", "${REDIS_SENTINEL_NODES:localhost:26379,localhost:26380,localhost:26381}"),
                ("spring.data.redis.sentinel.password", "${REDIS_SENTINEL_PASSWORD:}"),
                ("spring.data.redis.password", "${REDIS_PASSWORD:}"),
                ("spring.data.redis.database", "${REDIS_DB:0}"),
                ("spring.data.redis.lettuce.pool.max-active", "16"),
                ("spring.data.redis.lettuce.pool.max-idle", "8"),
                ("spring.data.redis.lettuce.pool.min-idle", "2"),
                ("spring.data.redis.lettuce.pool.max-wait", "-1ms"),
            ],
            docker_services: &[
                DockerService {
                    name: "redis-master",
                    image: "redis:7.2-alpine",
                    ports: &["6379:6379"],
                    environment: &[],
                    volumes: &["redis_master_data:/data"],
                    healthcheck: Some("redis-cli ping"),
                    depends_on: &[],
                },
                DockerService {
                    name: "redis-replica-1",
                    image: "redis:7.2-alpine",
                    ports: &["6380:6379"],
                    environment: &[],
                    volumes: &["redis_replica1_data:/data"],
                    healthcheck: Some("redis-cli ping"),
                    depends_on: &["redis-master"],
                },
                DockerService {
                    name: "redis-sentinel-1",
                    image: "redis:7.2-alpine",
                    ports: &["26379:26379"],
                    environment: &[],
                    volumes: &[],
                    healthcheck: Some("redis-cli -p 26379 ping"),
                    depends_on: &["redis-master", "redis-replica-1"],
                },
            ],
            env_vars: &[
                ("REDIS_SENTINEL_MASTER", "mymaster"),
                ("REDIS_SENTINEL_NODES", "localhost:26379,localhost:26380,localhost:26381"),
                ("REDIS_SENTINEL_PASSWORD", ""),
                ("REDIS_PASSWORD", ""),
                ("REDIS_DB", "0"),
            ],
            java_files: &["RedisSentinelConfig", "CacheConfig"],
            conflicts: &["redis", "redis-ssl"],
        },

        // ─────────────────────────────────────────────────────────
        // Kafka
        // ─────────────────────────────────────────────────────────
        FeatureSpec {
            key: "kafka",
            name: "Apache Kafka",
            description: "Kafka producer/consumer with schema registry support",
            maven_deps: &[
                MavenDep { group_id: "org.springframework.kafka", artifact_id: "spring-kafka", version: None, scope: None },
                MavenDep { group_id: "org.apache.kafka", artifact_id: "kafka-clients", version: None, scope: None },
            ],
            requires: &[],
            properties: &[
                ("spring.kafka.bootstrap-servers", "${KAFKA_BOOTSTRAP_SERVERS:localhost:9092}"),
                ("spring.kafka.consumer.group-id", "${spring.application.name}"),
                ("spring.kafka.consumer.auto-offset-reset", "earliest"),
                ("spring.kafka.consumer.key-deserializer", "org.apache.kafka.common.serialization.StringDeserializer"),
                ("spring.kafka.consumer.value-deserializer", "org.springframework.kafka.support.serializer.JsonDeserializer"),
                ("spring.kafka.consumer.properties.spring.json.trusted.packages", "*"),
                ("spring.kafka.producer.key-serializer", "org.apache.kafka.common.serialization.StringSerializer"),
                ("spring.kafka.producer.value-serializer", "org.springframework.kafka.support.serializer.JsonSerializer"),
                ("spring.kafka.producer.retries", "3"),
                ("spring.kafka.producer.acks", "all"),
                ("spring.kafka.producer.properties.enable.idempotence", "true"),
                ("spring.kafka.listener.concurrency", "${KAFKA_LISTENER_CONCURRENCY:3}"),
            ],
            docker_services: &[
                DockerService {
                    name: "zookeeper",
                    image: "confluentinc/cp-zookeeper:7.6.0",
                    ports: &["2181:2181"],
                    environment: &[
                        ("ZOOKEEPER_CLIENT_PORT", "2181"),
                        ("ZOOKEEPER_TICK_TIME", "2000"),
                    ],
                    volumes: &[],
                    healthcheck: None,
                    depends_on: &[],
                },
                DockerService {
                    name: "kafka",
                    image: "confluentinc/cp-kafka:7.6.0",
                    ports: &["9092:9092", "9101:9101"],
                    environment: &[
                        ("KAFKA_BROKER_ID", "1"),
                        ("KAFKA_ZOOKEEPER_CONNECT", "zookeeper:2181"),
                        ("KAFKA_ADVERTISED_LISTENERS", "PLAINTEXT://localhost:9092"),
                        ("KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR", "1"),
                        ("KAFKA_AUTO_CREATE_TOPICS_ENABLE", "true"),
                    ],
                    volumes: &[],
                    healthcheck: Some("kafka-broker-api-versions --bootstrap-server localhost:9092"),
                    depends_on: &["zookeeper"],
                },
                DockerService {
                    name: "kafka-ui",
                    image: "provectuslabs/kafka-ui:latest",
                    ports: &["8090:8080"],
                    environment: &[
                        ("KAFKA_CLUSTERS_0_NAME", "local"),
                        ("KAFKA_CLUSTERS_0_BOOTSTRAPSERVERS", "kafka:9092"),
                    ],
                    volumes: &[],
                    healthcheck: None,
                    depends_on: &["kafka"],
                },
            ],
            env_vars: &[
                ("KAFKA_BOOTSTRAP_SERVERS", "localhost:9092"),
                ("KAFKA_LISTENER_CONCURRENCY", "3"),
            ],
            java_files: &["KafkaConfig", "KafkaProducerService", "KafkaConsumerService", "KafkaTopicConfig"],
            conflicts: &["rabbitmq", "ibmmq"],
        },

        // ─────────────────────────────────────────────────────────
        // RabbitMQ
        // ─────────────────────────────────────────────────────────
        FeatureSpec {
            key: "rabbitmq",
            name: "RabbitMQ",
            description: "RabbitMQ messaging with Spring AMQP",
            maven_deps: &[
                MavenDep { group_id: "org.springframework.boot", artifact_id: "spring-boot-starter-amqp", version: None, scope: None },
            ],
            requires: &[],
            properties: &[
                ("spring.rabbitmq.host", "${RABBITMQ_HOST:localhost}"),
                ("spring.rabbitmq.port", "${RABBITMQ_PORT:5672}"),
                ("spring.rabbitmq.username", "${RABBITMQ_USER:guest}"),
                ("spring.rabbitmq.password", "${RABBITMQ_PASSWORD:guest}"),
            ],
            docker_services: &[
                DockerService {
                    name: "rabbitmq",
                    image: "rabbitmq:3-management",
                    ports: &["5672:5672", "15672:15672"],
                    environment: &[
                        ("RABBITMQ_DEFAULT_USER", "${RABBITMQ_USER:-guest}"),
                        ("RABBITMQ_DEFAULT_PASS", "${RABBITMQ_PASSWORD:-guest}"),
                    ],
                    volumes: &["rabbitmq_data:/var/lib/rabbitmq"],
                    healthcheck: Some("rabbitmq-diagnostics -q ping"),
                    depends_on: &[],
                },
            ],
            env_vars: &[
                ("RABBITMQ_HOST", "localhost"),
                ("RABBITMQ_PORT", "5672"),
                ("RABBITMQ_USER", "guest"),
                ("RABBITMQ_PASSWORD", "guest"),
            ],
            java_files: &["RabbitMqConfig"],
            conflicts: &["kafka", "ibmmq"],
        },

        // ─────────────────────────────────────────────────────────
        // IBM MQ
        // ─────────────────────────────────────────────────────────
        FeatureSpec {
            key: "ibmmq",
            name: "IBM MQ",
            description: "IBM MQ (JMS) integration",
            maven_deps: &[
                MavenDep { group_id: "com.ibm.mq", artifact_id: "mq-jms-spring-boot-starter", version: Some("3.3.4"), scope: None },
            ],
            requires: &[],
            properties: &[
                ("ibm.mq.queueManager", "${IBM_MQ_QM:QM1}"),
                ("ibm.mq.channel", "${IBM_MQ_CHANNEL:DEV.APP.SVRCONN}"),
                ("ibm.mq.connName", "${IBM_MQ_HOST:localhost}(${IBM_MQ_PORT:1414})"),
                ("ibm.mq.user", "${IBM_MQ_USER:app}"),
                ("ibm.mq.password", "${IBM_MQ_PASSWORD:}"),
            ],
            docker_services: &[
                DockerService {
                    name: "ibmmq",
                    image: "ibmcom/mq:latest",
                    ports: &["1414:1414", "9443:9443"],
                    environment: &[
                        ("LICENSE", "accept"),
                        ("MQ_QMGR_NAME", "${IBM_MQ_QM:-QM1}"),
                    ],
                    volumes: &["ibmmq_data:/mnt/mqm"],
                    healthcheck: None,
                    depends_on: &[],
                },
            ],
            env_vars: &[
                ("IBM_MQ_QM", "QM1"),
                ("IBM_MQ_CHANNEL", "DEV.APP.SVRCONN"),
                ("IBM_MQ_HOST", "localhost"),
                ("IBM_MQ_PORT", "1414"),
                ("IBM_MQ_USER", "app"),
                ("IBM_MQ_PASSWORD", ""),
            ],
            java_files: &["IbmMqConfig"],
            conflicts: &["kafka", "rabbitmq"],
        },

        // ─────────────────────────────────────────────────────────
        // PostgreSQL
        // ─────────────────────────────────────────────────────────
        FeatureSpec {
            key: "postgres",
            name: "PostgreSQL",
            description: "PostgreSQL with Spring Data JPA, Flyway migrations, and connection pooling",
            maven_deps: &[
                MavenDep { group_id: "org.springframework.boot", artifact_id: "spring-boot-starter-data-jpa", version: None, scope: None },
                MavenDep { group_id: "org.postgresql", artifact_id: "postgresql", version: None, scope: None },
                MavenDep { group_id: "org.flywaydb", artifact_id: "flyway-core", version: None, scope: None },
                MavenDep { group_id: "com.zaxxer", artifact_id: "HikariCP", version: None, scope: None },
            ],
            requires: &[],
            properties: &[
                ("spring.datasource.url", "jdbc:postgresql://${DB_HOST:localhost}:${DB_PORT:5432}/${DB_NAME:appdb}"),
                ("spring.datasource.username", "${DB_USER:postgres}"),
                ("spring.datasource.password", "${DB_PASSWORD:postgres}"),
                ("spring.datasource.driver-class-name", "org.postgresql.Driver"),
                ("spring.datasource.hikari.maximum-pool-size", "${DB_POOL_MAX:10}"),
                ("spring.datasource.hikari.minimum-idle", "2"),
                ("spring.datasource.hikari.idle-timeout", "300000"),
                ("spring.datasource.hikari.connection-timeout", "20000"),
                ("spring.jpa.hibernate.ddl-auto", "validate"),
                ("spring.jpa.properties.hibernate.dialect", "org.hibernate.dialect.PostgreSQLDialect"),
                ("spring.jpa.properties.hibernate.show_sql", "false"),
                ("spring.jpa.properties.hibernate.format_sql", "true"),
                ("spring.flyway.enabled", "true"),
                ("spring.flyway.locations", "classpath:db/migration"),
                ("spring.flyway.baseline-on-migrate", "true"),
            ],
            docker_services: &[DockerService {
                name: "postgres",
                image: "postgres:16-alpine",
                ports: &["5432:5432"],
                environment: &[
                    ("POSTGRES_DB", "${DB_NAME:-appdb}"),
                    ("POSTGRES_USER", "${DB_USER:-postgres}"),
                    ("POSTGRES_PASSWORD", "${DB_PASSWORD:-postgres}"),
                ],
                volumes: &["postgres_data:/var/lib/postgresql/data"],
                healthcheck: Some("pg_isready -U ${DB_USER:-postgres}"),
                depends_on: &[],
            }],
            env_vars: &[
                ("DB_HOST", "localhost"),
                ("DB_PORT", "5432"),
                ("DB_NAME", "appdb"),
                ("DB_USER", "postgres"),
                ("DB_PASSWORD", "postgres"),
                ("DB_POOL_MAX", "10"),
            ],
            java_files: &["JpaConfig"],
            conflicts: &["mysql"],
        },

        // ─────────────────────────────────────────────────────────
        // MySQL
        // ─────────────────────────────────────────────────────────
        FeatureSpec {
            key: "mysql",
            name: "MySQL",
            description: "MySQL 8 with Spring Data JPA and Flyway migrations",
            maven_deps: &[
                MavenDep { group_id: "org.springframework.boot", artifact_id: "spring-boot-starter-data-jpa", version: None, scope: None },
                MavenDep { group_id: "com.mysql", artifact_id: "mysql-connector-j", version: None, scope: None },
                MavenDep { group_id: "org.flywaydb", artifact_id: "flyway-mysql", version: None, scope: None },
                MavenDep { group_id: "com.zaxxer", artifact_id: "HikariCP", version: None, scope: None },
            ],
            requires: &[],
            properties: &[
                ("spring.datasource.url", "jdbc:mysql://${DB_HOST:localhost}:${DB_PORT:3306}/${DB_NAME:appdb}?useSSL=false&serverTimezone=UTC"),
                ("spring.datasource.username", "${DB_USER:root}"),
                ("spring.datasource.password", "${DB_PASSWORD:root}"),
                ("spring.datasource.driver-class-name", "com.mysql.cj.jdbc.Driver"),
                ("spring.jpa.hibernate.ddl-auto", "validate"),
                ("spring.jpa.properties.hibernate.dialect", "org.hibernate.dialect.MySQL8Dialect"),
                ("spring.flyway.enabled", "true"),
            ],
            docker_services: &[DockerService {
                name: "mysql",
                image: "mysql:8.3",
                ports: &["3306:3306"],
                environment: &[
                    ("MYSQL_ROOT_PASSWORD", "${DB_PASSWORD:-root}"),
                    ("MYSQL_DATABASE", "${DB_NAME:-appdb}"),
                ],
                volumes: &["mysql_data:/var/lib/mysql"],
                healthcheck: Some("mysqladmin ping -h localhost"),
                depends_on: &[],
            }],
            env_vars: &[
                ("DB_HOST", "localhost"),
                ("DB_PORT", "3306"),
                ("DB_NAME", "appdb"),
                ("DB_USER", "root"),
                ("DB_PASSWORD", "root"),
            ],
            java_files: &["JpaConfig"],
            conflicts: &["postgres"],
        },

        // ─────────────────────────────────────────────────────────
        // MongoDB
        // ─────────────────────────────────────────────────────────
        FeatureSpec {
            key: "mongodb",
            name: "MongoDB",
            description: "MongoDB with Spring Data Reactive MongoDB",
            maven_deps: &[
                MavenDep { group_id: "org.springframework.boot", artifact_id: "spring-boot-starter-data-mongodb", version: None, scope: None },
            ],
            requires: &[],
            properties: &[
                ("spring.data.mongodb.uri", "mongodb://${MONGO_HOST:localhost}:${MONGO_PORT:27017}/${MONGO_DB:appdb}"),
                ("spring.data.mongodb.auto-index-creation", "true"),
            ],
            docker_services: &[DockerService {
                name: "mongodb",
                image: "mongo:7.0",
                ports: &["27017:27017"],
                environment: &[
                    ("MONGO_INITDB_ROOT_USERNAME", "${MONGO_USER:-root}"),
                    ("MONGO_INITDB_ROOT_PASSWORD", "${MONGO_PASSWORD:-root}"),
                    ("MONGO_INITDB_DATABASE", "${MONGO_DB:-appdb}"),
                ],
                volumes: &["mongo_data:/data/db"],
                healthcheck: Some("mongosh --eval \"db.adminCommand('ping')\""),
                depends_on: &[],
            }],
            env_vars: &[
                ("MONGO_HOST", "localhost"),
                ("MONGO_PORT", "27017"),
                ("MONGO_DB", "appdb"),
                ("MONGO_USER", "root"),
                ("MONGO_PASSWORD", "root"),
            ],
            java_files: &["MongoConfig"],
            conflicts: &[],
        },

        // ─────────────────────────────────────────────────────────
        // Security
        // ─────────────────────────────────────────────────────────
        FeatureSpec {
            key: "security",
            name: "Spring Security",
            description: "Spring Security with RBAC, CORS, and CSRF configuration",
            maven_deps: &[
                MavenDep { group_id: "org.springframework.boot", artifact_id: "spring-boot-starter-security", version: None, scope: None },
            ],
            requires: &[],
            properties: &[
                ("spring.security.user.name", "${SECURITY_USER:admin}"),
                ("spring.security.user.password", "${SECURITY_PASSWORD:changeit}"),
            ],
            docker_services: &[],
            env_vars: &[
                ("SECURITY_USER", "admin"),
                ("SECURITY_PASSWORD", "changeit"),
            ],
            java_files: &["SecurityConfig", "UserDetailsServiceImpl"],
            conflicts: &[],
        },

        // ─────────────────────────────────────────────────────────
        // JWT
        // ─────────────────────────────────────────────────────────
        FeatureSpec {
            key: "jwt",
            name: "JWT Authentication",
            description: "Stateless JWT auth with access/refresh token rotation",
            maven_deps: &[
                MavenDep { group_id: "io.jsonwebtoken", artifact_id: "jjwt-api", version: Some("0.12.5"), scope: None },
                MavenDep { group_id: "io.jsonwebtoken", artifact_id: "jjwt-impl", version: Some("0.12.5"), scope: Some("runtime") },
                MavenDep { group_id: "io.jsonwebtoken", artifact_id: "jjwt-jackson", version: Some("0.12.5"), scope: Some("runtime") },
            ],
            requires: &["security"],
            properties: &[
                ("app.jwt.secret", "${JWT_SECRET:change-me-in-production-with-256bit-key}"),
                ("app.jwt.access-token-expiry-ms", "${JWT_ACCESS_EXPIRY_MS:900000}"),
                ("app.jwt.refresh-token-expiry-ms", "${JWT_REFRESH_EXPIRY_MS:604800000}"),
            ],
            docker_services: &[],
            env_vars: &[
                ("JWT_SECRET", "change-me-in-production-with-256bit-key"),
                ("JWT_ACCESS_EXPIRY_MS", "900000"),
                ("JWT_REFRESH_EXPIRY_MS", "604800000"),
            ],
            java_files: &["JwtService", "JwtAuthenticationFilter", "JwtProperties", "AuthController", "TokenResponse"],
            conflicts: &["oauth2"],
        },

        // ─────────────────────────────────────────────────────────
        // OAuth2
        // ─────────────────────────────────────────────────────────
        FeatureSpec {
            key: "oauth2",
            name: "OAuth2 Resource Server",
            description: "OAuth2 / OIDC resource server with JWT validation",
            maven_deps: &[
                MavenDep { group_id: "org.springframework.boot", artifact_id: "spring-boot-starter-oauth2-resource-server", version: None, scope: None },
                MavenDep { group_id: "org.springframework.boot", artifact_id: "spring-boot-starter-oauth2-client", version: None, scope: None },
            ],
            requires: &["security"],
            properties: &[
                ("spring.security.oauth2.resourceserver.jwt.issuer-uri", "${OAUTH2_ISSUER_URI:http://localhost:8180/realms/app}"),
                ("spring.security.oauth2.resourceserver.jwt.jwk-set-uri", "${OAUTH2_JWK_URI:${spring.security.oauth2.resourceserver.jwt.issuer-uri}/protocol/openid-connect/certs}"),
            ],
            docker_services: &[DockerService {
                name: "keycloak",
                image: "quay.io/keycloak/keycloak:24.0",
                ports: &["8180:8080"],
                environment: &[
                    ("KEYCLOAK_ADMIN", "admin"),
                    ("KEYCLOAK_ADMIN_PASSWORD", "admin"),
                ],
                volumes: &[],
                healthcheck: Some("curl -f http://localhost:8080/health/ready"),
                depends_on: &[],
            }],
            env_vars: &[
                ("OAUTH2_ISSUER_URI", "http://localhost:8180/realms/app"),
                ("OAUTH2_JWK_URI", ""),
            ],
            java_files: &["OAuth2SecurityConfig", "JwtConverterConfig"],
            conflicts: &["jwt"],
        },

        // ─────────────────────────────────────────────────────────
        // OpenAPI
        // ─────────────────────────────────────────────────────────
        FeatureSpec {
            key: "openapi",
            name: "OpenAPI / Swagger",
            description: "OpenAPI 3 documentation with Swagger UI",
            maven_deps: &[
                MavenDep { group_id: "org.springdoc", artifact_id: "springdoc-openapi-starter-webmvc-ui", version: Some("2.5.0"), scope: None },
            ],
            requires: &[],
            properties: &[
                ("springdoc.api-docs.path", "/api-docs"),
                ("springdoc.swagger-ui.path", "/swagger-ui.html"),
                ("springdoc.swagger-ui.operations-sorter", "alpha"),
                ("springdoc.swagger-ui.tags-sorter", "alpha"),
                ("springdoc.swagger-ui.try-it-out-enabled", "true"),
            ],
            docker_services: &[],
            env_vars: &[],
            java_files: &["OpenApiConfig"],
            conflicts: &[],
        },

        // ─────────────────────────────────────────────────────────
        // Actuator
        // ─────────────────────────────────────────────────────────
        FeatureSpec {
            key: "actuator",
            name: "Spring Actuator",
            description: "Health checks, metrics, and management endpoints",
            maven_deps: &[
                MavenDep { group_id: "org.springframework.boot", artifact_id: "spring-boot-starter-actuator", version: None, scope: None },
                MavenDep { group_id: "io.micrometer", artifact_id: "micrometer-registry-prometheus", version: None, scope: None },
            ],
            requires: &[],
            properties: &[
                ("management.endpoints.web.exposure.include", "health,info,metrics,prometheus,env"),
                ("management.endpoint.health.show-details", "when-authorized"),
                ("management.endpoint.health.probes.enabled", "true"),
                ("management.health.livenessstate.enabled", "true"),
                ("management.health.readinessstate.enabled", "true"),
                ("management.metrics.distribution.percentiles-histogram.http.server.requests", "true"),
            ],
            docker_services: &[],
            env_vars: &[],
            java_files: &[],
            conflicts: &[],
        },

        // ─────────────────────────────────────────────────────────
        // Distributed Tracing
        // ─────────────────────────────────────────────────────────
        FeatureSpec {
            key: "tracing",
            name: "Distributed Tracing",
            description: "Micrometer Tracing with Zipkin/Jaeger exporter",
            maven_deps: &[
                MavenDep { group_id: "io.micrometer", artifact_id: "micrometer-tracing-bridge-brave", version: None, scope: None },
                MavenDep { group_id: "io.zipkin.reporter2", artifact_id: "zipkin-reporter-brave", version: None, scope: None },
                MavenDep { group_id: "com.github.loki4j", artifact_id: "loki-logback-appender", version: Some("1.5.2"), scope: None },
            ],
            requires: &["actuator"],
            properties: &[
                ("management.tracing.sampling.probability", "${TRACING_SAMPLE_RATE:1.0}"),
                ("management.zipkin.tracing.endpoint", "${ZIPKIN_ENDPOINT:http://localhost:9411/api/v2/spans}"),
                ("logging.pattern.level", "%5p [${spring.application.name:},%X{traceId:-},%X{spanId:-}]"),
            ],
            docker_services: &[DockerService {
                name: "zipkin",
                image: "openzipkin/zipkin:latest",
                ports: &["9411:9411"],
                environment: &[],
                volumes: &[],
                healthcheck: Some("wget -qO- http://localhost:9411/health"),
                depends_on: &[],
            }],
            env_vars: &[
                ("ZIPKIN_ENDPOINT", "http://localhost:9411/api/v2/spans"),
                ("TRACING_SAMPLE_RATE", "1.0"),
            ],
            java_files: &[],
            conflicts: &[],
        },

        // ─────────────────────────────────────────────────────────
        // Elasticsearch
        // ─────────────────────────────────────────────────────────
        FeatureSpec {
            key: "elasticsearch",
            name: "Elasticsearch",
            description: "Spring Data Elasticsearch with Java client",
            maven_deps: &[
                MavenDep { group_id: "org.springframework.boot", artifact_id: "spring-boot-starter-data-elasticsearch", version: None, scope: None },
            ],
            requires: &[],
            properties: &[
                ("spring.elasticsearch.uris", "${ELASTICSEARCH_URIS:http://localhost:9200}"),
                ("spring.elasticsearch.username", "${ELASTICSEARCH_USER:elastic}"),
                ("spring.elasticsearch.password", "${ELASTICSEARCH_PASSWORD:elastic}"),
                ("spring.elasticsearch.socket-timeout", "10s"),
                ("spring.elasticsearch.connection-timeout", "3s"),
            ],
            docker_services: &[DockerService {
                name: "elasticsearch",
                image: "elasticsearch:8.12.0",
                ports: &["9200:9200", "9300:9300"],
                environment: &[
                    ("discovery.type", "single-node"),
                    ("xpack.security.enabled", "false"),
                    ("ES_JAVA_OPTS", "-Xms512m -Xmx512m"),
                ],
                volumes: &["es_data:/usr/share/elasticsearch/data"],
                healthcheck: Some("curl -f http://localhost:9200/_cluster/health"),
                depends_on: &[],
            }],
            env_vars: &[
                ("ELASTICSEARCH_URIS", "http://localhost:9200"),
                ("ELASTICSEARCH_USER", "elastic"),
                ("ELASTICSEARCH_PASSWORD", "elastic"),
            ],
            java_files: &["ElasticsearchConfig"],
            conflicts: &[],
        },

        // ─────────────────────────────────────────────────────────
        // AWS S3
        // ─────────────────────────────────────────────────────────
        FeatureSpec {
            key: "s3",
            name: "AWS S3 / MinIO",
            description: "S3 object storage with MinIO in local dev",
            maven_deps: &[
                MavenDep { group_id: "software.amazon.awssdk", artifact_id: "s3", version: Some("2.25.0"), scope: None },
                MavenDep { group_id: "software.amazon.awssdk", artifact_id: "auth", version: Some("2.25.0"), scope: None },
            ],
            requires: &[],
            properties: &[
                ("app.s3.bucket", "${S3_BUCKET:app-uploads}"),
                ("app.s3.region", "${AWS_REGION:us-east-1}"),
                ("app.s3.endpoint", "${S3_ENDPOINT:}"),
                ("app.s3.path-style-access", "${S3_PATH_STYLE:false}"),
            ],
            docker_services: &[DockerService {
                name: "minio",
                image: "minio/minio:latest",
                ports: &["9000:9000", "9001:9001"],
                environment: &[
                    ("MINIO_ROOT_USER", "${MINIO_USER:-minioadmin}"),
                    ("MINIO_ROOT_PASSWORD", "${MINIO_PASSWORD:-minioadmin}"),
                ],
                volumes: &["minio_data:/data"],
                healthcheck: Some("curl -f http://localhost:9000/minio/health/live"),
                depends_on: &[],
            }],
            env_vars: &[
                ("S3_BUCKET", "app-uploads"),
                ("AWS_REGION", "us-east-1"),
                ("S3_ENDPOINT", "http://localhost:9000"),
                ("S3_PATH_STYLE", "true"),
                ("AWS_ACCESS_KEY_ID", "minioadmin"),
                ("AWS_SECRET_ACCESS_KEY", "minioadmin"),
            ],
            java_files: &["S3Config", "S3Service"],
            conflicts: &[],
        },

        // ─────────────────────────────────────────────────────────
        // Email
        // ─────────────────────────────────────────────────────────
        FeatureSpec {
            key: "email",
            name: "Email (Spring Mail)",
            description: "Email sending with Thymeleaf templates and MailHog dev server",
            maven_deps: &[
                MavenDep { group_id: "org.springframework.boot", artifact_id: "spring-boot-starter-mail", version: None, scope: None },
                MavenDep { group_id: "org.springframework.boot", artifact_id: "spring-boot-starter-thymeleaf", version: None, scope: None },
            ],
            requires: &[],
            properties: &[
                ("spring.mail.host", "${MAIL_HOST:localhost}"),
                ("spring.mail.port", "${MAIL_PORT:1025}"),
                ("spring.mail.username", "${MAIL_USER:}"),
                ("spring.mail.password", "${MAIL_PASSWORD:}"),
                ("spring.mail.properties.mail.smtp.auth", "false"),
                ("spring.mail.properties.mail.smtp.starttls.enable", "false"),
                ("app.mail.from", "${MAIL_FROM:noreply@example.com}"),
            ],
            docker_services: &[DockerService {
                name: "mailhog",
                image: "mailhog/mailhog:latest",
                ports: &["1025:1025", "8025:8025"],
                environment: &[],
                volumes: &[],
                healthcheck: None,
                depends_on: &[],
            }],
            env_vars: &[
                ("MAIL_HOST", "localhost"),
                ("MAIL_PORT", "1025"),
                ("MAIL_USER", ""),
                ("MAIL_PASSWORD", ""),
                ("MAIL_FROM", "noreply@example.com"),
            ],
            java_files: &["EmailConfig", "EmailService"],
            conflicts: &[],
        },

        // ─────────────────────────────────────────────────────────
        // WebSocket
        // ─────────────────────────────────────────────────────────
        FeatureSpec {
            key: "websocket",
            name: "WebSocket",
            description: "STOMP WebSocket with SockJS fallback",
            maven_deps: &[
                MavenDep { group_id: "org.springframework.boot", artifact_id: "spring-boot-starter-websocket", version: None, scope: None },
            ],
            requires: &[],
            properties: &[
                ("app.websocket.allowed-origins", "${WS_ALLOWED_ORIGINS:*}"),
                ("app.websocket.endpoint", "/ws"),
                ("app.websocket.topic-prefix", "/topic"),
                ("app.websocket.app-prefix", "/app"),
            ],
            docker_services: &[],
            env_vars: &[("WS_ALLOWED_ORIGINS", "*")],
            java_files: &["WebSocketConfig", "WebSocketSecurityConfig"],
            conflicts: &[],
        },

        // ─────────────────────────────────────────────────────────
        // Docker
        // ─────────────────────────────────────────────────────────
        FeatureSpec {
            key: "docker",
            name: "Docker",
            description: "Multi-stage Dockerfile and docker-compose.yml",
            maven_deps: &[],
            requires: &[],
            properties: &[],
            docker_services: &[],
            env_vars: &[],
            java_files: &[],
            conflicts: &[],
        },

        // ─────────────────────────────────────────────────────────
        // Kubernetes
        // ─────────────────────────────────────────────────────────
        FeatureSpec {
            key: "kubernetes",
            name: "Kubernetes",
            description: "Kubernetes manifests (Deployment, Service, ConfigMap, HPA)",
            maven_deps: &[],
            requires: &["actuator"],
            properties: &[],
            docker_services: &[],
            env_vars: &[],
            java_files: &[],
            conflicts: &[],
        },
    ]
}

/// Lookup a feature by key
pub fn find_feature(key: &str) -> Option<FeatureSpec> {
    all_features().into_iter().find(|f| f.key == key)
}

/// Resolve a feature set including transitive `requires`, detecting conflicts
pub fn resolve_features(keys: &[String]) -> anyhow::Result<Vec<FeatureSpec>> {
    let all = all_features();
    let mut resolved: IndexMap<String, FeatureSpec> = IndexMap::new();
    let mut queue: Vec<String> = keys.to_vec();

    while let Some(key) = queue.pop() {
        if resolved.contains_key(&key) {
            continue;
        }
        let spec = all
            .iter()
            .find(|f| f.key == key)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Unknown feature: '{}'. Run `springboot-gen features` to see available features.",
                    key
                )
            })?
            .clone();

        // Queue dependencies
        for dep in spec.requires {
            if !resolved.contains_key(*dep) {
                queue.push(dep.to_string());
            }
        }

        resolved.insert(key.clone(), spec);
    }

    // Check conflicts within the resolved set
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
