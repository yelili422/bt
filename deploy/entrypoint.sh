#!/usr/bin/env bash

echo "Nginx is running..."

nginx

echo "BT web api is running..."

nohup sh -c web_api >web_api.log 2>&1 &

if [ -z "$RETRIEVE_INTERVAL" ]; then
    RETRIEVE_INTERVAL=600
fi

echo "BT Daemon is starting..."
echo "Downloading path mapping: $DOWNLOADING_PATH_MAPPING"
echo "Archived path: $ARCHIVED_PATH"
echo "Database path: $DATABASE_PATH"
echo "Retrieve interval: $RETRIEVE_INTERVAL"

cmd daemon start \
    -m "$DOWNLOADING_PATH_MAPPING" \
    -a "$ARCHIVED_PATH" \
    -i "$RETRIEVE_INTERVAL"
