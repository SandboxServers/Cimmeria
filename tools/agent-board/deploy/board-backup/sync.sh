#!/bin/sh
# Copy Discourse backups off-host to Azure Blob (cimmeriaboardbackupw2/discourse-backups).
# copy, not sync: local pruning never deletes remote copies; Azure lifecycle expires them at 30 days.
set -eu
exec /usr/bin/docker run --rm \
  --env-file /mnt/nvme/board-backup/azure.env \
  -e RCLONE_CONFIG_AZ_TYPE=azureblob \
  -e RCLONE_CONFIG_AZ_ACCOUNT=cimmeriaboardbackupw2 \
  -e RCLONE_CONFIG_AZ_ENV_AUTH=true \
  -v /mnt/nvme/discourse/shared/standalone/backups/default:/backups:ro \
  --cap-drop ALL --security-opt no-new-privileges:true \
  rclone/rclone:1 copy /backups az:discourse-backups --include "*.tar.gz" --immutable --log-level INFO
