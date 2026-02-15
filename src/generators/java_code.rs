use anyhow::Result;
use handlebars::Handlebars;
use serde_json::json;
use std::path::Path;

use crate::config::ProjectConfig;
use crate::engine::to_class_name;
use crate::features::FeatureSpec;

pub struct JavaCodeGenerator<'a> {
    config: &'a ProjectConfig,
    features: &'a [FeatureSpec],
    handlebars: Handlebars<'a>,
}

impl<'a> JavaCodeGenerator<'a> {
    pub fn new(config: &'a ProjectConfig, features: &'a [FeatureSpec]) -> Result<Self> {
        let mut handlebars = Handlebars::new();

        // Register templates from organized directory structure
        // Core templates
        handlebars.register_template_string(
            "core/Application",
            include_str!("../../templates/java/core/Application.java.hbs"),
        )?;
        handlebars.register_template_string(
            "core/ApiResponse",
            include_str!("../../templates/java/core/ApiResponse.java.hbs"),
        )?;
        handlebars.register_template_string(
            "core/GlobalExceptionHandler",
            include_str!("../../templates/java/core/GlobalExceptionHandler.java.hbs"),
        )?;
        handlebars.register_template_string(
            "core/HealthController",
            include_str!("../../templates/java/core/HealthController.java.hbs"),
        )?;

        // Redis templates
        handlebars.register_template_string(
            "redis/RedisConfig",
            include_str!("../../templates/java/redis/RedisConfig.java.hbs"),
        )?;
        handlebars.register_template_string(
            "redis/RedisSslConfig",
            include_str!("../../templates/java/redis/RedisSslConfig.java.hbs"),
        )?;
        handlebars.register_template_string(
            "redis/RedisSentinelConfig",
            include_str!("../../templates/java/redis/RedisSentinelConfig.java.hbs"),
        )?;
        handlebars.register_template_string(
            "redis/CacheConfig",
            include_str!("../../templates/java/redis/CacheConfig.java.hbs"),
        )?;

        // Kafka templates
        handlebars.register_template_string(
            "kafka/KafkaConfig",
            include_str!("../../templates/java/kafka/KafkaConfig.java.hbs"),
        )?;
        handlebars.register_template_string(
            "kafka/KafkaTopicConfig",
            include_str!("../../templates/java/kafka/KafkaTopicConfig.java.hbs"),
        )?;
        handlebars.register_template_string(
            "kafka/KafkaProducerService",
            include_str!("../../templates/java/kafka/KafkaProducerService.java.hbs"),
        )?;
        handlebars.register_template_string(
            "kafka/KafkaConsumerService",
            include_str!("../../templates/java/kafka/KafkaConsumerService.java.hbs"),
        )?;

        Ok(Self {
            config,
            features,
            handlebars,
        })
    }

    pub fn generate(&self, out: &Path, package_path: &str, artifact: &str) -> Result<()> {
        let group = &self.config.project.group;
        let package = format!("{}.{}", group, artifact);
        let base = out.join("src/main/java").join(package_path).join(artifact);
        let config_dir = base.join("config");
        let controller_dir = base.join("controller");
        let service_dir = base.join("service");
        let dto_dir = base.join("dto");
        let exception_dir = base.join("exception");

        // Main application class
        self.write_file(
            &base,
            &format!("{}Application.java", to_class_name(artifact)),
            &self.render_main_class(&package, artifact)?,
        )?;

        // Always-present: GlobalExceptionHandler
        self.write_file(
            &exception_dir,
            "GlobalExceptionHandler.java",
            &self.render_exception_handler(&package)?,
        )?;

        // Always: ApiResponse DTO
        self.write_file(
            &dto_dir,
            "ApiResponse.java",
            &self.render_api_response(&package)?,
        )?;

        // ── Feature-specific files ───────────────────────────────────────────
        if self.has("redis") {
            self.write_file(
                &config_dir,
                "RedisConfig.java",
                &self.render_redis_config(&package, false, false),
            )?;
            self.write_file(
                &config_dir,
                "CacheConfig.java",
                &self.render_cache_config(&package),
            )?;
        }

        if self.has("redis-ssl") {
            self.write_file(
                &config_dir,
                "RedisSslConfig.java",
                &self.render_redis_ssl_config(&package),
            )?;
            self.write_file(
                &config_dir,
                "CacheConfig.java",
                &self.render_cache_config(&package),
            )?;
        }

        if self.has("redis-sentinel") {
            self.write_file(
                &config_dir,
                "RedisSentinelConfig.java",
                &self.render_redis_sentinel_config(&package),
            )?;
            self.write_file(
                &config_dir,
                "CacheConfig.java",
                &self.render_cache_config(&package),
            )?;
        }

        if self.has("kafka") {
            self.write_file(
                &config_dir,
                "KafkaConfig.java",
                &self.render_kafka_config(&package),
            )?;
            self.write_file(
                &config_dir,
                "KafkaTopicConfig.java",
                &self.render_kafka_topic_config(&package),
            )?;
            self.write_file(
                &service_dir,
                "KafkaProducerService.java",
                &self.render_kafka_producer(&package),
            )?;
            self.write_file(
                &service_dir,
                "KafkaConsumerService.java",
                &self.render_kafka_consumer(&package),
            )?;
        }

        if self.has("security") && !self.has("jwt") && !self.has("oauth2") {
            self.write_file(
                &config_dir,
                "SecurityConfig.java",
                &self.render_basic_security(&package),
            )?;
        }

        if self.has("jwt") {
            self.write_file(
                &config_dir,
                "SecurityConfig.java",
                &self.render_jwt_security(&package),
            )?;
            self.write_file(
                &config_dir,
                "JwtProperties.java",
                &self.render_jwt_properties(&package),
            )?;
            self.write_file(
                &service_dir,
                "JwtService.java",
                &self.render_jwt_service(&package),
            )?;
            self.write_file(
                &config_dir,
                "JwtAuthenticationFilter.java",
                &self.render_jwt_filter(&package),
            )?;
            self.write_file(
                &controller_dir,
                "AuthController.java",
                &self.render_auth_controller(&package),
            )?;
            self.write_file(
                &dto_dir,
                "TokenResponse.java",
                &self.render_token_response(&package),
            )?;
            self.write_file(
                &dto_dir,
                "LoginRequest.java",
                &self.render_login_request(&package),
            )?;
        }

        if self.has("oauth2") {
            self.write_file(
                &config_dir,
                "SecurityConfig.java",
                &self.render_oauth2_security(&package),
            )?;
        }

        if self.has("openapi") {
            self.write_file(
                &config_dir,
                "OpenApiConfig.java",
                &self.render_openapi_config(&package),
            )?;
        }

        if self.has("s3") {
            self.write_file(
                &config_dir,
                "S3Config.java",
                &self.render_s3_config(&package),
            )?;
            self.write_file(
                &service_dir,
                "S3Service.java",
                &self.render_s3_service(&package),
            )?;
        }

        if self.has("email") {
            self.write_file(
                &config_dir,
                "EmailConfig.java",
                &self.render_email_config(&package),
            )?;
            self.write_file(
                &service_dir,
                "EmailService.java",
                &self.render_email_service(&package),
            )?;
        }

        if self.has("websocket") {
            self.write_file(
                &config_dir,
                "WebSocketConfig.java",
                &self.render_websocket_config(&package),
            )?;
        }

        if self.has("mongodb") {
            self.write_file(
                &config_dir,
                "MongoConfig.java",
                &self.render_mongo_config(&package),
            )?;
        }

        if self.has("postgres") || self.has("mysql") {
            self.write_file(
                &config_dir,
                "JpaConfig.java",
                &self.render_jpa_config(&package),
            )?;
        }

        // Generate a sample HealthController always
        self.write_file(
            &controller_dir,
            "HealthController.java",
            &self.render_health_controller(&package)?,
        )?;

        Ok(())
    }

    fn write_file(&self, dir: &Path, filename: &str, content: &str) -> Result<()> {
        std::fs::create_dir_all(dir)?;
        let path = dir.join(filename);
        if !path.exists() {
            std::fs::write(&path, content)?;
        }
        Ok(())
    }

    fn has(&self, key: &str) -> bool {
        self.features.iter().any(|f| f.key == key)
    }

    // ── Java file templates ───────────────────────────────────────────────────

    fn render_main_class(&self, pkg: &str, artifact: &str) -> Result<String> {
        let class_name = to_class_name(artifact);
        let data = json!({
            "package": pkg,
            "className": class_name
        });
        Ok(self.handlebars.render("core/Application", &data)?)
    }

    fn render_api_response(&self, pkg: &str) -> Result<String> {
        let data = json!({
            "package": pkg
        });
        Ok(self.handlebars.render("core/ApiResponse", &data)?)
    }

    fn render_exception_handler(&self, pkg: &str) -> Result<String> {
        let data = json!({
            "package": pkg
        });
        Ok(self.handlebars.render("core/GlobalExceptionHandler", &data)?)
    }

    fn render_health_controller(&self, pkg: &str) -> Result<String> {
        let data = json!({
            "package": pkg
        });
        Ok(self.handlebars.render("core/HealthController", &data)?)
    }

    fn render_redis_config(&self, pkg: &str, _ssl: bool, _sentinel: bool) -> String {
        format!(
            r#"package {pkg}.config;

import com.fasterxml.jackson.annotation.JsonAutoDetect;
import com.fasterxml.jackson.annotation.PropertyAccessor;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.jsontype.impl.LaissezFaireSubTypeValidator;
import org.apache.commons.pool2.impl.GenericObjectPoolConfig;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;
import org.springframework.data.redis.connection.RedisConnectionFactory;
import org.springframework.data.redis.connection.RedisStandaloneConfiguration;
import org.springframework.data.redis.connection.lettuce.LettuceConnectionFactory;
import org.springframework.data.redis.connection.lettuce.LettucePoolingClientConfiguration;
import org.springframework.data.redis.core.RedisTemplate;
import org.springframework.data.redis.serializer.Jackson2JsonRedisSerializer;
import org.springframework.data.redis.serializer.StringRedisSerializer;

import java.time.Duration;

/**
 * Redis configuration — Standalone mode.
 * Connection pooling via Lettuce + Apache Commons Pool2.
 */
@Configuration
public class RedisConfig {{

    @Value("${{spring.data.redis.host}}")
    private String host;

    @Value("${{spring.data.redis.port}}")
    private int port;

    @Value("${{spring.data.redis.password:}}")
    private String password;

    @Value("${{spring.data.redis.database:0}}")
    private int database;

    @Bean
    public RedisConnectionFactory redisConnectionFactory() {{
        var standaloneConfig = new RedisStandaloneConfiguration(host, port);
        if (password != null && !password.isBlank()) {{
            standaloneConfig.setPassword(password);
        }}
        standaloneConfig.setDatabase(database);

        var poolConfig = new GenericObjectPoolConfig<Object>();
        poolConfig.setMaxTotal(8);
        poolConfig.setMaxIdle(8);
        poolConfig.setMinIdle(0);
        poolConfig.setTestOnBorrow(true);
        poolConfig.setTestWhileIdle(true);

        var lettucePooling = LettucePoolingClientConfiguration.builder()
            .commandTimeout(Duration.ofMillis(2000))
            .poolConfig(poolConfig)
            .build();

        return new LettuceConnectionFactory(standaloneConfig, lettucePooling);
    }}

    @Bean
    public RedisTemplate<String, Object> redisTemplate(RedisConnectionFactory factory) {{
        var template = new RedisTemplate<String, Object>();
        template.setConnectionFactory(factory);

        var objectMapper = new ObjectMapper();
        objectMapper.setVisibility(PropertyAccessor.ALL, JsonAutoDetect.Visibility.ANY);
        objectMapper.activateDefaultTyping(
            LaissezFaireSubTypeValidator.instance,
            ObjectMapper.DefaultTyping.NON_FINAL
        );

        var jsonSerializer = new Jackson2JsonRedisSerializer<>(objectMapper, Object.class);
        var stringSerializer = new StringRedisSerializer();

        template.setKeySerializer(stringSerializer);
        template.setHashKeySerializer(stringSerializer);
        template.setValueSerializer(jsonSerializer);
        template.setHashValueSerializer(jsonSerializer);
        template.afterPropertiesSet();

        return template;
    }}
}}
"#,
            pkg = pkg
        )
    }

    fn render_redis_ssl_config(&self, pkg: &str) -> String {
        format!(
            r#"package {pkg}.config;

import io.lettuce.core.ClientOptions;
import io.lettuce.core.SslOptions;
import org.apache.commons.pool2.impl.GenericObjectPoolConfig;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;
import org.springframework.core.io.Resource;
import org.springframework.data.redis.connection.RedisConnectionFactory;
import org.springframework.data.redis.connection.RedisStandaloneConfiguration;
import org.springframework.data.redis.connection.lettuce.LettuceClientConfiguration;
import org.springframework.data.redis.connection.lettuce.LettuceConnectionFactory;
import org.springframework.data.redis.connection.lettuce.LettucePoolingClientConfiguration;
import org.springframework.data.redis.core.RedisTemplate;
import org.springframework.data.redis.serializer.Jackson2JsonRedisSerializer;
import org.springframework.data.redis.serializer.StringRedisSerializer;

import java.io.File;
import java.time.Duration;

/**
 * Redis configuration — SSL/TLS mode.
 *
 * <p>Requires a PKCS12 keystore and truststore on the classpath under {{@code ssl/}}.
 * To generate self-signed certs for development, run:
 * <pre>
 *   keytool -genkeypair -alias redis-client -keyalg RSA -keysize 2048 \
 *     -storetype PKCS12 -keystore src/main/resources/ssl/redis-client.p12 \
 *     -validity 365 -storepass changeit
 * </pre>
 */
@Configuration
public class RedisSslConfig {{

    @Value("${{spring.data.redis.host}}")
    private String host;

    @Value("${{spring.data.redis.port}}")
    private int port;

    @Value("${{spring.data.redis.password:}}")
    private String password;

    @Value("${{spring.ssl.bundle.jks.redis-ssl.keystore.location}}")
    private Resource keystoreLocation;

    @Value("${{spring.ssl.bundle.jks.redis-ssl.keystore.password}}")
    private String keystorePassword;

    @Value("${{spring.ssl.bundle.jks.redis-ssl.truststore.location}}")
    private Resource truststoreLocation;

    @Value("${{spring.ssl.bundle.jks.redis-ssl.truststore.password}}")
    private String truststorePassword;

    @Bean
    public RedisConnectionFactory redisConnectionFactory() throws Exception {{
        var sslOptions = SslOptions.builder()
            .keystore(keystoreLocation.getFile(), keystorePassword.toCharArray())
            .truststore(truststoreLocation.getFile(), truststorePassword.toCharArray())
            .build();

        var clientOptions = ClientOptions.builder()
            .sslOptions(sslOptions)
            .build();

        var poolConfig = new GenericObjectPoolConfig<Object>();
        poolConfig.setMaxTotal(8);
        poolConfig.setMaxIdle(8);
        poolConfig.setMinIdle(0);

        var lettuceConfig = LettucePoolingClientConfiguration.builder()
            .useSsl()
            .clientOptions(clientOptions)
            .commandTimeout(Duration.ofMillis(2000))
            .poolConfig(poolConfig)
            .build();

        var standaloneConfig = new RedisStandaloneConfiguration(host, port);
        if (password != null && !password.isBlank()) {{
            standaloneConfig.setPassword(password);
        }}

        return new LettuceConnectionFactory(standaloneConfig, lettuceConfig);
    }}

    @Bean
    public RedisTemplate<String, Object> redisTemplate(RedisConnectionFactory factory) {{
        var template = new RedisTemplate<String, Object>();
        template.setConnectionFactory(factory);
        template.setKeySerializer(new StringRedisSerializer());
        template.setValueSerializer(new Jackson2JsonRedisSerializer<>(Object.class));
        template.setHashKeySerializer(new StringRedisSerializer());
        template.setHashValueSerializer(new Jackson2JsonRedisSerializer<>(Object.class));
        template.afterPropertiesSet();
        return template;
    }}
}}
"#,
            pkg = pkg
        )
    }

    fn render_redis_sentinel_config(&self, pkg: &str) -> String {
        format!(
            r#"package {pkg}.config;

import org.apache.commons.pool2.impl.GenericObjectPoolConfig;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;
import org.springframework.data.redis.connection.RedisConnectionFactory;
import org.springframework.data.redis.connection.RedisSentinelConfiguration;
import org.springframework.data.redis.connection.lettuce.LettuceConnectionFactory;
import org.springframework.data.redis.connection.lettuce.LettucePoolingClientConfiguration;
import org.springframework.data.redis.core.RedisTemplate;
import org.springframework.data.redis.serializer.Jackson2JsonRedisSerializer;
import org.springframework.data.redis.serializer.StringRedisSerializer;

import java.time.Duration;
import java.util.Arrays;
import java.util.List;

/**
 * Redis configuration — Sentinel HA mode.
 *
 * <p>Sentinel provides automatic failover. At least 3 sentinels should be
 * running in production. The master name must match your Redis Sentinel config.
 *
 * <p>Environment variables:
 * <ul>
 *   <li>REDIS_SENTINEL_MASTER  — sentinel master name (default: mymaster)</li>
 *   <li>REDIS_SENTINEL_NODES   — comma-separated host:port list</li>
 *   <li>REDIS_SENTINEL_PASSWORD — sentinel auth password (if configured)</li>
 *   <li>REDIS_PASSWORD          — Redis data node password</li>
 * </ul>
 */
@Configuration
public class RedisSentinelConfig {{

    @Value("${{spring.data.redis.sentinel.master}}")
    private String sentinelMaster;

    @Value("${{spring.data.redis.sentinel.nodes}}")
    private String sentinelNodes;

    @Value("${{spring.data.redis.sentinel.password:}}")
    private String sentinelPassword;

    @Value("${{spring.data.redis.password:}}")
    private String redisPassword;

    @Value("${{spring.data.redis.database:0}}")
    private int database;

    @Bean
    public RedisConnectionFactory redisConnectionFactory() {{
        var sentinelConfig = new RedisSentinelConfiguration();
        sentinelConfig.setMaster(sentinelMaster);

        // Parse "host:port,host:port" sentinel nodes
        Arrays.stream(sentinelNodes.split(","))
            .map(String::trim)
            .filter(node -> !node.isBlank())
            .forEach(node -> {{
                String[] parts = node.split(":");
                sentinelConfig.sentinel(parts[0], Integer.parseInt(parts[1]));
            }});

        if (sentinelPassword != null && !sentinelPassword.isBlank()) {{
            sentinelConfig.setSentinelPassword(sentinelPassword);
        }}
        if (redisPassword != null && !redisPassword.isBlank()) {{
            sentinelConfig.setPassword(redisPassword);
        }}
        sentinelConfig.setDatabase(database);

        var poolConfig = new GenericObjectPoolConfig<Object>();
        poolConfig.setMaxTotal(16);
        poolConfig.setMaxIdle(8);
        poolConfig.setMinIdle(2);
        poolConfig.setBlockWhenExhausted(true);

        var lettuceConfig = LettucePoolingClientConfiguration.builder()
            .commandTimeout(Duration.ofMillis(2000))
            .poolConfig(poolConfig)
            .build();

        return new LettuceConnectionFactory(sentinelConfig, lettuceConfig);
    }}

    @Bean
    public RedisTemplate<String, Object> redisTemplate(RedisConnectionFactory factory) {{
        var template = new RedisTemplate<String, Object>();
        template.setConnectionFactory(factory);
        template.setKeySerializer(new StringRedisSerializer());
        template.setValueSerializer(new Jackson2JsonRedisSerializer<>(Object.class));
        template.setHashKeySerializer(new StringRedisSerializer());
        template.setHashValueSerializer(new Jackson2JsonRedisSerializer<>(Object.class));
        template.afterPropertiesSet();
        return template;
    }}
}}
"#,
            pkg = pkg
        )
    }

    fn render_cache_config(&self, pkg: &str) -> String {
        format!(
            r#"package {pkg}.config;

import org.springframework.cache.CacheManager;
import org.springframework.cache.annotation.EnableCaching;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;
import org.springframework.data.redis.cache.RedisCacheConfiguration;
import org.springframework.data.redis.cache.RedisCacheManager;
import org.springframework.data.redis.connection.RedisConnectionFactory;
import org.springframework.data.redis.serializer.GenericJackson2JsonRedisSerializer;
import org.springframework.data.redis.serializer.RedisSerializationContext;
import org.springframework.data.redis.serializer.StringRedisSerializer;

import java.time.Duration;
import java.util.HashMap;
import java.util.Map;

/**
 * Spring Cache Manager backed by Redis.
 *
 * <p>Usage: annotate service methods with {{@code @Cacheable("cacheName")}}.
 * Cache TTLs are configured per cache name here.
 */
@Configuration
@EnableCaching
public class CacheConfig {{

    /** Default TTL for all caches unless overridden */
    private static final Duration DEFAULT_TTL = Duration.ofMinutes(10);

    @Bean
    public CacheManager cacheManager(RedisConnectionFactory factory) {{
        var defaultConfig = RedisCacheConfiguration.defaultCacheConfig()
            .entryTtl(DEFAULT_TTL)
            .disableCachingNullValues()
            .serializeKeysWith(
                RedisSerializationContext.SerializationPair.fromSerializer(new StringRedisSerializer()))
            .serializeValuesWith(
                RedisSerializationContext.SerializationPair.fromSerializer(new GenericJackson2JsonRedisSerializer()));

        // Per-cache TTL overrides
        Map<String, RedisCacheConfiguration> cacheConfigs = new HashMap<>();
        cacheConfigs.put("users",         defaultConfig.entryTtl(Duration.ofMinutes(30)));
        cacheConfigs.put("sessions",      defaultConfig.entryTtl(Duration.ofHours(1)));
        cacheConfigs.put("rate-limits",   defaultConfig.entryTtl(Duration.ofSeconds(60)));

        return RedisCacheManager.builder(factory)
            .cacheDefaults(defaultConfig)
            .withInitialCacheConfigurations(cacheConfigs)
            .transactionAware()
            .build();
    }}
}}
"#,
            pkg = pkg
        )
    }

    fn render_kafka_config(&self, pkg: &str) -> String {
        format!(
            r#"package {pkg}.config;

import org.apache.kafka.clients.admin.NewTopic;
import org.springframework.context.annotation.Configuration;
import org.springframework.kafka.annotation.EnableKafka;
import org.springframework.kafka.config.ConcurrentKafkaListenerContainerFactory;
import org.springframework.kafka.core.ConsumerFactory;
import org.springframework.kafka.listener.DefaultErrorHandler;
import org.springframework.kafka.support.ExponentialBackOffWithMaxRetries;
import org.springframework.context.annotation.Bean;

/**
 * Kafka listener container configuration.
 * Topics are defined in KafkaTopicConfig.
 */
@Configuration
@EnableKafka
public class KafkaConfig {{

    @Bean
    public ConcurrentKafkaListenerContainerFactory<String, Object> kafkaListenerContainerFactory(
            ConsumerFactory<String, Object> consumerFactory) {{

        var factory = new ConcurrentKafkaListenerContainerFactory<String, Object>();
        factory.setConsumerFactory(consumerFactory);

        // Exponential backoff: 1s → 2s → 4s → 8s (max 3 retries before DLQ)
        var backOff = new ExponentialBackOffWithMaxRetries(3);
        backOff.setInitialInterval(1_000L);
        backOff.setMultiplier(2.0);
        backOff.setMaxInterval(8_000L);

        var errorHandler = new DefaultErrorHandler(backOff);
        factory.setCommonErrorHandler(errorHandler);

        return factory;
    }}
}}
"#,
            pkg = pkg
        )
    }

    fn render_kafka_topic_config(&self, pkg: &str) -> String {
        format!(
            r#"package {pkg}.config;

import org.apache.kafka.clients.admin.NewTopic;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;
import org.springframework.kafka.config.TopicBuilder;

/**
 * Kafka topic definitions.
 * Annotate beans as @Bean to auto-create topics on startup.
 *
 * <p><strong>Add your topics here.</strong>
 */
@Configuration
public class KafkaTopicConfig {{

    // ── Add your topics below ────────────────────────────────────────────────

    @Bean
    public NewTopic exampleTopic() {{
        return TopicBuilder.name("example-events")
            .partitions(3)
            .replicas(1)
            .compact()
            .build();
    }}

    @Bean
    public NewTopic exampleDlqTopic() {{
        return TopicBuilder.name("example-events.DLT")
            .partitions(1)
            .replicas(1)
            .build();
    }}
}}
"#,
            pkg = pkg
        )
    }

    fn render_kafka_producer(&self, pkg: &str) -> String {
        format!(
            r#"package {pkg}.service;

import lombok.RequiredArgsConstructor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.kafka.core.KafkaTemplate;
import org.springframework.kafka.support.SendResult;
import org.springframework.stereotype.Service;

import java.util.concurrent.CompletableFuture;

/**
 * Generic Kafka producer service.
 * Inject this into your domain services to publish events.
 */
@Slf4j
@Service
@RequiredArgsConstructor
public class KafkaProducerService {{

    private final KafkaTemplate<String, Object> kafkaTemplate;

    /**
     * Send a message to the given topic with a key for partitioning.
     */
    public <T> CompletableFuture<SendResult<String, T>> send(String topic, String key, T payload) {{
        log.debug("Publishing to topic={{}} key={{}} payload={{}}",  topic, key, payload);

        @SuppressWarnings("unchecked")
        CompletableFuture<SendResult<String, T>> future =
            (CompletableFuture<SendResult<String, T>>) (Object)
            kafkaTemplate.send(topic, key, payload);

        return future.whenComplete((result, ex) -> {{
            if (ex != null) {{
                log.error("Failed to publish to topic={{}} key={{}}: {{}}",  topic, key, ex.getMessage(), ex);
            }} else {{
                log.info("Published to topic={{}} partition={{}} offset={{}}",
                    result.getRecordMetadata().topic(),
                    result.getRecordMetadata().partition(),
                    result.getRecordMetadata().offset());
            }}
        }});
    }}

    /** Send without an explicit key (Kafka will assign round-robin partition). */
    public <T> CompletableFuture<SendResult<String, T>> send(String topic, T payload) {{
        return send(topic, null, payload);
    }}
}}
"#,
            pkg = pkg
        )
    }

    fn render_kafka_consumer(&self, pkg: &str) -> String {
        format!(
            r#"package {pkg}.service;

import lombok.extern.slf4j.Slf4j;
import org.springframework.kafka.annotation.KafkaListener;
import org.springframework.kafka.annotation.RetryableTopic;
import org.springframework.retry.annotation.Backoff;
import org.springframework.stereotype.Service;

/**
 * Example Kafka consumer.
 * Replace or extend with your actual domain event consumers.
 *
 * <p>@RetryableTopic configures retry topics automatically:
 * example-events → example-events-retry-0 → example-events-retry-1 → example-events.DLT
 */
@Slf4j
@Service
public class KafkaConsumerService {{

    @RetryableTopic(
        attempts = "4",
        backoff = @Backoff(delay = 1000, multiplier = 2, maxDelay = 8000),
        dltTopicSuffix = ".DLT"
    )
    @KafkaListener(topics = "example-events", groupId = "${{spring.kafka.consumer.group-id}}")
    public void consumeExampleEvent(String message) {{
        log.info("Consumed example event: {{}}", message);
        // TODO: implement your business logic here
    }}
}}
"#,
            pkg = pkg
        )
    }

    fn render_basic_security(&self, pkg: &str) -> String {
        format!(
            r#"package {pkg}.config;

import lombok.RequiredArgsConstructor;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;
import org.springframework.security.config.annotation.method.configuration.EnableMethodSecurity;
import org.springframework.security.config.annotation.web.builders.HttpSecurity;
import org.springframework.security.config.annotation.web.configuration.EnableWebSecurity;
import org.springframework.security.config.annotation.web.configurers.AbstractHttpConfigurer;
import org.springframework.security.crypto.bcrypt.BCryptPasswordEncoder;
import org.springframework.security.crypto.password.PasswordEncoder;
import org.springframework.security.web.SecurityFilterChain;
import org.springframework.web.cors.CorsConfiguration;
import org.springframework.web.cors.CorsConfigurationSource;
import org.springframework.web.cors.UrlBasedCorsConfigurationSource;

import java.util.List;

/**
 * Spring Security — HTTP Basic configuration.
 * Replace with JwtSecurityConfig or OAuth2SecurityConfig as needed.
 */
@Configuration
@EnableWebSecurity
@EnableMethodSecurity
@RequiredArgsConstructor
public class SecurityConfig {{

    private static final String[] PUBLIC_PATHS = {{
        "/actuator/health/**",
        "/actuator/info",
        "/api-docs/**",
        "/swagger-ui/**",
        "/swagger-ui.html"
    }};

    @Bean
    public SecurityFilterChain securityFilterChain(HttpSecurity http) throws Exception {{
        http
            .csrf(AbstractHttpConfigurer::disable)
            .cors(cors -> cors.configurationSource(corsConfigurationSource()))
            .authorizeHttpRequests(auth -> auth
                .requestMatchers(PUBLIC_PATHS).permitAll()
                .anyRequest().authenticated()
            )
            .httpBasic(basic -> {{}});

        return http.build();
    }}

    @Bean
    public PasswordEncoder passwordEncoder() {{
        return new BCryptPasswordEncoder();
    }}

    @Bean
    public CorsConfigurationSource corsConfigurationSource() {{
        var config = new CorsConfiguration();
        config.setAllowedOriginPatterns(List.of("*"));
        config.setAllowedMethods(List.of("GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"));
        config.setAllowedHeaders(List.of("*"));
        config.setAllowCredentials(true);
        config.setMaxAge(3600L);

        var source = new UrlBasedCorsConfigurationSource();
        source.registerCorsConfiguration("/**", config);
        return source;
    }}
}}
"#,
            pkg = pkg
        )
    }

    fn render_jwt_security(&self, pkg: &str) -> String {
        format!(
            r#"package {pkg}.config;

import lombok.RequiredArgsConstructor;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;
import org.springframework.security.authentication.AuthenticationManager;
import org.springframework.security.config.annotation.authentication.configuration.AuthenticationConfiguration;
import org.springframework.security.config.annotation.method.configuration.EnableMethodSecurity;
import org.springframework.security.config.annotation.web.builders.HttpSecurity;
import org.springframework.security.config.annotation.web.configuration.EnableWebSecurity;
import org.springframework.security.config.annotation.web.configurers.AbstractHttpConfigurer;
import org.springframework.security.config.http.SessionCreationPolicy;
import org.springframework.security.crypto.bcrypt.BCryptPasswordEncoder;
import org.springframework.security.crypto.password.PasswordEncoder;
import org.springframework.security.web.SecurityFilterChain;
import org.springframework.security.web.authentication.UsernamePasswordAuthenticationFilter;
import org.springframework.web.cors.CorsConfiguration;
import org.springframework.web.cors.CorsConfigurationSource;
import org.springframework.web.cors.UrlBasedCorsConfigurationSource;

import java.util.List;

/**
 * Spring Security — Stateless JWT configuration.
 * Session management is STATELESS; all state lives in the JWT.
 */
@Configuration
@EnableWebSecurity
@EnableMethodSecurity
@RequiredArgsConstructor
public class SecurityConfig {{

    private final JwtAuthenticationFilter jwtAuthFilter;

    private static final String[] PUBLIC_PATHS = {{
        "/api/v1/auth/**",
        "/actuator/health/**",
        "/api-docs/**",
        "/swagger-ui/**",
        "/swagger-ui.html"
    }};

    @Bean
    public SecurityFilterChain securityFilterChain(HttpSecurity http) throws Exception {{
        http
            .csrf(AbstractHttpConfigurer::disable)
            .cors(cors -> cors.configurationSource(corsConfigurationSource()))
            .sessionManagement(session ->
                session.sessionCreationPolicy(SessionCreationPolicy.STATELESS))
            .authorizeHttpRequests(auth -> auth
                .requestMatchers(PUBLIC_PATHS).permitAll()
                .anyRequest().authenticated()
            )
            .addFilterBefore(jwtAuthFilter, UsernamePasswordAuthenticationFilter.class);

        return http.build();
    }}

    @Bean
    public AuthenticationManager authenticationManager(AuthenticationConfiguration config)
            throws Exception {{
        return config.getAuthenticationManager();
    }}

    @Bean
    public PasswordEncoder passwordEncoder() {{
        return new BCryptPasswordEncoder();
    }}

    @Bean
    public CorsConfigurationSource corsConfigurationSource() {{
        var config = new CorsConfiguration();
        config.setAllowedOriginPatterns(List.of("*"));
        config.setAllowedMethods(List.of("GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"));
        config.setAllowedHeaders(List.of("*"));
        config.setAllowCredentials(true);
        var source = new UrlBasedCorsConfigurationSource();
        source.registerCorsConfiguration("/**", config);
        return source;
    }}
}}
"#,
            pkg = pkg
        )
    }

    fn render_oauth2_security(&self, pkg: &str) -> String {
        format!(
            r#"package {pkg}.config;

import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;
import org.springframework.security.config.annotation.method.configuration.EnableMethodSecurity;
import org.springframework.security.config.annotation.web.builders.HttpSecurity;
import org.springframework.security.config.annotation.web.configuration.EnableWebSecurity;
import org.springframework.security.config.annotation.web.configurers.AbstractHttpConfigurer;
import org.springframework.security.config.http.SessionCreationPolicy;
import org.springframework.security.web.SecurityFilterChain;

/**
 * Spring Security — OAuth2 Resource Server (JWT validation via JWKS).
 * Tokens are validated against the issuer's public keys automatically.
 */
@Configuration
@EnableWebSecurity
@EnableMethodSecurity
public class SecurityConfig {{

    @Bean
    public SecurityFilterChain securityFilterChain(HttpSecurity http) throws Exception {{
        http
            .csrf(AbstractHttpConfigurer::disable)
            .sessionManagement(session ->
                session.sessionCreationPolicy(SessionCreationPolicy.STATELESS))
            .authorizeHttpRequests(auth -> auth
                .requestMatchers("/actuator/health/**", "/api-docs/**", "/swagger-ui/**").permitAll()
                .anyRequest().authenticated()
            )
            .oauth2ResourceServer(oauth2 ->
                oauth2.jwt(jwt -> {{}}));  // issuer-uri configured in application.yml

        return http.build();
    }}
}}
"#,
            pkg = pkg
        )
    }

    fn render_jwt_properties(&self, pkg: &str) -> String {
        format!(
            r#"package {pkg}.config;

import lombok.Data;
import org.springframework.boot.context.properties.ConfigurationProperties;
import org.springframework.stereotype.Component;

/**
 * JWT configuration properties.
 * Bound to {{@code app.jwt.*}} in application.yml.
 */
@Data
@Component
@ConfigurationProperties(prefix = "app.jwt")
public class JwtProperties {{
    private String secret;
    private long accessTokenExpiryMs = 900_000;   // 15 minutes
    private long refreshTokenExpiryMs = 604_800_000; // 7 days
}}
"#,
            pkg = pkg
        )
    }

    fn render_jwt_service(&self, pkg: &str) -> String {
        format!(
            r#"package {pkg}.service;

import io.jsonwebtoken.Claims;
import io.jsonwebtoken.Jwts;
import io.jsonwebtoken.io.Decoders;
import io.jsonwebtoken.security.Keys;
import lombok.RequiredArgsConstructor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.security.core.userdetails.UserDetails;
import org.springframework.stereotype.Service;
import {pkg}.config.JwtProperties;

import javax.crypto.SecretKey;
import java.util.Date;
import java.util.HashMap;
import java.util.Map;
import java.util.function.Function;

/**
 * JWT token generation and validation service.
 * Uses HMAC-SHA256 signing with the configured secret.
 *
 * <p><strong>Security note:</strong> Use a random 256-bit key in production.
 * Generate one with: {{@code openssl rand -base64 32}}
 */
@Slf4j
@Service
@RequiredArgsConstructor
public class JwtService {{

    private final JwtProperties jwtProperties;

    public String extractUsername(String token) {{
        return extractClaim(token, Claims::getSubject);
    }}

    public String generateAccessToken(UserDetails userDetails) {{
        return generateAccessToken(new HashMap<>(), userDetails);
    }}

    public String generateAccessToken(Map<String, Object> extraClaims, UserDetails userDetails) {{
        return buildToken(extraClaims, userDetails, jwtProperties.getAccessTokenExpiryMs());
    }}

    public String generateRefreshToken(UserDetails userDetails) {{
        return buildToken(new HashMap<>(), userDetails, jwtProperties.getRefreshTokenExpiryMs());
    }}

    public boolean isTokenValid(String token, UserDetails userDetails) {{
        final String username = extractUsername(token);
        return username.equals(userDetails.getUsername()) && !isTokenExpired(token);
    }}

    private <T> T extractClaim(String token, Function<Claims, T> claimsResolver) {{
        return claimsResolver.apply(extractAllClaims(token));
    }}

    private String buildToken(Map<String, Object> extraClaims, UserDetails user, long expiryMs) {{
        return Jwts.builder()
            .claims(extraClaims)
            .subject(user.getUsername())
            .issuedAt(new Date(System.currentTimeMillis()))
            .expiration(new Date(System.currentTimeMillis() + expiryMs))
            .signWith(getSigningKey())
            .compact();
    }}

    private boolean isTokenExpired(String token) {{
        return extractClaim(token, Claims::getExpiration).before(new Date());
    }}

    private Claims extractAllClaims(String token) {{
        return Jwts.parser()
            .verifyWith(getSigningKey())
            .build()
            .parseSignedClaims(token)
            .getPayload();
    }}

    private SecretKey getSigningKey() {{
        byte[] keyBytes = Decoders.BASE64.decode(jwtProperties.getSecret());
        return Keys.hmacShaKeyFor(keyBytes);
    }}
}}
"#,
            pkg = pkg
        )
    }

    fn render_jwt_filter(&self, pkg: &str) -> String {
        format!(
            r#"package {pkg}.config;

import jakarta.servlet.FilterChain;
import jakarta.servlet.ServletException;
import jakarta.servlet.http.HttpServletRequest;
import jakarta.servlet.http.HttpServletResponse;
import lombok.RequiredArgsConstructor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.lang.NonNull;
import org.springframework.security.authentication.UsernamePasswordAuthenticationToken;
import org.springframework.security.core.context.SecurityContextHolder;
import org.springframework.security.core.userdetails.UserDetailsService;
import org.springframework.security.web.authentication.WebAuthenticationDetailsSource;
import org.springframework.stereotype.Component;
import org.springframework.web.filter.OncePerRequestFilter;
import {pkg}.service.JwtService;

import java.io.IOException;

/**
 * JWT authentication filter — extracts and validates the Bearer token.
 * Runs once per request, before Spring's UsernamePasswordAuthenticationFilter.
 */
@Slf4j
@Component
@RequiredArgsConstructor
public class JwtAuthenticationFilter extends OncePerRequestFilter {{

    private static final String BEARER_PREFIX = "Bearer ";

    private final JwtService jwtService;
    private final UserDetailsService userDetailsService;

    @Override
    protected void doFilterInternal(
            @NonNull HttpServletRequest request,
            @NonNull HttpServletResponse response,
            @NonNull FilterChain filterChain) throws ServletException, IOException {{

        final String authHeader = request.getHeader("Authorization");

        if (authHeader == null || !authHeader.startsWith(BEARER_PREFIX)) {{
            filterChain.doFilter(request, response);
            return;
        }}

        final String jwt = authHeader.substring(BEARER_PREFIX.length());

        try {{
            final String username = jwtService.extractUsername(jwt);
            if (username != null && SecurityContextHolder.getContext().getAuthentication() == null) {{
                var userDetails = userDetailsService.loadUserByUsername(username);
                if (jwtService.isTokenValid(jwt, userDetails)) {{
                    var authToken = new UsernamePasswordAuthenticationToken(
                        userDetails, null, userDetails.getAuthorities());
                    authToken.setDetails(new WebAuthenticationDetailsSource().buildDetails(request));
                    SecurityContextHolder.getContext().setAuthentication(authToken);
                }}
            }}
        }} catch (Exception ex) {{
            log.debug("JWT validation failed: {{}}", ex.getMessage());
        }}

        filterChain.doFilter(request, response);
    }}
}}
"#,
            pkg = pkg
        )
    }

    fn render_auth_controller(&self, pkg: &str) -> String {
        format!(
            r#"package {pkg}.controller;

import lombok.RequiredArgsConstructor;
import org.springframework.http.ResponseEntity;
import org.springframework.security.authentication.AuthenticationManager;
import org.springframework.security.authentication.UsernamePasswordAuthenticationToken;
import org.springframework.security.core.userdetails.UserDetailsService;
import org.springframework.web.bind.annotation.*;
import {pkg}.dto.LoginRequest;
import {pkg}.dto.TokenResponse;
import {pkg}.service.JwtService;

/**
 * Authentication REST endpoints.
 * POST /api/v1/auth/login  — exchange credentials for tokens
 * POST /api/v1/auth/refresh — exchange refresh token for new access token
 */
@RestController
@RequestMapping("/api/v1/auth")
@RequiredArgsConstructor
public class AuthController {{

    private final AuthenticationManager authenticationManager;
    private final UserDetailsService userDetailsService;
    private final JwtService jwtService;

    @PostMapping("/login")
    public ResponseEntity<TokenResponse> login(@RequestBody LoginRequest request) {{
        authenticationManager.authenticate(
            new UsernamePasswordAuthenticationToken(request.username(), request.password()));

        var userDetails = userDetailsService.loadUserByUsername(request.username());
        var accessToken = jwtService.generateAccessToken(userDetails);
        var refreshToken = jwtService.generateRefreshToken(userDetails);

        return ResponseEntity.ok(new TokenResponse(accessToken, refreshToken));
    }}
}}
"#,
            pkg = pkg
        )
    }

    fn render_token_response(&self, pkg: &str) -> String {
        format!(
            r#"package {pkg}.dto;

/**
 * JWT token pair returned after successful authentication.
 */
public record TokenResponse(
    String accessToken,
    String refreshToken
) {{}}
"#,
            pkg = pkg
        )
    }

    fn render_login_request(&self, pkg: &str) -> String {
        format!(
            r#"package {pkg}.dto;

import jakarta.validation.constraints.NotBlank;

public record LoginRequest(
    @NotBlank String username,
    @NotBlank String password
) {{}}
"#,
            pkg = pkg
        )
    }

    fn render_openapi_config(&self, pkg: &str) -> String {
        let name = &self.config.project.name;
        format!(
            r#"package {pkg}.config;

import io.swagger.v3.oas.models.OpenAPI;
import io.swagger.v3.oas.models.info.Contact;
import io.swagger.v3.oas.models.info.Info;
import io.swagger.v3.oas.models.info.License;
import io.swagger.v3.oas.models.security.SecurityRequirement;
import io.swagger.v3.oas.models.security.SecurityScheme;
import io.swagger.v3.oas.models.Components;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;

/**
 * OpenAPI 3 / Swagger UI configuration.
 * Access at: http://localhost:8080/swagger-ui.html
 */
@Configuration
public class OpenApiConfig {{

    @Bean
    public OpenAPI openApi() {{
        return new OpenAPI()
            .info(new Info()
                .title("{name} API")
                .version("v1")
                .description("API documentation")
                .contact(new Contact()
                    .name("Your Team")
                    .email("team@example.com"))
                .license(new License()
                    .name("Apache 2.0")
                    .url("http://www.apache.org/licenses/LICENSE-2.0")))
            .addSecurityItem(new SecurityRequirement().addList("Bearer Authentication"))
            .components(new Components()
                .addSecuritySchemes("Bearer Authentication", new SecurityScheme()
                    .type(SecurityScheme.Type.HTTP)
                    .scheme("bearer")
                    .bearerFormat("JWT")
                    .description("Enter JWT token")));
    }}
}}
"#,
            pkg = pkg,
            name = name
        )
    }

    fn render_s3_config(&self, pkg: &str) -> String {
        format!(
            r#"package {pkg}.config;

import org.springframework.beans.factory.annotation.Value;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;
import software.amazon.awssdk.auth.credentials.DefaultCredentialsProvider;
import software.amazon.awssdk.regions.Region;
import software.amazon.awssdk.services.s3.S3Client;
import software.amazon.awssdk.services.s3.presigner.S3Presigner;

import java.net.URI;

@Configuration
public class S3Config {{

    @Value("${{app.s3.region}}")
    private String region;

    @Value("${{app.s3.endpoint:}}")
    private String endpoint;

    @Value("${{app.s3.path-style-access:false}}")
    private boolean pathStyleAccess;

    @Bean
    public S3Client s3Client() {{
        var builder = S3Client.builder()
            .region(Region.of(region))
            .credentialsProvider(DefaultCredentialsProvider.create());

        if (endpoint != null && !endpoint.isBlank()) {{
            builder.endpointOverride(URI.create(endpoint));
        }}
        if (pathStyleAccess) {{
            builder.forcePathStyle(true);
        }}
        return builder.build();
    }}

    @Bean
    public S3Presigner s3Presigner() {{
        var builder = S3Presigner.builder()
            .region(Region.of(region))
            .credentialsProvider(DefaultCredentialsProvider.create());
        if (endpoint != null && !endpoint.isBlank()) {{
            builder.endpointOverride(URI.create(endpoint));
        }}
        return builder.build();
    }}
}}
"#,
            pkg = pkg
        )
    }

    fn render_s3_service(&self, pkg: &str) -> String {
        format!(
            r#"package {pkg}.service;

import lombok.RequiredArgsConstructor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.stereotype.Service;
import software.amazon.awssdk.core.sync.RequestBody;
import software.amazon.awssdk.services.s3.S3Client;
import software.amazon.awssdk.services.s3.model.*;
import software.amazon.awssdk.services.s3.presigner.S3Presigner;
import software.amazon.awssdk.services.s3.presigner.model.GetObjectPresignRequest;

import java.io.InputStream;
import java.time.Duration;

@Slf4j
@Service
@RequiredArgsConstructor
public class S3Service {{

    private final S3Client s3Client;
    private final S3Presigner s3Presigner;

    @Value("${{app.s3.bucket}}")
    private String bucket;

    public void upload(String key, InputStream content, long contentLength, String contentType) {{
        s3Client.putObject(
            PutObjectRequest.builder().bucket(bucket).key(key).contentType(contentType).build(),
            RequestBody.fromInputStream(content, contentLength));
        log.info("Uploaded s3://{{}}/{{}}", bucket, key);
    }}

    public InputStream download(String key) {{
        return s3Client.getObject(GetObjectRequest.builder().bucket(bucket).key(key).build());
    }}

    public void delete(String key) {{
        s3Client.deleteObject(DeleteObjectRequest.builder().bucket(bucket).key(key).build());
    }}

    public String generatePresignedUrl(String key, Duration expiry) {{
        var presignRequest = GetObjectPresignRequest.builder()
            .signatureDuration(expiry)
            .getObjectRequest(GetObjectRequest.builder().bucket(bucket).key(key).build())
            .build();
        return s3Presigner.presignGetObject(presignRequest).url().toString();
    }}
}}
"#,
            pkg = pkg
        )
    }

    fn render_email_config(&self, pkg: &str) -> String {
        format!(
            r#"package {pkg}.config;

import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;
import org.thymeleaf.spring6.SpringTemplateEngine;
import org.thymeleaf.spring6.templateresolver.SpringResourceTemplateResolver;
import org.thymeleaf.templatemode.TemplateMode;

import java.nio.charset.StandardCharsets;

@Configuration
public class EmailConfig {{

    @Bean
    public SpringResourceTemplateResolver emailTemplateResolver() {{
        var resolver = new SpringResourceTemplateResolver();
        resolver.setPrefix("classpath:/templates/email/");
        resolver.setSuffix(".html");
        resolver.setTemplateMode(TemplateMode.HTML);
        resolver.setCharacterEncoding(StandardCharsets.UTF_8.name());
        resolver.setOrder(1);
        resolver.setCheckExistence(true);
        return resolver;
    }}
}}
"#,
            pkg = pkg
        )
    }

    fn render_email_service(&self, pkg: &str) -> String {
        format!(
            r#"package {pkg}.service;

import lombok.RequiredArgsConstructor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.mail.javamail.JavaMailSender;
import org.springframework.mail.javamail.MimeMessageHelper;
import org.springframework.stereotype.Service;
import org.thymeleaf.context.Context;
import org.thymeleaf.spring6.SpringTemplateEngine;

import java.util.Map;

@Slf4j
@Service
@RequiredArgsConstructor
public class EmailService {{

    private final JavaMailSender mailSender;
    private final SpringTemplateEngine templateEngine;

    @Value("${{app.mail.from}}")
    private String from;

    public void sendTemplateEmail(String to, String subject, String template, Map<String, Object> variables) {{
        try {{
            var ctx = new Context();
            ctx.setVariables(variables);
            String html = templateEngine.process("email/" + template, ctx);

            var message = mailSender.createMimeMessage();
            var helper = new MimeMessageHelper(message, true, "UTF-8");
            helper.setFrom(from);
            helper.setTo(to);
            helper.setSubject(subject);
            helper.setText(html, true);

            mailSender.send(message);
            log.info("Sent email to={{}} subject={{}}", to, subject);
        }} catch (Exception e) {{
            log.error("Failed to send email to={{}}: {{}}", to, e.getMessage(), e);
            throw new RuntimeException("Email send failed", e);
        }}
    }}
}}
"#,
            pkg = pkg
        )
    }

    fn render_websocket_config(&self, pkg: &str) -> String {
        format!(
            r#"package {pkg}.config;

import org.springframework.beans.factory.annotation.Value;
import org.springframework.context.annotation.Configuration;
import org.springframework.messaging.simp.config.MessageBrokerRegistry;
import org.springframework.web.socket.config.annotation.*;

@Configuration
@EnableWebSocketMessageBroker
public class WebSocketConfig implements WebSocketMessageBrokerConfigurer {{

    @Value("${{app.websocket.allowed-origins:*}}")
    private String allowedOrigins;

    @Override
    public void registerStompEndpoints(StompEndpointRegistry registry) {{
        registry.addEndpoint("/ws")
            .setAllowedOriginPatterns(allowedOrigins)
            .withSockJS();
    }}

    @Override
    public void configureMessageBroker(MessageBrokerRegistry config) {{
        config.enableSimpleBroker("/topic", "/queue");
        config.setApplicationDestinationPrefixes("/app");
        config.setUserDestinationPrefix("/user");
    }}
}}
"#,
            pkg = pkg
        )
    }

    fn render_mongo_config(&self, pkg: &str) -> String {
        format!(
            r#"package {pkg}.config;

import org.springframework.context.annotation.Configuration;
import org.springframework.data.mongodb.config.EnableMongoAuditing;
import org.springframework.data.mongodb.repository.config.EnableMongoRepositories;

@Configuration
@EnableMongoAuditing
@EnableMongoRepositories(basePackages = "{pkg}.repository")
public class MongoConfig {{
    // Auditing fields (@CreatedDate, @LastModifiedDate) are auto-populated
}}
"#,
            pkg = pkg
        )
    }

    fn render_jpa_config(&self, pkg: &str) -> String {
        format!(
            r#"package {pkg}.config;

import org.springframework.context.annotation.Configuration;
import org.springframework.data.jpa.repository.config.EnableJpaAuditing;
import org.springframework.data.jpa.repository.config.EnableJpaRepositories;
import org.springframework.transaction.annotation.EnableTransactionManagement;

@Configuration
@EnableJpaAuditing
@EnableTransactionManagement
@EnableJpaRepositories(basePackages = "{pkg}.repository")
public class JpaConfig {{
    // JPA auditing: annotate entity fields with @CreatedDate, @LastModifiedDate
    // Use @EntityListeners(AuditingEntityListener.class) on entities
}}
"#,
            pkg = pkg
        )
    }
}
