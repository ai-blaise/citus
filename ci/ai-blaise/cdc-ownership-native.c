/* FEATURE: C1 -- native ownership probe; not a substitute for live CDC TAP. */
#include "postgres.h"

#include "fmgr.h"

#include "access/htup_details.h"
#include "catalog/pg_type_d.h"
#include "replication/logical.h"
#include "replication/output_plugin.h"
#include "replication/reorderbuffer.h"
#include "utils/builtins.h"
#include "utils/memutils.h"
#include "utils/rel.h"

#define PG_VERSION_17 170000
PG_MODULE_MAGIC;
PG_FUNCTION_INFO_V1(cdc_ownership_native);

static Relation witness_relation;
static TupleDesc witness_desc;
static HeapTuple owned[2];
static bool released[2];
static int owned_count, foreign_frees, closes, callbacks, fault, fail_allocation;
static bool changed, callback_contents_ok;

static Relation
witness_lookup(Oid ignored)
{
	return witness_relation;
}


static void
witness_close(Relation ignored)
{
	closes++;
}


static void
witness_free(HeapTuple tuple)
{
	int i;
	for (i = 0; i < owned_count; i++)
	{
		if (owned[i] == tuple && !released[i])
		{
			released[i] = true;
			heap_freetuple(tuple);
			return;
		}
	}
	foreign_frees++;
}


static HeapTuple
witness_form(TupleDesc descriptor, Datum *values, bool *nulls)
{
	HeapTuple tuple;
	if (fault == 1 && owned_count + 1 == fail_allocation)
	{
		ereport(ERROR, (errcode(ERRCODE_DATA_EXCEPTION), errmsg(
							"witness translation allocation error")));
	}
	tuple = heap_form_tuple(descriptor, values, nulls);
	owned[owned_count++] = tuple;
	return tuple;
}


static void
witness_callback(LogicalDecodingContext *ctx, ReorderBufferTXN *txn,
				 Relation relation, ReorderBufferChange *change)
{
	HeapTuple tuples[2];
	int i;
	callbacks++;
#if PG_VERSION_NUM >= PG_VERSION_17
	tuples[0] = change->data.tp.newtuple;
	tuples[1] = change->data.tp.oldtuple;
#else
	tuples[0] = change->data.tp.newtuple ? &change->data.tp.newtuple->tuple : NULL;
	tuples[1] = change->data.tp.oldtuple ? &change->data.tp.oldtuple->tuple : NULL;
#endif
	for (i = 0; i < 2; i++)
	{
		if (tuples[i])
		{
			bool isnull;
			Datum value = heap_getattr(tuples[i], 1, witness_desc, &isnull);
			if (isnull || DatumGetInt32(value) != 42 ||
				HeapTupleHeaderGetNatts(tuples[i]->t_data) != (changed ? 1 : 2))
			{
				callback_contents_ok = false;
			}
		}
	}
	if (fault == 2)
	{
		ereport(ERROR, (errcode(ERRCODE_DATA_EXCEPTION), errmsg("witness callback error"))
				);
	}
}


static LogicalDecodeChangeCB ouputPluginChangeCB = witness_callback;
#define RelationIdGetRelation witness_lookup
#define RelationClose witness_close
#define heap_freetuple witness_free
#define heap_form_tuple witness_form
#include "cdc-publish-extracted.inc"
#undef RelationIdGetRelation
#undef RelationClose
#undef heap_freetuple
#undef heap_form_tuple

Datum
cdc_ownership_native(PG_FUNCTION_ARGS)
{
	char *action = text_to_cstring(PG_GETARG_TEXT_PP(0));
	ReorderBufferChange *change = palloc0(sizeof(ReorderBufferChange));
	RelationData relation = { 0 }, target_relation = { 0 };
	TupleDesc target_desc;
	HeapTuple original[2] = { NULL, NULL };
	Datum values[2] = { Int32GetDatum(42), Int32GetDatum(7) };
	bool nulls[2] = { false, false };
	bool has_new, has_old, restored, caught_expected;
	volatile int caught = 0;
	int i, release_count = 0;
	MemoryContext caller_context = CurrentMemoryContext;
#if PG_VERSION_NUM < PG_VERSION_17
	ReorderBufferTupleBuf *buffers = palloc0(2 * sizeof(ReorderBufferTupleBuf));
	HeapTupleData saved[2] = { 0 };
#endif

	if (strcmp(action, "insert") == 0)
	{
		change->action = REORDER_BUFFER_CHANGE_INSERT;
	}
	else if (strcmp(action, "delete") == 0)
	{
		change->action = REORDER_BUFFER_CHANGE_DELETE;
	}
	else if (strcmp(action, "update-key") == 0 || strcmp(action, "update-full") == 0)
	{
		change->action = REORDER_BUFFER_CHANGE_UPDATE;
	}
	else
	{
		ereport(ERROR, (errmsg("unknown witness action")));
	}
	has_new = change->action != REORDER_BUFFER_CHANGE_DELETE;
	has_old = change->action == REORDER_BUFFER_CHANGE_DELETE || strcmp(action,
																	   "update-full") == 0
	;
	changed = PG_GETARG_BOOL(1);
	fault = PG_GETARG_INT32(2);
	if (fault < 0 || fault > 2)
	{
		ereport(ERROR, (errmsg("unknown witness fault")));
	}
	memset(owned, 0, sizeof(owned));
	memset(released, 0, sizeof(released));
	owned_count = foreign_frees = closes = callbacks = 0;
	fail_allocation = has_new && has_old ? 2 : 1;
	callback_contents_ok = true;
	witness_desc = CreateTemplateTupleDesc(2);
	TupleDescInitEntry(witness_desc, 1, "value", INT4OID, -1, 0);
	TupleDescInitEntry(witness_desc, 2, "removed", INT4OID, -1, 0);
	target_desc = CreateTemplateTupleDesc(changed ? 1 : 2);
	TupleDescInitEntry(target_desc, 1, "value", INT4OID, -1, 0);
	if (!changed)
	{
		TupleDescInitEntry(target_desc, 2, "removed", INT4OID, -1, 0);
	}
	relation.rd_att = witness_desc;
	target_relation.rd_att = target_desc;
	witness_relation = &target_relation;
	if (has_new)
	{
		original[0] = heap_form_tuple(witness_desc, values, nulls);
	}
	if (has_old)
	{
		original[1] = heap_form_tuple(witness_desc, values, nulls);
	}
#if PG_VERSION_NUM >= PG_VERSION_17
	change->data.tp.newtuple = original[0];
	change->data.tp.oldtuple = original[1];
#else
	for (i = 0; i < 2; i++)
	{
		if (original[i])
		{
			saved[i] = *original[i];
			buffers[i].tuple = saved[i];
		}
	}
	change->data.tp.newtuple = has_new ? &buffers[0] : NULL;
	change->data.tp.oldtuple = has_old ? &buffers[1] : NULL;
#endif
	PG_TRY();
	{
		TranslateAndPublishRelationForCDC(NULL, NULL, &relation, change, 1, 1);
	}
	PG_CATCH();
	{
		ErrorData *error;
		MemoryContextSwitchTo(caller_context);
		error = CopyErrorData();
		caught = error->sqlerrcode;
		FreeErrorData(error);
		FlushErrorState();
	}
	PG_END_TRY();
#if PG_VERSION_NUM >= PG_VERSION_17
	restored = change->data.tp.newtuple == original[0] && change->data.tp.oldtuple ==
			   original[1];
#else
	restored = change->data.tp.newtuple == (has_new ? &buffers[0] : NULL) &&
			   change->data.tp.oldtuple == (has_old ? &buffers[1] : NULL) &&
			   (!has_new || memcmp(&buffers[0].tuple, &saved[0], sizeof(HeapTupleData)) ==
				0) &&
			   (!has_old || memcmp(&buffers[1].tuple, &saved[1], sizeof(HeapTupleData)) ==
				0);
#endif
	caught_expected = caught == (((fault == 1 && changed) || fault == 2) ?
								 ERRCODE_DATA_EXCEPTION : 0);

	/* Capture verdict before test-only recovery. Never free a foreign original. */
	for (i = 0; i < owned_count; i++)
	{
		if (released[i])
		{
			release_count++;
		}
		else
		{
			heap_freetuple(owned[i]);
		}
	}
	for (i = 0; i < 2; i++)
	{
		if (original[i])
		{
			heap_freetuple(original[i]);
		}
	}
	FreeTupleDesc(witness_desc);
	FreeTupleDesc(target_desc);
	pfree(change);
#if PG_VERSION_NUM < PG_VERSION_17
	pfree(buffers);
#endif
	if (!restored || release_count != owned_count || foreign_frees || closes != 1 ||
		!caught_expected || !callback_contents_ok || callbacks != ((fault == 1 && changed)
	? 0 : 1))
	{
		ereport(ERROR, (errmsg(
							"CDC ownership failed: restored=%d allocated=%d released=%d foreign_frees=%d closes=%d error_preserved=%d callback_contents=%d callbacks=%d",
							restored, owned_count, release_count, foreign_frees, closes
																						 ,
							caught_expected, callback_contents_ok, callbacks)));
	}
	PG_RETURN_TEXT_P(cstring_to_text(psprintf(
										 "pass action=%s changed=%d fault=%d allocated=%d released=%d restored=1 error_preserved=1",
										 action, changed, fault, owned_count,
										 release_count)));
}
