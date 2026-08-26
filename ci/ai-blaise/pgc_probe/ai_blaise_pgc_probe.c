/* FEATURE: PGC1 PGC2 */
#include "postgres.h"

#include "fmgr.h"

#include "access/commit_ts.h"
#include "access/transam.h"
#include "access/xact.h"
#include "access/xlog.h"
#include "utils/builtins.h"
#include "utils/timestamp.h"

PG_MODULE_MAGIC;

PG_FUNCTION_INFO_V1(ai_blaise_pgc_logical_clock_roundtrip);
PG_FUNCTION_INFO_V1(ai_blaise_pgc_subtrans_override);

Datum
ai_blaise_pgc_logical_clock_roundtrip(PG_FUNCTION_ARGS)
{
	TimestampTz requested = PG_GETARG_TIMESTAMPTZ(0);

	XLogSetLastTransactionStopTimestamp(requested);
	remoteTransactionStopTimestamp = requested;

	PG_RETURN_TIMESTAMPTZ(XLogGetLastTransactionStopTimestamp());
}


Datum
ai_blaise_pgc_subtrans_override(PG_FUNCTION_ARGS)
{
	TimestampTz requested = PG_GETARG_TIMESTAMPTZ(0);
	int32 nodeid_arg = PG_GETARG_INT32(1);
	RepOriginId nodeid;
	TransactionId xid;
	char xid_text[32];

	if (nodeid_arg < 0 || nodeid_arg > 65535)
	{
		ereport(ERROR,
				(errcode(ERRCODE_NUMERIC_VALUE_OUT_OF_RANGE),
				 errmsg("nodeid must fit in uint16")));
	}

	nodeid = (RepOriginId) nodeid_arg;
	xid = GetCurrentTransactionId();

	SubTransactionIdSetCommitTsData(xid, requested, nodeid);

	snprintf(xid_text, sizeof(xid_text), "%u", xid);
	PG_RETURN_TEXT_P(cstring_to_text(xid_text));
}
