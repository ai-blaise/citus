#!/usr/bin/env bash
set -euo pipefail

# FEATURE: Sec7 Sec8
# Live security proof: External Secrets Operator reconciles reference-only
# ExternalSecrets into Kubernetes Secrets, runtime ServiceAccounts cannot read
# Secret objects through the API, and mounted TLS Secret material enforces TLS
# 1.3 plus client certificates over in-cluster traffic.

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

cluster="${SEC78_KIND_CLUSTER:-ai-blaise-sec78-live}"
ns="${SEC78_NAMESPACE:-ai-blaise-sec78}"
node_image="${SEC78_KIND_NODE_IMAGE:-kindest/node:v1.30.0}"
eso_chart_version="${SEC78_ESO_CHART_VERSION:-0.10.7}"
python_image="${SEC78_PYTHON_IMAGE:-python:3.12-alpine}"
evidence_file="${SEC78_EVIDENCE:-artifacts/security-external-secrets-tls-live-evidence.tsv}"
keep_cluster="${KEEP_KIND_CLUSTER:-0}"
work=""

require_tool() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "$1 is required for Sec7/Sec8 live smoke" >&2
    exit 1
  fi
}

cleanup(){
  if [[ -n "${work}" ]]; then
    rm -rf "${work}"
  fi
  if [[ "${keep_cluster}" != "1" ]]; then
    kind delete cluster --name "${cluster}" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

require_tool base64
require_tool docker
require_tool helm
require_tool kind
require_tool kubectl
require_tool openssl
require_tool sha256sum
mkdir -p "$(dirname "${evidence_file}")"
kind delete cluster --name "$cluster" >/dev/null 2>&1 || true
kind create cluster --name "$cluster" --image "$node_image" --wait 120s >/dev/null
kubectl config use-context "kind-$cluster" >/dev/null
helm repo add external-secrets https://charts.external-secrets.io >/dev/null 2>&1 || true
helm repo update external-secrets >/dev/null
helm upgrade --install external-secrets external-secrets/external-secrets --version "${eso_chart_version}" --namespace external-secrets --create-namespace --set installCRDs=true --wait --timeout 5m >/dev/null
kubectl wait --for=condition=Established crd/externalsecrets.external-secrets.io --timeout=120s >/dev/null
kubectl create namespace "$ns" >/dev/null
work=$(mktemp -d)
openssl genrsa -out "$work/ca.key" 2048 >/dev/null 2>&1
openssl req -x509 -new -nodes -key "$work/ca.key" -sha256 -days 2 -subj "/CN=ai-blaise-sec78-ca" -out "$work/ca.crt" >/dev/null 2>&1
openssl genrsa -out "$work/server.key" 2048 >/dev/null 2>&1
openssl req -new -key "$work/server.key" -subj "/CN=sec78-tls-server.${ns}.svc" -out "$work/server.csr" >/dev/null 2>&1
cat > "$work/server.ext" <<EOF
subjectAltName=DNS:sec78-tls-server,DNS:sec78-tls-server.${ns}.svc,DNS:sec78-tls-server.${ns}.svc.cluster.local
extendedKeyUsage=serverAuth
EOF
openssl x509 -req -in "$work/server.csr" -CA "$work/ca.crt" -CAkey "$work/ca.key" -CAcreateserial -out "$work/server.crt" -days 2 -sha256 -extfile "$work/server.ext" >/dev/null 2>&1
openssl genrsa -out "$work/client.key" 2048 >/dev/null 2>&1
openssl req -new -key "$work/client.key" -subj "/CN=ai-blaise-sec78-client" -out "$work/client.csr" >/dev/null 2>&1
cat > "$work/client.ext" <<EOF
extendedKeyUsage=clientAuth
EOF
openssl x509 -req -in "$work/client.csr" -CA "$work/ca.crt" -CAkey "$work/ca.key" -CAcreateserial -out "$work/client.crt" -days 2 -sha256 -extfile "$work/client.ext" >/dev/null 2>&1
indent(){ sed "s/^/              /" "$1"; }
cat > "$work/secstore.yaml" <<EOF
apiVersion: external-secrets.io/v1beta1
kind: SecretStore
metadata:
  name: ai-blaise-cluster-secrets
spec:
  provider:
    fake:
      data:
        - key: /postgres/pool/password
          value: sec78-postgres-password
        - key: /tls/pool/server
          valueMap:
            tls.crt: |-
$(indent "$work/server.crt")
            tls.key: |-
$(indent "$work/server.key")
            ca.crt: |-
$(indent "$work/ca.crt")
        - key: /tls/pool/client
          valueMap:
            tls.crt: |-
$(indent "$work/client.crt")
            tls.key: |-
$(indent "$work/client.key")
            ca.crt: |-
$(indent "$work/ca.crt")
EOF
kubectl -n "$ns" apply -f "$work/secstore.yaml" >/dev/null
cat <<EOF | kubectl -n "$ns" apply -f - >/dev/null
apiVersion: external-secrets.io/v1beta1
kind: ExternalSecret
metadata:
  name: ai-blaise-citus-pool-postgres-auth
spec:
  refreshInterval: 5m
  secretStoreRef:
    name: ai-blaise-cluster-secrets
    kind: SecretStore
  target:
    name: ai-blaise-citus-pool-postgres-auth
    creationPolicy: Owner
  data:
    - secretKey: password
      remoteRef:
        key: /postgres/pool/password
---
apiVersion: external-secrets.io/v1beta1
kind: ExternalSecret
metadata:
  name: ai-blaise-citus-pool-tls
spec:
  refreshInterval: 5m
  secretStoreRef:
    name: ai-blaise-cluster-secrets
    kind: SecretStore
  target:
    name: ai-blaise-citus-pool-tls
    creationPolicy: Owner
    template:
      type: kubernetes.io/tls
  dataFrom:
    - extract:
        key: /tls/pool/server
---
apiVersion: external-secrets.io/v1beta1
kind: ExternalSecret
metadata:
  name: ai-blaise-citus-pool-client-tls
spec:
  refreshInterval: 5m
  secretStoreRef:
    name: ai-blaise-cluster-secrets
    kind: SecretStore
  target:
    name: ai-blaise-citus-pool-client-tls
    creationPolicy: Owner
    template:
      type: kubernetes.io/tls
  dataFrom:
    - extract:
        key: /tls/pool/client
EOF
kubectl -n "$ns" wait externalsecret/ai-blaise-citus-pool-postgres-auth --for=condition=Ready --timeout=120s >/dev/null
kubectl -n "$ns" wait externalsecret/ai-blaise-citus-pool-tls --for=condition=Ready --timeout=120s >/dev/null
kubectl -n "$ns" wait externalsecret/ai-blaise-citus-pool-client-tls --for=condition=Ready --timeout=120s >/dev/null
for s in ai-blaise-citus-pool-postgres-auth ai-blaise-citus-pool-tls ai-blaise-citus-pool-client-tls; do kubectl -n "$ns" get secret "$s" >/dev/null; done
kubectl -n "$ns" create serviceaccount ai-blaise-citus-pool >/dev/null
can_i=$(kubectl -n "$ns" auth can-i get secrets --as "system:serviceaccount:${ns}:ai-blaise-citus-pool" || true)
[[ "$can_i" == no ]]
cat <<EOF | kubectl -n "$ns" apply -f - >/dev/null
apiVersion: v1
kind: ConfigMap
metadata:
  name: sec78-python
  labels:
    app: sec78
binaryData: {}
data:
  server.py: |
    import socket, ssl
    ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    ctx.minimum_version = ssl.TLSVersion.TLSv1_3
    ctx.verify_mode = ssl.CERT_REQUIRED
    ctx.load_cert_chain("/tls/tls.crt", "/tls/tls.key")
    ctx.load_verify_locations("/tls/ca.crt")
    sock = socket.socket()
    sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    sock.bind(("0.0.0.0", 8443))
    sock.listen(20)
    while True:
      conn, _ = sock.accept()
      try:
        with ctx.wrap_socket(conn, server_side=True) as tls:
          tls.recv(1024)
          tls.sendall(b"ok\\n")
      except Exception:
        conn.close()
  client.py: |
    import os, socket, ssl, sys
    mode = os.environ.get("MODE", "success")
    host = "sec78-tls-server.ai-blaise-sec78.svc"
    ctx = ssl.create_default_context(cafile="/client/ca.crt")
    if mode == "tls12":
      ctx.minimum_version = ssl.TLSVersion.TLSv1_2
      ctx.maximum_version = ssl.TLSVersion.TLSv1_2
    else:
      ctx.minimum_version = ssl.TLSVersion.TLSv1_3
    if mode != "no-cert":
      ctx.load_cert_chain("/client/tls.crt", "/client/tls.key")
    try:
      with socket.create_connection((host, 8443), timeout=10) as raw:
        with ctx.wrap_socket(raw, server_hostname=host) as tls:
          tls.sendall(b"ping")
          data = tls.recv(16)
          if mode == "success" and data == b"ok\\n":
            sys.exit(0)
          print("unexpected success", data, file=sys.stderr)
          sys.exit(1)
    except Exception as exc:
      if mode in ("no-cert", "tls12"):
        print("expected failure", mode, exc)
        sys.exit(0)
      print("unexpected failure", mode, exc, file=sys.stderr)
      sys.exit(1)
EOF
cat <<EOF | kubectl -n "$ns" apply -f - >/dev/null
apiVersion: apps/v1
kind: Deployment
metadata:
  name: sec78-tls-server
spec:
  replicas: 1
  selector:
    matchLabels:
      app: sec78-tls-server
  template:
    metadata:
      labels:
        app: sec78-tls-server
    spec:
      serviceAccountName: ai-blaise-citus-pool
      containers:
        - name: server
          image: ${python_image}
          command: ["python", "/scripts/server.py"]
          ports:
            - containerPort: 8443
          volumeMounts:
            - name: scripts
              mountPath: /scripts
            - name: tls
              mountPath: /tls
              readOnly: true
      volumes:
        - name: scripts
          configMap:
            name: sec78-python
        - name: tls
          secret:
            secretName: ai-blaise-citus-pool-tls
---
apiVersion: v1
kind: Service
metadata:
  name: sec78-tls-server
spec:
  selector:
    app: sec78-tls-server
  ports:
    - name: tls
      port: 8443
      targetPort: 8443
EOF
kubectl -n "$ns" rollout status deployment/sec78-tls-server --timeout=180s >/dev/null
run_job(){
  local name=$1 mode=$2
  cat <<EOF | kubectl -n "$ns" apply -f - >/dev/null
apiVersion: batch/v1
kind: Job
metadata:
  name: $name
spec:
  backoffLimit: 0
  template:
    spec:
      restartPolicy: Never
      serviceAccountName: ai-blaise-citus-pool
      containers:
        - name: client
          image: ${python_image}
          command: ["python", "/scripts/client.py"]
          env:
            - name: MODE
              value: "$mode"
          volumeMounts:
            - name: scripts
              mountPath: /scripts
            - name: client-tls
              mountPath: /client
              readOnly: true
      volumes:
        - name: scripts
          configMap:
            name: sec78-python
        - name: client-tls
          secret:
            secretName: ai-blaise-citus-pool-client-tls
EOF
  kubectl -n "$ns" wait --for=condition=Complete "job/$name" --timeout=120s >/dev/null
}
run_job sec78-client-success success
run_job sec78-client-no-cert no-cert
run_job sec78-client-tls12 tls12
password_sha="$(kubectl -n "${ns}" get secret ai-blaise-citus-pool-postgres-auth -o jsonpath="{.data.password}" | base64 -d | sha256sum | sed "s/ .*//")"
git_sha="$(git rev-parse --short=12 HEAD)"
{
  printf "feature\tassertion\tstatus\tdetail\n"
  printf "Sec7\texternal_secrets_operator\tpassed\tchart=external-secrets-%s namespace=external-secrets\n" "${eso_chart_version}"
  printf "Sec7\tfake_provider_secret_sync\tpassed\tstore=ai-blaise-cluster-secrets secrets=3 password_sha256=%s\n" "${password_sha}"
  printf "Sec7\truntime_secret_api_denied\tpassed\tserviceaccount=ai-blaise-citus-pool can_get_secrets=%s\n" "${can_i}"
  printf "Sec8\ttls_secret_mount\tpassed\tsecret=ai-blaise-citus-pool-tls keys=tls.crt,tls.key,ca.crt\n"
  printf "Sec8\ttls13_mtls_success\tpassed\tserver=sec78-tls-server client=sec78-client-success image=%s git=%s\n" "${python_image}" "${git_sha}"
  printf "Sec8\tclient_cert_required\tpassed\tjob=sec78-client-no-cert expected_failure=true\n"
  printf "Sec8\ttls12_rejected\tpassed\tjob=sec78-client-tls12 expected_failure=true\n"
} >"${evidence_file}"
cat "${evidence_file}"
echo "ai_blaise_citus Sec7/Sec8 external-secrets TLS live smoke passed"
