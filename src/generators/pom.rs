use anyhow::Result;
use std::path::Path;

use crate::config::ProjectConfig;
use crate::features::FeatureSpec;

pub struct PomGenerator<'a> {
    config: &'a ProjectConfig,
    features: &'a [FeatureSpec],
}

impl<'a> PomGenerator<'a> {
    pub fn new(config: &'a ProjectConfig, features: &'a [FeatureSpec]) -> Self {
        Self { config, features }
    }

    pub fn generate(&self, out: &Path) -> Result<()> {
        let pom = self.render();
        std::fs::write(out.join("pom.xml"), pom)?;
        Ok(())
    }

    fn render(&self) -> String {
        let meta = &self.config.project;
        let artifact = crate::engine::to_artifact_id(&meta.name);

        let mut deps = String::new();
        deps.push_str(&self.dep(
            "org.springframework.boot",
            "spring-boot-starter-web",
            None,
            None,
        ));
        deps.push_str(&self.dep(
            "org.springframework.boot",
            "spring-boot-starter-validation",
            None,
            None,
        ));
        deps.push_str(&self.dep("org.projectlombok", "lombok", None, Some("provided")));
        deps.push_str(&self.dep(
            "org.springframework.boot",
            "spring-boot-starter-test",
            None,
            Some("test"),
        ));

        // Collect all feature deps (deduplicated)
        let mut seen_deps: Vec<(&str, &str)> = Vec::new();
        for feature in self.features {
            for dep in feature.maven_deps {
                let key = (dep.group_id, dep.artifact_id);
                if !seen_deps.contains(&key) {
                    seen_deps.push(key);
                    deps.push_str(&self.dep(dep.group_id, dep.artifact_id, dep.version, dep.scope));
                }
            }
        }

        // Test deps always included
        deps.push_str(&self.dep("org.testcontainers", "junit-jupiter", None, Some("test")));

        let has_kafka = self.features.iter().any(|f| f.key == "kafka");
        if has_kafka {
            deps.push_str(&self.dep(
                "org.springframework.kafka",
                "spring-kafka-test",
                None,
                Some("test"),
            ));
        }

        let has_postgres = self.features.iter().any(|f| f.key == "postgres");
        if has_postgres {
            deps.push_str(&self.dep("org.testcontainers", "postgresql", None, Some("test")));
        }

        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 https://maven.apache.org/xsd/maven-4.0.0.xsd">
    <modelVersion>4.0.0</modelVersion>

    <parent>
        <groupId>org.springframework.boot</groupId>
        <artifactId>spring-boot-starter-parent</artifactId>
        <version>{boot_version}</version>
        <relativePath/>
    </parent>

    <groupId>{group}</groupId>
    <artifactId>{artifact}</artifactId>
    <version>{version}</version>
    <packaging>jar</packaging>

    <name>{name}</name>
    <description>{description}</description>

    <properties>
        <java.version>{java_version}</java.version>
        <project.build.sourceEncoding>UTF-8</project.build.sourceEncoding>
        <project.reporting.outputEncoding>UTF-8</project.reporting.outputEncoding>
        <!-- Dependency versions managed here for visibility -->
        <testcontainers.version>1.19.7</testcontainers.version>
    </properties>

    <dependencyManagement>
        <dependencies>
            <dependency>
                <groupId>org.testcontainers</groupId>
                <artifactId>testcontainers-bom</artifactId>
                <version>${{testcontainers.version}}</version>
                <type>pom</type>
                <scope>import</scope>
            </dependency>
        </dependencies>
    </dependencyManagement>

    <dependencies>
{deps}
    </dependencies>

    <build>
        <plugins>
            <plugin>
                <groupId>org.springframework.boot</groupId>
                <artifactId>spring-boot-maven-plugin</artifactId>
                <configuration>
                    <excludes>
                        <exclude>
                            <groupId>org.projectlombok</groupId>
                            <artifactId>lombok</artifactId>
                        </exclude>
                    </excludes>
                    <image>
                        <name>${{project.artifactId}}:${{project.version}}</name>
                    </image>
                </configuration>
            </plugin>
            <plugin>
                <groupId>org.apache.maven.plugins</groupId>
                <artifactId>maven-compiler-plugin</artifactId>
                <configuration>
                    <source>${{java.version}}</source>
                    <target>${{java.version}}</target>
                    <compilerArgs>
                        <arg>-parameters</arg>
                    </compilerArgs>
                    <annotationProcessorPaths>
                        <path>
                            <groupId>org.projectlombok</groupId>
                            <artifactId>lombok</artifactId>
                        </path>
                    </annotationProcessorPaths>
                </configuration>
            </plugin>
            <plugin>
                <groupId>org.apache.maven.plugins</groupId>
                <artifactId>maven-surefire-plugin</artifactId>
                <configuration>
                    <excludes>
                        <exclude>**/*IntegrationTest.java</exclude>
                    </excludes>
                </configuration>
            </plugin>
            <plugin>
                <groupId>org.apache.maven.plugins</groupId>
                <artifactId>maven-failsafe-plugin</artifactId>
                <executions>
                    <execution>
                        <goals>
                            <goal>integration-test</goal>
                            <goal>verify</goal>
                        </goals>
                    </execution>
                </executions>
            </plugin>
        </plugins>
    </build>

    <profiles>
        <profile>
            <id>dev</id>
            <activation><activeByDefault>true</activeByDefault></activation>
            <properties>
                <spring.profiles.active>dev</spring.profiles.active>
            </properties>
            <dependencies>
                <dependency>
                    <groupId>org.springframework.boot</groupId>
                    <artifactId>spring-boot-devtools</artifactId>
                    <optional>true</optional>
                </dependency>
            </dependencies>
        </profile>
        <profile>
            <id>prod</id>
            <properties>
                <spring.profiles.active>prod</spring.profiles.active>
            </properties>
        </profile>
    </profiles>

</project>
"#,
            boot_version = meta.boot_version,
            group = meta.group,
            artifact = artifact,
            version = meta.version,
            name = meta.name,
            description = meta.description,
            java_version = meta.java_version,
            deps = deps,
        )
    }

    fn dep(
        &self,
        group_id: &str,
        artifact_id: &str,
        version: Option<&str>,
        scope: Option<&str>,
    ) -> String {
        let mut s = format!(
            "        <dependency>\n            <groupId>{}</groupId>\n            <artifactId>{}</artifactId>\n",
            group_id, artifact_id
        );
        if let Some(v) = version {
            s.push_str(&format!("            <version>{}</version>\n", v));
        }
        if let Some(sc) = scope {
            s.push_str(&format!("            <scope>{}</scope>\n", sc));
        }
        s.push_str("        </dependency>\n");
        s
    }
}
