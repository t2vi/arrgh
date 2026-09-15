# Kubernetes Deployment

> **Best-effort / community-supported.** These manifests are examples for a home/single-node cluster (e.g. [k3s](https://k3s.io)) — they are **not** exercised by this project's CI the way `docker-compose.yml` is. If something's wrong, please open an issue, but expect a slower turnaround than the Docker/Portainer paths.

## Why single-replica

*ARRgh!* stores its data in a single SQLite file (`DatabasePath` / `DATABASE_URL`) — one writer at a time. The example `k8s/deployment.yaml` hardcodes `replicas: 1` and uses `strategy: Recreate` (stops the old pod before starting the new one) so two pods never hold the DB file open simultaneously during a rollout. Don't raise the replica count — SQLite is a given for this app, not a migration target (see `CLAUDE.md`'s "Rust Conventions" section).

## What's included

| File | Kind | Purpose |
|---|---|---|
| `k8s/configmap.yaml` | ConfigMap | Non-secret env vars — same names as `docker-compose.yml` |
| `k8s/pvc.yaml` | PersistentVolumeClaim | `ReadWriteOnce`, backs both the DB file and the downloads directory |
| `k8s/deployment.yaml` | Deployment | The `arrgh` app itself, single replica |
| `k8s/service.yaml` | Service | `ClusterIP` by default |

**Out of scope for these examples**: `plugin-host` and `cloakbrowser` (the app needs a reachable plugin-host to browse/download sources — deploy it separately, e.g. via the existing `docker-compose.yml` on another host, or add your own Deployment/Service for `ghcr.io/t2vi/plugin-host` and point `PLUGIN_URLS` at it), Ingress (too cluster-specific for one example — front the `Service` with whatever Ingress controller your cluster already has), Helm chart / Kustomize overlay (not provided — see `specs/010-readme-deployment-guide/research.md` for why).

## Deploy

```bash
kubectl apply -f k8s/
kubectl rollout status deployment/arrgh
```

Then reach it (no Ingress yet):

```bash
kubectl port-forward svc/arrgh 8282:8080
```

Open `http://localhost:8282`.

## Configuration

Edit `k8s/configmap.yaml` before applying, or `kubectl edit configmap arrgh-config` after — same variable names as [docker-compose.md](docker-compose.md#environment-variables): `DATABASE_URL`, `DOWNLOAD_DIR`, `PLUGIN_URLS`, `LOG_LEVEL`. Changing the ConfigMap doesn't restart the pod automatically — `kubectl rollout restart deployment/arrgh` to pick it up.

**`JWT_SECRET`**: without it, the container auto-generates one at boot and sessions reset on every pod restart. For a stable secret, create a `Secret` and reference it from `k8s/deployment.yaml`'s commented `env:` block:

```bash
kubectl create secret generic arrgh-secret --from-literal=jwt-secret="$(openssl rand -hex 32)"
```

## Storage

`k8s/pvc.yaml` requests `20Gi` by default — resize the `resources.requests.storage` field to fit your library. Whatever `StorageClass` your cluster's default provisioner uses (e.g. k3s's `local-path`) needs to support `ReadWriteOnce`, which nearly all do.

## Upgrading

```bash
kubectl set image deployment/arrgh arrgh=ghcr.io/t2vi/arrgh:latest
kubectl rollout status deployment/arrgh
```

Or re-`kubectl apply -f k8s/` after pulling repo changes. Migrations run automatically on pod startup, same as Docker.

## Removing

```bash
kubectl delete -f k8s/
```

The `PersistentVolumeClaim` (and your data) survives this unless you also `kubectl delete pvc arrgh-data`.
