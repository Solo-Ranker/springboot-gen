use anyhow::Result;
use std::path::Path;

use crate::config::ProjectConfig;
use crate::features::FeatureSpec;

pub struct KubernetesGenerator<'a> {
    config: &'a ProjectConfig,
    features: &'a [FeatureSpec],
}

impl<'a> KubernetesGenerator<'a> {
    pub fn new(config: &'a ProjectConfig, features: &'a [FeatureSpec]) -> Self {
        Self { config, features }
    }

    pub fn generate(&self, out: &Path) -> Result<()> {
        let k8s_dir = out.join("k8s");
        std::fs::create_dir_all(&k8s_dir)?;

        let artifact = crate::engine::to_artifact_id(&self.config.project.name);
        let port = self.config.docker.app_port;

        std::fs::write(
            k8s_dir.join("namespace.yaml"),
            self.render_namespace(&artifact),
        )?;
        std::fs::write(
            k8s_dir.join("configmap.yaml"),
            self.render_configmap(&artifact),
        )?;
        std::fs::write(k8s_dir.join("secret.yaml"), self.render_secret(&artifact))?;
        std::fs::write(
            k8s_dir.join("deployment.yaml"),
            self.render_deployment(&artifact, port),
        )?;
        std::fs::write(
            k8s_dir.join("service.yaml"),
            self.render_service(&artifact, port),
        )?;
        std::fs::write(k8s_dir.join("hpa.yaml"), self.render_hpa(&artifact))?;
        std::fs::write(
            k8s_dir.join("ingress.yaml"),
            self.render_ingress(&artifact, port),
        )?;
        std::fs::write(
            k8s_dir.join("kustomization.yaml"),
            self.render_kustomization(),
        )?;

        Ok(())
    }

    fn render_namespace(&self, artifact: &str) -> String {
        format!(
            r#"apiVersion: v1
kind: Namespace
metadata:
  name: {artifact}
  labels:
    app.kubernetes.io/name: {artifact}
    managed-by: springboot-gen
"#,
            artifact = artifact
        )
    }

    fn render_configmap(&self, artifact: &str) -> String {
        let mut data = format!(
            r#"apiVersion: v1
kind: ConfigMap
metadata:
  name: {artifact}-config
  namespace: {artifact}
data:
  SPRING_PROFILES_ACTIVE: "prod"
  SERVER_PORT: "{port}"
"#,
            artifact = artifact,
            port = self.config.docker.app_port
        );

        if self.has("redis") {
            data.push_str("  REDIS_HOST: redis-service\n");
            data.push_str("  REDIS_PORT: \"6379\"\n");
        }
        if self.has("redis-sentinel") {
            data.push_str("  REDIS_SENTINEL_MASTER: \"mymaster\"\n");
            data.push_str(
                "  REDIS_SENTINEL_NODES: \"redis-sentinel-0:26379,redis-sentinel-1:26379\"\n",
            );
        }
        if self.has("kafka") {
            data.push_str("  KAFKA_BOOTSTRAP_SERVERS: \"kafka-service:9092\"\n");
        }
        if self.has("postgres") {
            data.push_str("  DB_HOST: \"postgres-service\"\n");
            data.push_str("  DB_PORT: \"5432\"\n");
        }
        if self.has("tracing") {
            data.push_str("  ZIPKIN_ENDPOINT: \"http://zipkin-service:9411/api/v2/spans\"\n");
        }

        data
    }

    fn render_secret(&self, artifact: &str) -> String {
        format!(
            r#"# ─────────────────────────────────────────────────────────
# WARNING: Base64-encode real secrets before committing.
# Better: use Sealed Secrets, Vault, or External Secrets Operator.
# ─────────────────────────────────────────────────────────
apiVersion: v1
kind: Secret
metadata:
  name: {artifact}-secret
  namespace: {artifact}
type: Opaque
stringData:
  DB_PASSWORD: "changeit"
  REDIS_PASSWORD: ""
  JWT_SECRET: "change-me-in-production-with-a-256bit-key"
  AWS_ACCESS_KEY_ID: ""
  AWS_SECRET_ACCESS_KEY: ""
"#,
            artifact = artifact
        )
    }

    fn render_deployment(&self, artifact: &str, port: u16) -> String {
        format!(
            r#"apiVersion: apps/v1
kind: Deployment
metadata:
  name: {artifact}
  namespace: {artifact}
  labels:
    app: {artifact}
    app.kubernetes.io/name: {artifact}
    app.kubernetes.io/version: "latest"
spec:
  replicas: 2
  selector:
    matchLabels:
      app: {artifact}
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxSurge: 1
      maxUnavailable: 0
  template:
    metadata:
      labels:
        app: {artifact}
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "{port}"
        prometheus.io/path: "/actuator/prometheus"
    spec:
      serviceAccountName: {artifact}
      securityContext:
        runAsNonRoot: true
        runAsUser: 1000
        fsGroup: 2000
      containers:
        - name: {artifact}
          image: {artifact}:latest
          imagePullPolicy: Always
          ports:
            - containerPort: {port}
              protocol: TCP
          envFrom:
            - configMapRef:
                name: {artifact}-config
            - secretRef:
                name: {artifact}-secret
          resources:
            requests:
              memory: "256Mi"
              cpu: "100m"
            limits:
              memory: "512Mi"
              cpu: "500m"
          livenessProbe:
            httpGet:
              path: /actuator/health/liveness
              port: {port}
            initialDelaySeconds: 60
            periodSeconds: 10
            failureThreshold: 3
          readinessProbe:
            httpGet:
              path: /actuator/health/readiness
              port: {port}
            initialDelaySeconds: 30
            periodSeconds: 5
            failureThreshold: 3
          startupProbe:
            httpGet:
              path: /actuator/health
              port: {port}
            initialDelaySeconds: 10
            periodSeconds: 5
            failureThreshold: 30
          lifecycle:
            preStop:
              exec:
                command: ["/bin/sh", "-c", "sleep 10"]
      terminationGracePeriodSeconds: 60
---
apiVersion: v1
kind: ServiceAccount
metadata:
  name: {artifact}
  namespace: {artifact}
"#,
            artifact = artifact,
            port = port
        )
    }

    fn render_service(&self, artifact: &str, port: u16) -> String {
        format!(
            r#"apiVersion: v1
kind: Service
metadata:
  name: {artifact}-service
  namespace: {artifact}
  labels:
    app: {artifact}
spec:
  type: ClusterIP
  selector:
    app: {artifact}
  ports:
    - protocol: TCP
      port: 80
      targetPort: {port}
      name: http
"#,
            artifact = artifact,
            port = port
        )
    }

    fn render_hpa(&self, artifact: &str) -> String {
        format!(
            r#"apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: {artifact}-hpa
  namespace: {artifact}
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: {artifact}
  minReplicas: 2
  maxReplicas: 10
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
    - type: Resource
      resource:
        name: memory
        target:
          type: Utilization
          averageUtilization: 80
"#,
            artifact = artifact
        )
    }

    fn render_ingress(&self, artifact: &str, _port: u16) -> String {
        format!(
            r#"apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: {artifact}-ingress
  namespace: {artifact}
  annotations:
    nginx.ingress.kubernetes.io/rewrite-target: /
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
    cert-manager.io/cluster-issuer: "letsencrypt-prod"
spec:
  ingressClassName: nginx
  tls:
    - hosts:
        - {artifact}.example.com
      secretName: {artifact}-tls
  rules:
    - host: {artifact}.example.com
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: {artifact}-service
                port:
                  number: 80
"#,
            artifact = artifact
        )
    }

    fn render_kustomization(&self) -> String {
        r#"apiVersion: kustomize.config.k8s.io/v1beta1
kind: Kustomization

resources:
  - namespace.yaml
  - configmap.yaml
  - secret.yaml
  - deployment.yaml
  - service.yaml
  - hpa.yaml
  - ingress.yaml
"#
        .to_string()
    }

    fn has(&self, key: &str) -> bool {
        self.features.iter().any(|f| f.key == key)
    }
}
