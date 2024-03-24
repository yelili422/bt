#!/usr/bin/env bash

/usr/bin/nginx -g "daemon off;";

echo "BT Daemon is starting...";
echo "Downloading path mapping: $DOWNLOADING_PATH_MAPPING";
echo "Archived path: $ARCHIVED_PATH";
echo "Database path: $DATABASE_PATH";

/usr/bin/bt daemon start -m "$DOWNLOADING_PATH_MAPPING" -a "$ARCHIVED_PATH";
