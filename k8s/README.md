# Kubernetes Deployment Guide

This directory contains Kubernetes manifests for deploying Paladin in a production environment.

## `paladin-server` (HTTP API)

The [`server/`](server/) subdirectory holds a standalone deployment of the **`paladin-server`**
HTTP API (Milestone 12) — no Redis/MinIO required (agents are LLM + prompt only):

- [`server/configmap.yaml`](server/configmap.yaml) — the `config.yml` (`/v1` agent API, auth, docs).
- [`server/deployment.yaml`](server/deployment.yaml) — Deployment with **liveness `/health`** +
  **readiness `/ready`** probes, read-only root FS, non-root, config mounted from the ConfigMap.
- [`server/service.yaml`](server/service.yaml) — ClusterIP Service on port 80 → container `8080`.
- [`server/secret.yaml.example`](server/secret.yaml.example) — provider + API-key secrets template.

```bash
# Build & load the image (or push to your registry and update the Deployment image:)
make docker-build-server
kubectl apply -f k8s/namespace.yaml
cp k8s/server/secret.yaml.example k8s/server/secret.yaml   # fill in real values (gitignored)
kubectl apply -f k8s/server/secret.yaml -f k8s/server/
# Probe:  kubectl -n paladin port-forward svc/paladin-server 8080:80  &&  curl localhost:8080/health
```

> **Scaling note:** the shipped `configmap.yaml` authenticates with static API keys sourced
> from a Secret, byte-identical in every pod, so horizontal scaling is safe with the shipped
> configuration. If you flip `http.auth.bearer_token.enabled: true`, do not scale past a
> single replica — that store is in-process and per-pod, so a token issued by one replica
> does not verify on another. See ADR-0041.

## Quick Start

### Prerequisites

- Kubernetes cluster (1.24+)
- `kubectl` configured
- Docker registry access (for custom images)

### 1. Create Namespace

```bash
kubectl apply -f k8s/namespace.yaml
```

### 2. Create Secrets

Copy the example secret and fill in your API keys:

```bash
cp k8s/secret.yaml.example k8s/secret.yaml

# Edit secret.yaml with your actual API keys (base64 encoded)
# To encode: echo -n "your-api-key" | base64

kubectl apply -f k8s/secret.yaml
```

### 3. Deploy Dependencies

```bash
# Redis (for queue management)
kubectl apply -f k8s/redis.yaml

# MinIO (for object storage)
kubectl apply -f k8s/minio.yaml

# Wait for dependencies to be ready
kubectl wait --for=condition=ready pod -l app=redis -n paladin --timeout=120s
kubectl wait --for=condition=ready pod -l app=minio -n paladin --timeout=120s
```

### 4. Deploy Paladin

```bash
# Apply configuration
kubectl apply -f k8s/configmap.yaml

# Deploy Paladin
kubectl apply -f k8s/deployment.yaml
kubectl apply -f k8s/service.yaml

# Wait for Paladin to be ready
kubectl wait --for=condition=ready pod -l app=paladin -n paladin --timeout=180s
```

### 5. Verify Deployment

```bash
# Check pod status
kubectl get pods -n paladin

# Check logs
kubectl logs -l app=paladin -n paladin --tail=50

# Test health endpoint
kubectl port-forward -n paladin svc/paladin 8080:80
curl http://localhost:8080/health
```

## Configuration

### Environment Variables

Paladin configuration is managed through ConfigMap and Secrets:

- **ConfigMap** (`k8s/configmap.yaml`): Non-sensitive configuration
- **Secrets** (`k8s/secret.yaml`): API keys and sensitive data

### Resource Limits

Default resource allocation (per pod):

```yaml
requests:
  cpu: "500m"      # 0.5 CPU cores
  memory: "256Mi"  # 256 MB RAM
limits:
  cpu: "2000m"     # 2 CPU cores
  memory: "512Mi"  # 512 MB RAM
```

Adjust based on workload:

```bash
kubectl edit deployment paladin -n paladin
# Modify resources.requests and resources.limits
```

### Scaling

#### Horizontal Scaling

Safe with the `paladin-server` shipped configuration — authentication is by static API keys
from a Secret, byte-identical in every pod. **Not** safe while
`http.auth.bearer_token.enabled: true` in [`server/configmap.yaml`](server/configmap.yaml):
that store is in-process and per-pod, so a token issued by one replica does not verify on
another. Pin to a single replica if you enable it, until the shared-store implementation
exists (ADR-0041).

```bash
# Scale to 5 replicas
kubectl scale deployment paladin -n paladin --replicas=5

# Auto-scaling (requires metrics-server)
kubectl autoscale deployment paladin -n paladin \
  --min=3 --max=10 --cpu-percent=70
```

#### Vertical Scaling

Edit deployment resource requests/limits:

```bash
kubectl edit deployment paladin -n paladin
```

## Health Checks

Paladin includes three types of probes:

- **Liveness Probe**: Restarts container if unhealthy (30s interval)
- **Readiness Probe**: Removes from service if not ready (10s interval)
- **Startup Probe**: Gives 30s for application startup

Health check endpoints:
- `/health` - Overall health status
- `/ready` - Readiness status

## Graceful Shutdown

Both `server/deployment.yaml` and `deployment.yaml` set
`terminationGracePeriodSeconds: 60` on the pod spec (HITL-04, D-23). **The rule: set
`terminationGracePeriodSeconds` to at least twice the configured
`APP_ENGINE_SHUTDOWN_GRACE_SECS`.** 60 is 2x the 30-second default — if you tune the grace
window via the env var below, raise `terminationGracePeriodSeconds` to match, or the
kubelet's SIGKILL deadline can fire while the process is still mid-drain.

Two env vars, both read by `EngineConfig` (`src/config/engine.rs`), control the wait:

- `APP_ENGINE_SHUTDOWN_GRACE_SECS` (default `30`) — how long the process waits, after
  SIGTERM/SIGINT, for in-flight superstep runs to finish before giving up on the
  stragglers.
- `APP_ENGINE_GRACEFUL_SHUTDOWN` (default `true`) — set to `false` to restore the legacy
  no-wait behavior: the process exits immediately on SIGTERM/SIGINT without waiting for
  any in-flight run to drain.

On SIGTERM, an operator observes one of two outcomes per in-flight run: it finishes
inside the grace window and its Waypoint records completion normally, or it is still
running at the deadline, in which case it is aborted, its `NodeExecutionRecord` reads
`Skipped { reason: "shutdown" }`, and its node id is re-listed in the Halted Waypoint's
vanguard so `resume`/`WarEngine::resume` re-runs it exactly once on the next process
start — no work silently vanishes.

## Monitoring

### Prometheus Metrics

Paladin exposes Prometheus metrics on port 9090:

```bash
# Port forward metrics endpoint
kubectl port-forward -n paladin svc/paladin-metrics 9090:9090

# Scrape metrics
curl http://localhost:9090/metrics
```

### Logs

```bash
# Stream logs from all Paladin pods
kubectl logs -f -l app=paladin -n paladin

# View logs from specific pod
kubectl logs -n paladin paladin-<pod-id>

# View logs with timestamps
kubectl logs -n paladin -l app=paladin --timestamps=true
```

## Troubleshooting

### Pod Not Starting

```bash
# Check pod events
kubectl describe pod -l app=paladin -n paladin

# Check logs
kubectl logs -l app=paladin -n paladin --tail=100

# Common issues:
# - Missing secrets: kubectl get secret paladin-secrets -n paladin
# - Image pull errors: Check imagePullPolicy and registry access
# - Resource constraints: Check node capacity
```

### High Memory Usage

```bash
# Check current memory usage
kubectl top pod -l app=paladin -n paladin

# Review configuration
kubectl get configmap paladin-config -n paladin -o yaml

# Adjust Garrison max_entries to reduce memory
kubectl edit configmap paladin-config -n paladin
# Then restart pods:
kubectl rollout restart deployment paladin -n paladin
```

### Connection Issues to Redis/MinIO

```bash
# Verify Redis is running
kubectl get pods -l app=redis -n paladin
kubectl logs -l app=redis -n paladin

# Verify MinIO is running
kubectl get pods -l app=minio -n paladin
kubectl logs -l app=minio -n paladin

# Test connectivity from Paladin pod
kubectl exec -it -n paladin <paladin-pod> -- sh
# nc -zv paladin-redis 6379
# nc -zv paladin-minio 9000
```

### Performance Issues

```bash
# Check resource usage
kubectl top pod -n paladin

# Check for throttling
kubectl describe pod -l app=paladin -n paladin | grep -A 5 "Limits\\|Requests"

# Review metrics
kubectl port-forward -n paladin svc/paladin-metrics 9090:9090
curl http://localhost:9090/metrics | grep paladin_
```

## Production Best Practices

### 1. Use Persistent Volumes

For production, replace `emptyDir` with PersistentVolumeClaims:

```yaml
volumes:
  - name: data
    persistentVolumeClaim:
      claimName: paladin-data
```

### 2. Configure Ingress

Example NGINX Ingress:

```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: paladin
  namespace: paladin
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-prod
spec:
  ingressClassName: nginx
  tls:
    - hosts:
        - paladin.yourdomain.com
      secretName: paladin-tls
  rules:
    - host: paladin.yourdomain.com
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

### 3. Use External Secrets

For production, use External Secrets Operator or similar:

```bash
# Install External Secrets Operator
helm repo add external-secrets https://charts.external-secrets.io
helm install external-secrets external-secrets/external-secrets -n external-secrets-system --create-namespace

# See k8s/secret.yaml.example for ExternalSecret configuration
```

### 4. Enable Network Policies

```yaml
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
    - to:  # Allow external LLM API access
        - namespaceSelector: {}
      ports:
        - protocol: TCP
          port: 443
```

### 5. Configure Pod Disruption Budget

```yaml
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

### 6. Use HPA (Horizontal Pod Autoscaler)

```yaml
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
```

## Cleanup

```bash
# Delete all Paladin resources
kubectl delete -f k8s/

# Or delete namespace (removes everything)
kubectl delete namespace paladin
```

## Additional Resources

- [Kubernetes Documentation](https://kubernetes.io/docs/)
- [Kubectl Cheat Sheet](https://kubernetes.io/docs/reference/kubectl/cheatsheet/)
- [Paladin Performance Tuning](../docs/operations/performance-tuning.md)
- [Paladin Operations Guide](../docs/operations/operations-guide.md)
