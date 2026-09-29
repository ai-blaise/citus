# Native CDC tuple-ownership witness

This test mechanically extracts four actual product functions from
`src/backend/distributed/cdc/cdc_decoder.c`: `GetTupleForTargetSchemaForCdc`,
`HasSchemaChanged`, `TranslateChangesIfSchemaChanged`, and
`TranslateAndPublishRelationForCDC`. It records hashes for the complete source
and extracted body, then compiles those bytes in a PostgreSQL backend module.
It never maintains a copied implementation of translation or cleanup.

Run only against disposable resources, as an unprivileged PostgreSQL user:

```
bash ci/ai-blaise/cdc-ownership-native-run.sh /absolute/source /fresh/result/path
```

The selected `pg_config`, compiler, server and headers must belong to the same
PostgreSQL major. The runner creates its own cluster, trusts only its private
Unix socket, disables TCP listening, executes 24 cases, and stops the server.
A caller must impose resource and runtime bounds and preserve its actual exit.
Do not point the result path at a shared database, an existing cluster, or a
production service. Compile/setup failure is not a product regression result.

The matrix crosses INSERT, UPDATE without an old tuple, UPDATE with an old
tuple, and DELETE with changed/unchanged descriptors and three paths: success,
allocation failure during translation, and an error from the output callback.
For two-tuple UPDATE the allocation fault happens after the first translated
tuple has been installed and before the second is allocated. This checks
partial cleanup. Every case verifies original pointer/view restoration, exact
owned-allocation releases, no foreign frees, relation close, publication-visible
tuple contents/layout, and preservation of the original error SQLSTATE.

PostgreSQL headers, heap tuple allocation/deformation, backend memory contexts,
and PG_TRY/PG_FINALLY error handling are native. Catalog lookup/relation close
and the output-plugin callback are instrumented boundaries. This is not a live
walsender, catalog-lock, network, or end-to-end CDC proof. The actual CDC TAP
suite, including `006_cdc_schema_change_and_move.pl`, remains separately required.

## Product adaptation and patch provenance

The PG17+ ownership branch preserves upstream commit
`211afb11fd5074b12621dd2947260ca570f918c2`. PG16 instead embeds HeapTupleData in
ReorderBufferTupleBuf. Before TRY the wrapper's embedded data is snapshotted;
volatile out-slots retain each actual owning heap_form_tuple allocation before
its data is copied into the embedded view. FINALLY frees only these owners and
restores original wrapper pointers and data, including partial-translation and
callback errors. The wrapper and its original storage are never freed here.

`patches/0013-cdc-translated-tuple-lifetime.patch` is the exact source-file diff
against original operand `cd119257cdcba890d0c887a235ec6dc39d7e0cf9`. It is a
standalone backport representation, not a silently appended member of the
historical `patches/series` with its different frozen applicability targets.
No selected-upstream pin, operand lock, release threshold or historical receipt
is changed by this repair.

The separately tracked CDC TAP fixture compatibility change is represented by
`patches/0014-cdc-output-plugin-admission.patch`, based on the frozen ownership
repair `35c39b7d2be0438119d020dc6000c6da8bc9de6a`. It is likewise standalone,
not appended to historical patch-series targets. See the operator admission
runbook in `docs/ai-blaise/CDC_OUTPUT_PLUGIN_ADMISSION.md`.
