# Production-bounded CitusCluster reconciliation

`FEATURE: S4` supplies a tightly scoped live apply path for coordinator-worker
clusters. It does not promote coordinator-less Kubernetes apply and it does not
replace the independent release-authority evidence required by Command Center.

## Apply contract

Set `AI_BLAISE_OPERATOR_EXECUTION_MODE=apply` and optionally restrict the
runtime with `AI_BLAISE_OPERATOR_CONTROLLERS=citus_cluster`. Apply-ready CRs
must use `citus.ai-blaise.io/v2`, a name of at most 40 characters, an operand
image pinned by lowercase `@sha256`, coordinator-worker topology, at least two
logical workers, exactly the `citus` and `ai_blaise_citus` extensions, no
TimescaleDB cohabitation, no inline pool or sidecars, no more than 32 worker
groups, nine replicas per group, or 32 databases, and the production block below.
The image must contain `psql` and the exact extension versions declared by the
CR.

```yaml
apiVersion: citus.ai-blaise.io/v2
kind: CitusCluster
metadata:
  name: production
spec:
  image: ghcr.io/ai-blaise/citus@sha256:<64-lowercase-hex-characters>
  workers: 2
  coordinators: 3
  coordinatorLess: false
  extensions: [citus, ai_blaise_citus]
  storageClass: fast-ssd
  production:
    postgresMajor: 17
    postgresUid: 999
    postgresGid: 999
    workerReplicas: 3
    storageSize: 100Gi
    clusterDomain: cluster.local
    databases: [app]
    extensionVersions:
      citus: 13.2-1
      companion: 0.1.2
    nodeTls:
      serverCaSecret: production-server-ca
      serverTlsSecret: production-server-tls
      superuserSecret: production-superuser
      sslMode: verify-full
      sslRootCert: /controller/certificates/server-ca.crt
      connectTimeoutSeconds: 5
    bootstrap:
      backoffLimit: 3
      activeDeadlineSeconds: 600
```

Generate the authoritative CRD with:

```bash
cargo run -p ai_blaise_citus_operator -- print-citus-cluster-crd
```

The CA Secret must contain a currently valid X.509 CA in `ca.crt`. The server
Secret must have type `kubernetes.io/tls`, a currently valid certificate in
`tls.crt` signed by the configured CA, and a PEM private key in `tls.key`.
The private key must be a supported PKCS#8, PKCS#1 RSA, or SEC1 EC key and its
public key must match `tls.crt`.
Both certificate Secrets must carry the `cnpg.io/reload` label so CNPG reloads
rotated material. Because the example shares one
server certificate across the CNPG groups, its DNS SAN set must contain all
four service forms for the coordinator and every worker. For namespace
`database`, these include:

```text
production-coordinator-rw
production-coordinator-rw.database
production-coordinator-rw.database.svc
production-coordinator-rw.database.svc.cluster.local
production-worker-0-rw
production-worker-0-rw.database
production-worker-0-rw.database.svc
production-worker-0-rw.database.svc.cluster.local
production-worker-1-rw
production-worker-1-rw.database
production-worker-1-rw.database.svc
production-worker-1-rw.database.svc.cluster.local
```

The superuser Secret must have type `kubernetes.io/basic-auth`, username
`postgres`, and a 16-to-256-byte password without NUL or newline characters.
The operator only gets these Secrets; it
cannot list, create, patch, update, or delete them. A Secret resource-version,
CR generation, or rendered CNPG/bootstrap contract change produces a new
reconcile hash and therefore a new immutable bootstrap ConfigMap and Job,
including password rotation and spec reverts.

## Reconcile and readiness semantics

Before mutating children, the controller validates the complete apply contract,
all referenced Secrets, and the live owner-UID inventory of CNPG groups. The
live inventory closes the status-write crash window and prevents a contraction
from ignoring an already-created group. A missing child is created with create-only
semantics; an existing child is server-side applied only after its controller
owner UID is verified and its UID/resource-version preconditions are attached.
This prevents a colliding name or delete/recreate race from being force-adopted.
The controller manages a namespaced CNPG
`ImageCatalog`, one coordinator CNPG `Cluster`, and one CNPG `Cluster` per
logical worker. Every child has a controller owner reference and the exact
digest/version/TLS reconcile annotations. The CR finalizer delegates deletion
to owner-reference garbage collection.

Every CNPG group receives the exact `citus.node_conninfo`:

```text
sslmode=verify-full sslrootcert=/controller/certificates/server-ca.crt connect_timeout=5
```

The bootstrap Job is created only after `status.readyInstances` equals the
desired instance count for every group, CNPG reports its healthy phase and
`Ready=True`, `status.image` equals the requested digest, and no PVC resize is
pending. If CNPG supplies a top-level or Ready-condition
`observedGeneration`, it must equal the child's current generation. Current
CNPG v1 does not publish either acknowledgement, so fields without another
exact live fence—storage size/class, PostgreSQL major, PostgreSQL UID/GID, and
the initdb database—are immutable after child creation and fail closed before
the ImageCatalog or any Cluster is mutated. Their reviewed change path is
data migration into a newly provisioned cluster. Exact Citus and companion
extension versions are likewise immutable on an existing child: the bootstrap
contract verifies or initially creates those versions but never performs an
implicit extension upgrade. Version changes require the separately reviewed
extension-upgrade path and fail before child mutation. Image and replica changes
remain fenced by CNPG's exact running-image and ready-instance status.

Before a hash-named bootstrap Job can be created, the controller enumerates
every non-terminating CNPG instance Pod by the exact applied Cluster owner UID.
Pod inventory calls are per desired Cluster with exact closed label selectors
and a `desired instances + 1` server-side limit, so unrelated namespace size
cannot amplify memory use and an overfull result fails readiness closed.
Each Pod must be Ready and must complete a direct PostgreSQL SSL negotiation
whose CA and service-name verification succeeds and whose peer leaf DER hash
equals the current `serverTLSSecret` leaf. This explicit fence covers CNPG v1's
lack of a certificate-reload acknowledgement and prevents a same-CA leaf
rotation from admitting bootstrap or Ready while any primary or failover
candidate still serves the superseded leaf.

The hash-named bootstrap Job is bounded by the CR's
retry/deadline settings, uses the digest-pinned operand image, runs non-root
with a read-only root filesystem, drops all capabilities, mounts no service
account token, has CPU/memory/ephemeral-storage requests and limits, bounds its
writable temporary volume, and reads its script from an immutable ConfigMap.
Before its first write, the Job repeatedly requires every live endpoint to
accept the exact current password over `verify-full` TLS and expose the exact
requested `citus.node_conninfo`; a stale CNPG Ready status therefore cannot
admit stale-configuration writes. Success means that every declared database exists on every primary endpoint, exact Citus and
companion versions match on every endpoint/database, every administrator
session is TLS, `pg_dist_authinfo` is current, all workers are registered and
metadata-synced with distinct positive worker group IDs, Citus worker commands use TLS, and normalized `pg_dist_node`
JSON is byte-equal across every endpoint/database.

The CR is `Ready` only after both CNPG readiness and bootstrap Job completion.
Status carries `observedGeneration`, the spec-and-Secret reconcile hash,
per-CNPG desired/ready instances, the immutable Job name, exact expected
versions, exact node conninfo, errors, and stable transition conditions.
Failed Jobs do not retry without bound. The controller re-reads the exact UID
and resource version of every referenced Secret before bootstrap creation and
again immediately before publishing Ready, closing a rotation race between
certificate probing, Job creation, and status. Before a new bootstrap revision starts,
every older non-terminal Job is deleted with foreground propagation and the
controller waits for its Pods to terminate; stale credential generations
therefore cannot write after the replacement verifies. Deletion uses the same
fence: the finalizer remains installed until every still-runnable bootstrap
Job is foreground-deleted and no longer present, after which owner-reference
garbage collection removes non-writing children. Removing a logical CNPG group fails
closed because shard evacuation is outside this controller; adding groups and
changing replica counts remain declarative CNPG operations. A superseded Job
and ConfigMap are discovered from the owner/managed-label inventory and deleted
only after their replacement succeeds and only after their controller owner
reference is verified. Inventory-based cleanup is retried after crashes and
does not depend on a single name retained in status.

## Minimum namespace RBAC

The following rules are the minimum for a CitusCluster-only operator. The
ServiceAccount, Role, and RoleBinding are deployment-owned because the full
chart lives in Command Center.

The operator's NetworkPolicy must also allow egress to TCP 5432 on every CNPG
instance Pod in its namespace. The live leaf fence accepts only a parsed
literal `status.podIP` and never resolves API-provided text through DNS; it
uses the expected `*-rw.<namespace>.svc.<cluster-domain>` identity solely as
TLS SNI. A blocked route or malformed/non-IP Pod status fails readiness closed.

```yaml
rules:
  - apiGroups: [citus.ai-blaise.io]
    resources: [citusclusters]
    verbs: [get, list, watch, patch, update]
  - apiGroups: [citus.ai-blaise.io]
    resources: [citusclusters/status]
    verbs: [get, patch, update]
  - apiGroups: [citus.ai-blaise.io]
    resources: [citusclusters/finalizers]
    verbs: [update]
  - apiGroups: [postgresql.cnpg.io]
    resources: [clusters, imagecatalogs]
    verbs: [get, list, create, patch]
  - apiGroups: ['']
    resources: [configmaps]
    verbs: [get, list, create, patch, delete]
  - apiGroups: ['']
    resources: [secrets]
    verbs: [get]
  - apiGroups: ['']
    resources: [pods]
    verbs: [get, list]
  - apiGroups: [batch]
    resources: [jobs]
    verbs: [get, list, create, patch, delete]
  - apiGroups: ['']
    resources: [events]
    verbs: [create, patch]
```

## Promotion evidence remains external

Source tests and rendered manifests cannot manufacture live operational
evidence. Promotion still requires one independently reviewed, immutable
artifact for each of the eight Command Center checks:

1. `live_watch`: sustained watch, reconnect, and resource-version recovery.
2. `reconcile`: desired-state mutation by the exact operator image.
3. `status`: observed-generation and condition-transition history.
4. `recovery`: controller and database recovery after deliberate interruption.
5. `upgrade`: forward upgrade with data and service continuity.
6. `failover`: induced primary failure with bounded recovery and consistency.
7. `immutable_images`: running operator and database image IDs equal the
   reviewed digests.
8. `node_tls`: the complete coordinator/worker TLS, exact-version,
   database-local topology/capability, quarantine/containment, interrupted
   decision, prepared-transaction, session-drain, reviewed-function-digest,
   DDL/parameter ACL, large-object, and `pg_init_privs` closure required by the
   promotion contract.

The evidence document, per-artifact hashes, reviewer identity, source commit,
image digests, and exact extension versions must then be compiled into the
release-authority allowlist. Until that occurs, this implementation is
deployable source—not authorization to render a production release.
