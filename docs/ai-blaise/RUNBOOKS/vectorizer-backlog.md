# Runbook: Vectorizer Backlog

`FEATURE: A2` `FEATURE: A3` `FEATURE: A4` `FEATURE: A5` `FEATURE: A6`

## When to run this

The `sidecar/vectorizer` worker has fallen behind on the embedding
queue. `ai.usage_log` records are not advancing, the
`/healthz` probe reports `detail=queue_depth=<N>` where `N` is large
and growing, and the embedding destination column for one or more
tenants has stale rows.

## Pre-conditions

- The `Vectorizer` CR exists for the affected source table
  (`operator/src/crds/vectorizer.rs`, `FEATURE: A8`).
- `sidecar/vectorizer` is enabled in `values-prod.yaml`
  (`sidecars[name=vectorizer].enabled: true`).
- Per-tenant token budgets are configured (`TenantTokenBudget` in
  `sidecar/vectorizer/src/lib.rs`).
- The provider secret referenced by `Vectorizer.spec.secret_ref` exists
  and is readable from the vectorizer pod.
- `ai.usage_log` and `ai.vectorizer_queue_<name>` tables exist on the
  coordinator (created by `companion/src/vector.rs`).

## Detection

1. Read queue depth straight from the vectorizer probe port:

   ```bash
   kubectl -n ai-blaise-citus port-forward \
     deploy/ai-blaise-citus-sidecar-vectorizer 8080:8080 &
   curl -sf localhost:8080/healthz
   ```

   The `detail` field is `queue_depth=<N>`. Any value above the
   tenant's configured `max_qps * 60` warrants this runbook.

2. Inspect the queue at the source of truth:

   ```sql
   SELECT tenant_id, COUNT(*) AS pending,
          MIN(enqueued_at) AS oldest,
          NOW() - MIN(enqueued_at) AS oldest_age
     FROM ai.vectorizer_queue_<name>
    GROUP BY tenant_id
    ORDER BY pending DESC
    LIMIT 20;
   ```

3. Identify why the worker is stalled. Read the usage-log tail per
   provider to see if the provider is rejecting requests:

   ```sql
   SELECT provider, model, COUNT(*) FILTER (WHERE tokens = 0) AS rejected,
          COUNT(*) FILTER (WHERE tokens > 0) AS accepted,
          SUM(cost_micros) AS cost_micros
     FROM ai.usage_log
    WHERE recorded_at > NOW() - INTERVAL '15 minutes'
    GROUP BY provider, model
    ORDER BY rejected DESC;
   ```

4. Confirm tenant token budgets are not exhausted:

   ```bash
   curl -sf localhost:8080/vectorizer/budgets \
     | jq '.[] | select(.remaining_tokens < 1000)'
   ```

5. Confirm the destination column dimensions still match the model:

   ```sql
   SELECT a.atttypmod AS declared_dim,
          v.embedding_model, v.dimensions AS spec_dim
     FROM pg_attribute a
     JOIN ai.vectorizers v
       ON v.destination_table = '<schema>.<table>'::regclass::oid::int
    WHERE a.attrelid = '<destination_table>'::regclass
      AND a.attname = '<destination_column>';
   ```

   A mismatch between `declared_dim` and `spec_dim` is the
   `VectorizerError::InvalidEmbeddingDimension` cause.

## Recovery procedure

Pick the cause that matches the detection output. Run causes in
sequence — each step is idempotent.

1. Provider rate limit. Raise the per-tenant token budget for the
   throttled tenants by patching the `Vectorizer` CR:

   ```bash
   kubectl -n ai-blaise-citus patch vectorizer/<name> --type=merge -p \
     '{"spec":{"scheduling":{"perTenantTokensPerMinute": <new_value>}}}'
   ```

   Reload the worker so it picks up the new budget:

   ```bash
   kubectl -n ai-blaise-citus rollout restart \
     deploy/ai-blaise-citus-sidecar-vectorizer
   kubectl -n ai-blaise-citus rollout status \
     deploy/ai-blaise-citus-sidecar-vectorizer
   ```

2. Provider outage. Switch the embedding provider for the affected
   `Vectorizer` to a secondary route. The supported providers are
   enumerated in `EmbeddingProvider`
   (`sidecar/vectorizer/src/lib.rs`):

   ```bash
   kubectl -n ai-blaise-citus patch vectorizer/<name> --type=merge -p \
     '{"spec":{"embedding_provider":"voyage","embedding_model":"voyage-3-large","secret_ref":"voyage-prod"}}'
   ```

   The worker will requeue in-flight jobs against the new provider on
   restart.

3. Network blip. Shrink the per-batch size so partial failures cost
   less; this is the `ChunkingSpec.max_tokens` field on the CR:

   ```bash
   kubectl -n ai-blaise-citus patch vectorizer/<name> --type=merge -p \
     '{"spec":{"chunking":{"max_tokens": 256, "overlap_tokens": 32}}}'
   ```

4. Dimension change. If the model now emits a different vector size,
   add a new destination column at the correct dimension and let the
   worker backfill it; do not silently truncate the existing column:

   ```sql
   ALTER TABLE <schema>.<table>
     ADD COLUMN <new_column> vector(<new_dim>);
   ```

   ```bash
   kubectl -n ai-blaise-citus patch vectorizer/<name> --type=merge -p \
     '{"spec":{"destination":{"column":"<new_column>","dimensions":<new_dim>}}}'
   ```

5. Drain the queue. Force a one-shot drain pass:

   ```bash
   curl -sf -X POST localhost:8080/vectorizer/drain \
     -H 'content-type: application/json' \
     -d '{"max_batches": 200, "max_seconds": 300}'
   ```

6. Monitor catch-up. The probe port emits the same `queue_depth` field
   on every poll. Wait until `queue_depth < (max_qps * 5)` before
   marking the incident resolved.

## Verification

1. Queue depth is shrinking monotonically over a 10-minute window:

   ```bash
   for i in $(seq 1 10); do
     curl -sf localhost:8080/healthz | jq -r '.detail'
     sleep 60
   done
   ```

   Expected: each sample's `queue_depth=` value is lower than the
   previous one.

2. Usage log shows accepted tokens against the new or unchanged
   provider:

   ```sql
   SELECT provider, model, SUM(tokens) AS tokens, SUM(cost_micros) AS cost
     FROM ai.usage_log
    WHERE recorded_at > NOW() - INTERVAL '10 minutes'
    GROUP BY provider, model;
   ```

   Expected: at least one row where `tokens > 0`.

3. Tenant budgets are not stuck at zero:

   ```bash
   curl -sf localhost:8080/vectorizer/budgets \
     | jq '.[] | select(.remaining_tokens == 0)'
   ```

   Expected: `[]` (or, for known-throttled tenants, an explicit allow
   list logged in the incident ticket).

4. The canonical execution report still parses, proving the worker
   contract is intact:

   ```bash
   cargo run -q -p ai_blaise_citus_sidecar_vectorizer -- run-canonical \
     | tail -1
   ```

## Rollback

If switching providers or shrinking the batch makes the backlog worse:

1. Revert the `Vectorizer` CR to its previous spec:

   ```bash
   kubectl -n ai-blaise-citus rollout undo \
     deploy/ai-blaise-citus-sidecar-vectorizer
   ```

2. If the dimension column was changed, drop the new column and revert
   the CR:

   ```sql
   ALTER TABLE <schema>.<table> DROP COLUMN <new_column>;
   ```

   ```bash
   kubectl -n ai-blaise-citus patch vectorizer/<name> --type=merge -p \
     '{"spec":{"destination":{"column":"<old_column>","dimensions":<old_dim>}}}'
   ```

3. If the queue grows past a safety threshold during rollback, pause
   intake by scaling the worker to zero and let upstream producers buffer
   in `ai.vectorizer_queue_<name>`. Do not delete queue rows.

## References

- Related: `tenant-migration.md`, `rebalance-stuck.md`.
- CRD: `operator/src/crds/vectorizer.rs` (`FEATURE: A8`).
- Companion module: `sidecar/vectorizer/src/lib.rs`
  (`FEATURE: A2`, `A3`, `A4`, `A5`, `A6`),
  `companion/src/vector.rs` (`ai.vectorizer_queue_*`, `ai.usage_log`).
- Probe and metrics: `sidecar/shared/README.md`.
- agentmemory pattern: `CITUS-VECTORIZER-BACKLOG-<tenant>-<UTC>`
  recorded against `:3911` with the queue depth before and after, plus
  the resolved cause from the cause list above.
