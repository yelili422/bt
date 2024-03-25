#!/usr/bin/env bash


echo "Nginx is running...";

nginx;

echo "BT web api is running...";

nohup sh -c web_api > web_api.log 2>&1 &

echo "BT Daemon is starting...";
echo "Downloading path mapping: $DOWNLOADING_PATH_MAPPING";
echo "Archived path: $ARCHIVED_PATH";
echo "Database path: $DATABASE_PATH";

cmd daemon start -m "$DOWNLOADING_PATH_MAPPING" -a "$ARCHIVED_PATH";
