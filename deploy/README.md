# Deployment

Production deployment manifests for the **ontostar** (open-ontologies) MCP server.

The MCP server speaks the Streamable HTTP transport at `http://<host>:3050/mcp`.

## Layout

```
Dockerfile                          # Multi-stage build (builder + debian:bookworm-slim)
docker/
  docker-compose.yml                # Local / single-host deployment
  config.toml                       # Canonical config consumed by the container
deploy/
  k8s/
    namespace.yaml
    configmap.yaml                  # Mounts /config/config.toml
    secret.example.yaml             # Template — copy to secret.yaml & fill in
    pvc.yaml                        # 5 GiB RWO claim mounted at /data
    service.yaml                    # ClusterIP on port 3050
    deployment.yaml                 # 1 replica, 500m / 1Gi requests
  helm/
    Chart.yaml
    values.yaml                     # Override per environment
    templates/                      # ConfigMap, Secret, PVC, Service, Deployment
```

## Build & push the image

```bash
# Default (no extra features)
docker build -t ghcr.io/fabio-rovai/ontostar:latest .

# With local ONNX embeddings
docker build --build-arg FEATURES=embeddings \
             -t ghcr.io/fabio-rovai/ontostar:latest-embeddings .

# Smoke test — proves the binary made it into the image
docker run --rm ghcr.io/fabio-rovai/ontostar:latest --version

docker push ghcr.io/fabio-rovai/ontostar:latest
```

## Option 1 — docker-compose (single host)

```bash
# 1. Create .env at repo root with at minimum:
#       GROQ_API_KEY=sk-...
#    .env is gitignored and excluded from the docker build context.
cp .env.example .env
$EDITOR .env

# 2. Bring it up
docker compose -f docker/docker-compose.yml up -d --build

# 3. Verify
curl -fsS http://localhost:3050/mcp || true   # MCP endpoint (POST/SSE)
docker compose -f docker/docker-compose.yml logs -f ontostar
```

The pm4py sidecar is gated behind a profile and disabled by default. Enable
with `docker compose --profile pm4py up`.

## Option 2 — raw kubectl

Apply order matters — namespace first, then PVC/ConfigMap/Secret, then
Service, then Deployment last so probes reference an existing volume.

```bash
# 1. Create the secret OUT OF BAND. Never commit a real key.
cp deploy/k8s/secret.example.yaml deploy/k8s/secret.yaml
$EDITOR deploy/k8s/secret.yaml         # set GROQ_API_KEY

kubectl apply -f deploy/k8s/namespace.yaml
kubectl apply -f deploy/k8s/configmap.yaml
kubectl apply -f deploy/k8s/secret.yaml
kubectl apply -f deploy/k8s/pvc.yaml
kubectl apply -f deploy/k8s/service.yaml
kubectl apply -f deploy/k8s/deployment.yaml

kubectl -n ontostar rollout status deploy/ontostar
kubectl -n ontostar port-forward svc/ontostar 3050:3050
```

## Option 3 — Helm

```bash
helm lint deploy/helm/

# Create the secret first (recommended) and reference it via existingSecret:
kubectl create ns ontostar
kubectl -n ontostar create secret generic ontostar-secrets \
    --from-literal=GROQ_API_KEY="$GROQ_API_KEY"

helm upgrade --install ontostar deploy/helm/ \
    --namespace ontostar \
    --set secret.existingSecret=ontostar-secrets \
    --set image.tag=latest
```

## Required secrets

The `Secret` (k8s) or `.env` (compose) must contain at minimum:

| Key                                   | Required | Purpose                                    |
|---------------------------------------|----------|--------------------------------------------|
| `GROQ_API_KEY`                        | yes      | Default LLM provider for enrichment tools  |
| `OPENAI_API_KEY`                      | no       | If `[llm] provider = "openai"`             |
| `OPEN_ONTOLOGIES_EMBEDDINGS_API_KEY`  | no       | OpenAI-compatible embedding gateways       |
| `OPEN_ONTOLOGIES_EMBEDDINGS_BASE_URL` | no       | Override embedding endpoint                |

### Secret rotation

1. Update the upstream secret store (Vault, GSM, ASM, ESO, sealed-secrets).
2. `kubectl -n ontostar rollout restart deploy/ontostar` — pods re-read the
   Secret on restart (no hot reload).
3. Verify with `kubectl -n ontostar logs deploy/ontostar | grep -i 'llm\|groq'`.

For zero-downtime rotation, run two replicas behind the Service during the
window, then scale back down.

## Multi-tenant deployment

The binary reads `OPEN_ONTOLOGIES_TENANT_ID` and namespaces in-memory state
per tenant. Two patterns:

**A. One Deployment per tenant (recommended for isolation):**

```bash
helm upgrade --install ontostar-acme deploy/helm/ \
    --namespace ontostar-acme --create-namespace \
    --set tenant.id=acme \
    --set persistence.size=20Gi
```

**B. Single Deployment, tenant per request:** the binary supports passing
tenant via MCP request metadata. In that mode set `tenant.id=multi` and size
the PVC for the union of all tenants.

## Persistent volume sizing

`/data` holds the Oxigraph DB, the compile cache, and version snapshots.
Rule of thumb:

| Workload                              | Suggested size |
|---------------------------------------|----------------|
| One small ontology (<10k triples)     | 1 GiB          |
| Default single-tenant (this chart)    | 5 GiB          |
| Multi-ontology / many snapshots       | 20 GiB         |
| Large enterprise (millions of triples)| 100 GiB+       |

PVCs are RWO — to scale beyond one replica you must shard per tenant
(StatefulSet) or front a shared SPARQL endpoint and disable the embedded
store.

## Verification

```bash
docker build -t ontostar:test .
docker run --rm ontostar:test --version
yamllint deploy/k8s/
helm lint deploy/helm/
```
