/*-------------------------------------------------------------------------
 *
 * shared_library_init.h
 *	  Functionality related to the initialization of the Citus extension.
 *
 * Copyright (c) Citus Data, Inc.
 *
 *-------------------------------------------------------------------------
 */

#ifndef SHARED_LIBRARY_INIT_H
#define SHARED_LIBRARY_INIT_H

#include "columnar/columnar.h"

#define GUC_STANDARD 0
#define MAX_SHARD_COUNT 64000
#define MAX_SHARD_REPLICATION_FACTOR 100

extern PGDLLEXPORT ColumnarSupportsIndexAM_type extern_ColumnarSupportsIndexAM;
extern PGDLLEXPORT CompressionTypeStr_type extern_CompressionTypeStr;
extern PGDLLEXPORT IsColumnarTableAmTable_type extern_IsColumnarTableAmTable;
extern PGDLLEXPORT ReadColumnarOptions_type extern_ReadColumnarOptions;

typedef enum CitusCohabitExtensionKind
{
	CITUS_COHABIT_EXTENSION_UNSUPPORTED = 0,
	CITUS_COHABIT_EXTENSION_TRUSTED_HOOK,
	CITUS_COHABIT_EXTENSION_CLOCK,
	CITUS_COHABIT_EXTENSION_PARTITION_MANAGER,
} CitusCohabitExtensionKind;

extern void StartupCitusBackend(void);
extern const char * GetClientMinMessageLevelNameForValue(int minMessageLevel);
extern CitusCohabitExtensionKind ClassifyCitusCohabitExtension(const char *extensionName);
extern bool CitusCohabitExtensionConfigured(const char *extensionName);

#endif /* SHARED_LIBRARY_INIT_H */
