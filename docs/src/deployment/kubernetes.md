# Kubernetes Deployment Guide

Complete guide for deploying Paladin on Kubernetes with high availability, scalability, and production best practices.

## Table of Contents

- [Overview](#overview)
- [Prerequisites](#prerequisites)
- [Quick Start](#quick-start)
- [Architecture](#architecture)
- [Kubernetes Manifests](#kubernetes-manifests)
- [ConfigMaps and Secrets](#configmaps-and-secrets)
- [Helm Chart](#helm-chart)
- [Resource Management](#resource-management)
- [High Availability](#high-availability)
- [Graceful Shutdown](#graceful-shutdown)
- [Horizontal Scaling](#horizontal-scaling)
- [Storage](#storage)
- [Networking](#networking)
- [Monitoring](#monitoring)
- [Security](#security)
- [Troubleshooting](#troubleshooting)

## Overview

Paladin on Kubernetes provides:
- **High Availability**: Multi-replica deployments with health checks
- **Auto-scaling**: HPA based on CPU/memory/custom metrics
- **Rolling Updates**: Zero-downtime deployments
- **Resource Management**: CPU/memory limits and requests
- **Service Discovery**: Internal DNS for service communication

> **Scope note (read before following this guide):** the `k8s/` manifests actually shipped in
> this repository (`k8s/namespace.yaml`, `k8s/deployment.yaml`, `k8s/service.yaml`,
> `k8s/configmap.yaml`, `k8s/secret.yaml.example`, `k8s/redis.yaml`, `k8s/minio.yaml`, plus a
> `k8s/server/` variant for `paladin-server`) are a **local/CI testing fixture**, not a
> production deployment kit — `k8s/deployment.yaml` runs the image `paladin:test` with
> `imagePullPolicy: Never`, a placeholder `sleep 3600` command instead of the real binary, and
> its liveness/readiness/startup probes commented out ("Disabled for testing — needs HTTP
> server endpoint"). No Helm chart is shipped anywhere in this repository, and
> `https://charts.paladin.dev` is not a real Helm repository. Everything below this note —
> the numbered `k8s/NN-*.yaml` filenames, the Ingress/HPA/PDB/ResourceQuota/NetworkPolicy/
> ServiceMonitor/RBAC manifests, and the entire Helm Chart section — is illustrative production
> guidance the reader must author themselves; it does not describe files that exist in this
> repository today. Where a manifest below *does* have a real, shipped 1:1 counterpart, its
> filename comment has been corrected to the real path.

## Prerequisites

```bash
# Kubernetes 1.25+
kubectl version

# Helm 3.0+ (optional but recommended)
helm version

# kubectl-ctx and kubectl-ns (optional, for context switching)
kubectl ctx
kubectl ns
```

## Quick Start

### Using Kubectl

```bash
# Create namespace
kubectl create namespace paladin

# Apply manifests
kubectl apply -f k8s/ -n paladin

# Check status
kubectl get pods -n paladin
kubectl get svc -n paladin

# View logs
kubectl logs -f deployment/paladin -n paladin
```

### Using Helm

```bash
# Add Paladin Helm repository
helm repo add paladin https://charts.paladin.dev
helm repo update

# Install with default values
helm install paladin paladin/paladin -n paladin --create-namespace

# Install with custom values
helm install paladin paladin/paladin \
  -n paladin \
  --create-namespace \
  --values values.yaml

# Upgrade
helm upgrade paladin paladin/paladin -n paladin

# Uninstall
helm uninstall paladin -n paladin
```

## Architecture

```
┌──────────────────────────────────────────────────────┐
│              Kubernetes Cluster                       │
│                                                       │
│  ┌────────────────────────────────────────────────┐ │
│  │           Namespace: paladin                    │ │
│  │                                                  │ │
│  │  ┌──────────────┐      ┌──────────────┐       │ │
│  │  │   Ingress    │      │   Service    │       │ │
│  │  │  (External)  │─────▶│ (ClusterIP)  │       │ │
│  │  └──────────────┘      └───────┬──────┘       │ │
│  │                                 │               │ │
│  │                        ┌────────▼────────┐     │ │
│  │                        │   Deployment    │     │ │
│  │                        │  (Paladin x3)   │     │ │
│  │                        └────┬───┬───┬────┘     │ │
│  │                             │   │   │          │ │
│  │                 ┌───────────┼───┼───┼───────┐ │ │
│  │                 │           │   │   │       │ │ │
│  │            ┌────▼───┐  ┌───▼───▼───▼────┐  │ │ │
│  │            │ Redis  │  │ MinIO/S3        │  │ │ │
│  │            │StatefulSet│ │ StatefulSet    │  │ │ │
│  │            └────────┘  └────────────────┘  │ │ │
│  │                                              │ │ │
│  │  ┌──────────────┐      ┌──────────────┐   │ │ │
│  │  │  ConfigMap   │      │   Secret     │   │ │ │
│  │  │  (config.yml)│      │  (API keys)  │   │ │ │
│  │  └──────────────┘      └──────────────┘   │ │ │
│  └─────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────┘
```

## Kubernetes Manifests

### Namespace

```yaml
# k8s/namespace.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: paladin
  labels:
    app: paladin
    environment: production
```

### Deployment

```yaml
# k8s/deployment.yaml (illustrative production shape — the shipped file at this path is a
# local/CI test fixture; see the scope note above)
apiVersion: apps/v1
kind: Deployment
metadata:
  name: paladin
  namespace: paladin
  labels:
    app: paladin
    component: server
spec:
  replicas: 3
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxSurge: 1
      maxUnavailable: 0
  selector:
    matchLabels:
      app: paladin
      component: server
  template:
    metadata:
      labels:
        app: paladin
        component: server
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "9090"
        prometheus.io/path: "/metrics"
        # Reserved for future Prometheus metrics — no /metrics HTTP handler is wired up yet;
        # the shipped routes are /health and /ready (crates/paladin-web/src/health.rs).
    spec:
      serviceAccountName: paladin
      # 2x the configured APP_ENGINE_SHUTDOWN_GRACE_SECS (default 30s) so the
      # kubelet's SIGKILL deadline never lands mid-drain — see the Graceful
      # Shutdown section below (HITL-04, D-23).
      terminationGracePeriodSeconds: 60
      securityContext:
        runAsNonRoot: true
        runAsUser: 1000
        fsGroup: 1000

      initContainers:
      - name: wait-for-redis
        image: busybox:1.35
        command: ['sh', '-c', 'until nc -zv redis 6379; do echo waiting for redis; sleep 2; done;']

      containers:
      - name: paladin
        image: ghcr.io/your-org/paladin:v0.8.0
        imagePullPolicy: IfNotPresent

        ports:
        - name: http
          containerPort: 8080
          protocol: TCP
        - name: metrics
          containerPort: 9090
          protocol: TCP

        env:
        # NOTE: there is no SERVER_HOST/SERVER_PORT/LOG_LEVEL environment override — Paladin
        # loads config via `Environment::with_prefix("APP")` (src/config/settings.rs:66);
        # server.host/server.port are config-file-only, and logging uses RUST_LOG (the
        # standard Rust convention), not a custom LOG_LEVEL variable.
        - name: RUST_LOG
          value: "info,paladin=debug"

        # Secrets from Secret resource
        - name: OPENAI_API_KEY
          valueFrom:
            secretKeyRef:
              name: paladin-secrets
              key: openai-api-key
        - name: DEEPSEEK_API_KEY
          valueFrom:
            secretKeyRef:
              name: paladin-secrets
              key: deepseek-api-key
              optional: true
        - name: ANTHROPIC_API_KEY
          valueFrom:
            secretKeyRef:
              name: paladin-secrets
              key: anthropic-api-key
              optional: true

        # Mount configuration
        volumeMounts:
        - name: config
          mountPath: /app/config.yml
          subPath: config.yml
          readOnly: true
        - name: data
          mountPath: /app/data
        - name: tmp
          mountPath: /tmp

        # Resource limits
        resources:
          requests:
            cpu: 500m
            memory: 1Gi
          limits:
            cpu: 2000m
            memory: 4Gi

        # Health checks
        livenessProbe:
          httpGet:
            path: /health
            port: http
          initialDelaySeconds: 30
          periodSeconds: 10
          timeoutSeconds: 5
          failureThreshold: 3

        readinessProbe:
          httpGet:
            path: /ready
            port: http
          initialDelaySeconds: 10
          periodSeconds: 5
          timeoutSeconds: 3
          failureThreshold: 3

        # Graceful shutdown
        lifecycle:
          preStop:
            exec:
              command: ["/bin/sh", "-c", "sleep 10"]

      volumes:
      - name: config
        configMap:
          name: paladin-config
      - name: data
        persistentVolumeClaim:
          claimName: paladin-data
      - name: tmp
        emptyDir: {}

      # Affinity for spreading pods across nodes
      affinity:
        podAntiAffinity:
          preferredDuringSchedulingIgnoredDuringExecution:
          - weight: 100
            podAffinityTerm:
              labelSelector:
                matchExpressions:
                - key: app
                  operator: In
                  values:
                  - paladin
              topologyKey: kubernetes.io/hostname
```

### Service

```yaml
# k8s/service.yaml (illustrative production shape; the shipped file at this path defines three
# services — a ClusterIP `paladin`, a headless `paladin-headless`, and a `paladin-metrics`
# service, all on port 9090 for metrics, not 8081 — see the scope note above)
apiVersion: v1
kind: Service
metadata:
  name: paladin
  namespace: paladin
  labels:
    app: paladin
spec:
  type: ClusterIP
  selector:
    app: paladin
    component: server
  ports:
  - name: http
    port: 80
    targetPort: http
    protocol: TCP
  - name: metrics
    port: 9090
    targetPort: metrics
    protocol: TCP
  sessionAffinity: ClientIP
  sessionAffinityConfig:
    clientIP:
      timeoutSeconds: 10800
```

### Ingress

Not shipped in this repository — no `k8s/*ingress*.yaml` file exists (per the scope note above).
The following is illustrative:

```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: paladin
  namespace: paladin
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-prod
    nginx.ingress.kubernetes.io/proxy-body-size: "50m"
    nginx.ingress.kubernetes.io/proxy-read-timeout: "600"
    nginx.ingress.kubernetes.io/rate-limit: "100"
spec:
  ingressClassName: nginx
  tls:
  - hosts:
    - paladin.example.com
    secretName: paladin-tls
  rules:
  - host: paladin.example.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: paladin
            port:
              number: 80
```

## ConfigMaps and Secrets

### ConfigMap

```yaml
# k8s/configmap.yaml (illustrative production shape — corrected to the real Settings struct
# field names; the shipped file at this path is a CI test fixture and carries the same
# `type:`/`paladin:` field-name drift this correction fixes here, per the scope note above)
apiVersion: v1
kind: ConfigMap
metadata:
  name: paladin-config
  namespace: paladin
data:
  config.yml: |
    server:
      host: "0.0.0.0"
      port: 8080

    # No top-level `paladin:` defaults section exists — a single Paladin's model/temperature/
    # max_loops are set via the Rust PaladinBuilder API; the HTTP service host loads a list of
    # agent definitions under `agents:` instead (see docs/src/user-guides/paladin-configuration.md).

    garrison:
      garrison_type: "sqlite"
      path: "/app/data/garrison.db"
      max_entries: 1000
      max_tokens: 8000

    arsenal:
      mcp_servers:
        - name: "web_search"
          server_type: "stdio"
          command: "uvx"
          args: ["mcp-web-search"]

    llm:
      openai:
        api_key: "${OPENAI_API_KEY}"
        base_url: "https://api.openai.com/v1"
      deepseek:
        api_key: "${DEEPSEEK_API_KEY}"
        base_url: "https://api.deepseek.com/v1"
      anthropic:
        api_key: "${ANTHROPIC_API_KEY}"
        base_url: "https://api.anthropic.com/v1"

    file_storage:
      minio_endpoint: "minio.paladin.svc.cluster.local:9000"
      minio_access_key: "minioadmin"
      minio_secret_key: "minioadmin"
      minio_bucket: "paladin"
      minio_secure: false

    queue:
      redis_host: "redis.paladin.svc.cluster.local"
      redis_port: 6379
```

### Secret

```bash
# Create secret from literals
kubectl create secret generic paladin-secrets \
  --from-literal=openai-api-key="sk-..." \
  --from-literal=deepseek-api-key="..." \
  --from-literal=anthropic-api-key="..." \
  -n paladin

# Or from env file
kubectl create secret generic paladin-secrets \
  --from-env-file=secrets.env \
  -n paladin

# Or from YAML (base64 encoded) — illustrative shape; the repo ships a template at
# k8s/secret.yaml.example (copy to k8s/secret.yaml, which is gitignored, and fill in real values)
apiVersion: v1
kind: Secret
metadata:
  name: paladin-secrets
  namespace: paladin
type: Opaque
data:
  openai-api-key: <base64-encoded-key>
  deepseek-api-key: <base64-encoded-key>
  anthropic-api-key: <base64-encoded-key>
```

## Helm Chart

### Chart Structure

```
paladin-chart/
├── Chart.yaml
├── values.yaml
├── templates/
│   ├── _helpers.tpl
│   ├── deployment.yaml
│   ├── service.yaml
│   ├── ingress.yaml
│   ├── configmap.yaml
│   ├── secret.yaml
│   ├── serviceaccount.yaml
│   ├── hpa.yaml
│   ├── pdb.yaml
│   └── NOTES.txt
└── crds/
```

### values.yaml

```yaml
# Default values for paladin
replicaCount: 3

image:
  repository: ghcr.io/your-org/paladin
  tag: "v0.8.0"
  pullPolicy: IfNotPresent

serviceAccount:
  create: true
  name: paladin

service:
  type: ClusterIP
  port: 80
  targetPort: 8080

ingress:
  enabled: true
  className: nginx
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-prod
  hosts:
    - host: paladin.example.com
      paths:
        - path: /
          pathType: Prefix
  tls:
    - secretName: paladin-tls
      hosts:
        - paladin.example.com

resources:
  requests:
    cpu: 500m
    memory: 1Gi
  limits:
    cpu: 2000m
    memory: 4Gi

autoscaling:
  enabled: true
  minReplicas: 3
  maxReplicas: 10
  targetCPUUtilizationPercentage: 70
  targetMemoryUtilizationPercentage: 80

persistence:
  enabled: true
  storageClass: "fast-ssd"
  accessMode: ReadWriteOnce
  size: 10Gi

# Paladin configuration
config:
  paladin:
    defaultModel: "gpt-4"
    defaultTemperature: 0.7
    defaultMaxLoops: 3

  garrison:
    type: "sqlite"
    maxEntries: 1000
    maxTokens: 8000

  redis:
    url: "redis://redis:6379"

  minio:
    endpoint: "minio:9000"
    bucket: "paladin"

# Secrets (should be overridden)
secrets:
  openaiApiKey: ""
  deepseekApiKey: ""
  anthropicApiKey: ""
```

### Install with Helm

```bash
# Create values-prod.yaml
cat > values-prod.yaml <<EOF
replicaCount: 5

ingress:
  hosts:
    - host: paladin.prod.example.com
      paths:
        - path: /
          pathType: Prefix

resources:
  requests:
    cpu: 1000m
    memory: 2Gi
  limits:
    cpu: 4000m
    memory: 8Gi

autoscaling:
  enabled: true
  minReplicas: 5
  maxReplicas: 20

secrets:
  openaiApiKey: ${OPENAI_API_KEY}
EOF

# Install
helm install paladin ./paladin-chart \
  -n paladin \
  --create-namespace \
  -f values-prod.yaml
```

## Resource Management

### Resource Requests and Limits

```yaml
resources:
  requests:
    cpu: 500m       # Guaranteed CPU
    memory: 1Gi     # Guaranteed memory
  limits:
    cpu: 2000m      # Max CPU (burst)
    memory: 4Gi     # Max memory (OOM if exceeded)
```

### QoS Classes

| Class | Configuration | Behavior |
|-------|---------------|----------|
| **Guaranteed** | requests = limits | Highest priority, last to evict |
| **Burstable** | requests < limits | Medium priority |
| **BestEffort** | No requests/limits | Lowest priority, first to evict |

**Recommendation**: Use **Burstable** for production (requests < limits).

### Resource Quotas

```yaml
# illustrative — not shipped in this repository (see the scope note above)
apiVersion: v1
kind: ResourceQuota
metadata:
  name: paladin-quota
  namespace: paladin
spec:
  hard:
    requests.cpu: "10"
    requests.memory: "20Gi"
    limits.cpu: "20"
    limits.memory: "40Gi"
    pods: "50"
    services: "10"
    persistentvolumeclaims: "10"
```

## High Availability

### Pod Disruption Budget

```yaml
# illustrative — not shipped in this repository (see the scope note above)
apiVersion: policy/v1
kind: PodDisruptionBudget
metadata:
  name: paladin
  namespace: paladin
spec:
  minAvailable: 2
  selector:
    matchLabels:
      app: paladin
```

### Multi-Zone Deployment

```yaml
affinity:
  podAntiAffinity:
    preferredDuringSchedulingIgnoredDuringExecution:
    - weight: 100
      podAffinityTerm:
        labelSelector:
          matchExpressions:
          - key: app
            operator: In
            values:
            - paladin
        topologyKey: topology.kubernetes.io/zone
```

## Graceful Shutdown

On SIGTERM/SIGINT, `paladin-server` and the `ServiceRunner`-based binaries cancel a
`ShutdownCoordinator` shared with every in-flight superstep run and wait up to a configured
grace window for those runs to finish before the process exits (HITL-04, D-21/D-22).

**The rule: `terminationGracePeriodSeconds` must be at least twice the configured
`APP_ENGINE_SHUTDOWN_GRACE_SECS`.** Both `k8s/server/deployment.yaml` and `k8s/deployment.yaml`
set `terminationGracePeriodSeconds: 60` — 2x the 30-second default grace — so the kubelet's
SIGKILL deadline never lands while the process is still mid-drain. If you raise
`APP_ENGINE_SHUTDOWN_GRACE_SECS`, raise `terminationGracePeriodSeconds` to at least twice that
new value too.

Two env vars, both read by `EngineConfig` (`src/config/engine.rs`):

| Env var | Default | Meaning |
|---|---|---|
| `APP_ENGINE_SHUTDOWN_GRACE_SECS` | `30` | Seconds the process waits, after SIGTERM/SIGINT, for in-flight superstep runs to finish before giving up on the stragglers. `0` aborts immediately; values above 3600 are rejected as a misconfiguration. |
| `APP_ENGINE_GRACEFUL_SHUTDOWN` | `true` | Set to `false` to restore the legacy no-wait behavior — the process exits immediately on SIGTERM/SIGINT without waiting for any in-flight run (the `MIGRATION.md` M-B-02 disable switch for legacy-only deployments). |

What an operator observes on SIGTERM: a run still executing when the signal arrives either
finishes inside the grace window and its Waypoint records completion normally, or it is still
running at the deadline — in which case it is aborted, its node's execution record reads
`Skipped { reason: "shutdown" }`, and the node's id is re-listed in the Halted Waypoint's
vanguard so the next `resume` re-runs it exactly once. No in-flight work silently vanishes
either way.

## Horizontal Scaling

### Horizontal Pod Autoscaler

```yaml
# illustrative — not shipped in this repository (see the scope note above)
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: paladin
  namespace: paladin
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: paladin
  minReplicas: 3
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
  behavior:
    scaleDown:
      stabilizationWindowSeconds: 300
      policies:
      - type: Percent
        value: 50
        periodSeconds: 60
    scaleUp:
      stabilizationWindowSeconds: 0
      policies:
      - type: Percent
        value: 100
        periodSeconds: 30
      - type: Pods
        value: 2
        periodSeconds: 30
      selectPolicy: Max
```

## Storage

### PersistentVolumeClaim

```yaml
# illustrative — the shipped k8s/deployment.yaml uses an emptyDir for /data instead of a PVC
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: paladin-data
  namespace: paladin
spec:
  accessModes:
    - ReadWriteOnce
  storageClassName: fast-ssd
  resources:
    requests:
      storage: 10Gi
```

### StatefulSet for Redis

```yaml
# illustrative — the shipped k8s/redis.yaml is a plain Deployment with an emptyDir volume
# (ephemeral), not a StatefulSet with a PVC; use a StatefulSet like this one for real persistence
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: redis
  namespace: paladin
spec:
  serviceName: redis
  replicas: 1
  selector:
    matchLabels:
      app: redis
  template:
    metadata:
      labels:
        app: redis
    spec:
      containers:
      - name: redis
        image: redis:7-alpine
        ports:
        - containerPort: 6379
          name: redis
        volumeMounts:
        - name: data
          mountPath: /data
  volumeClaimTemplates:
  - metadata:
      name: data
    spec:
      accessModes: [ "ReadWriteOnce" ]
      storageClassName: fast-ssd
      resources:
        requests:
          storage: 5Gi
```

## Networking

### Network Policies

```yaml
# illustrative — not shipped in this repository (see the scope note above)
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: paladin
  namespace: paladin
spec:
  podSelector:
    matchLabels:
      app: paladin
  policyTypes:
  - Ingress
  - Egress
  ingress:
  - from:
    - namespaceSelector:
        matchLabels:
          name: ingress-nginx
    ports:
    - protocol: TCP
      port: 8080
  egress:
  - to:
    - podSelector:
        matchLabels:
          app: redis
    ports:
    - protocol: TCP
      port: 6379
  - to:
    - podSelector:
        matchLabels:
          app: minio
    ports:
    - protocol: TCP
      port: 9000
  - to: []  # Allow all external (LLM APIs)
```

## Monitoring

### ServiceMonitor (Prometheus Operator)

```yaml
# illustrative — not shipped in this repository; also depends on the Prometheus Operator CRDs
# and a real /metrics handler, neither of which exists yet (see the scope note above)
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: paladin
  namespace: paladin
  labels:
    app: paladin
spec:
  selector:
    matchLabels:
      app: paladin
  endpoints:
  - port: metrics
    interval: 30s
    path: /metrics
```

## Security

### ServiceAccount and RBAC

```yaml
# illustrative — not shipped in this repository (see the scope note above)
apiVersion: v1
kind: ServiceAccount
metadata:
  name: paladin
  namespace: paladin

---
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: paladin
  namespace: paladin
rules:
- apiGroups: [""]
  resources: ["configmaps", "secrets"]
  verbs: ["get", "list"]

---
apiVersion: rbac.authorization.k8s.io/v1
kind: RoleBinding
metadata:
  name: paladin
  namespace: paladin
subjects:
- kind: ServiceAccount
  name: paladin
  namespace: paladin
roleRef:
  kind: Role
  name: paladin
  apiGroup: rbac.authorization.k8s.io
```

## Troubleshooting

### Common Issues

```bash
# Pods not starting
kubectl describe pod <pod-name> -n paladin
kubectl logs <pod-name> -n paladin

# Service not accessible
kubectl get svc -n paladin
kubectl get endpoints -n paladin

# Config issues
kubectl get configmap paladin-config -o yaml -n paladin
kubectl get secret paladin-secrets -o yaml -n paladin

# Resource constraints
kubectl top pods -n paladin
kubectl describe node <node-name>

# Network issues
kubectl exec -it <pod-name> -n paladin -- curl http://redis:6379
kubectl get networkpolicy -n paladin
```

## Next Steps

- **[CI/CD](cicd.md)** - Automated deployments
- **[Monitoring](../operations/monitoring.md)** - Observability
- **[Production Best Practices](production.md)** - Production checklist
