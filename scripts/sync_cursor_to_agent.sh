#!/bin/bash

# Define source and destination directories
SOURCE_DIR="./.cursor/skills"
DEST_DIR="./.agent/skills"

# Check if source directory exists
if [ ! -d "$SOURCE_DIR" ]; then
  echo "Error: Source directory '$SOURCE_DIR' does not exist."
  exit 1
fi

# Create destination directory if it doesn't exist
if [ ! -d "$DEST_DIR" ]; then
  echo "Creating destination directory '$DEST_DIR'..."
  mkdir -p "$DEST_DIR"
else
  echo "Destination directory '$DEST_DIR' already exists."
fi

# Sync files using rsync
# -a: archive mode (preserves permissions, times, etc.)
# -v: verbose output
# --delete: delete extraneous files from dest dirs
echo "Syncing skills from '$SOURCE_DIR' to '$DEST_DIR'..."
rsync -av --delete "$SOURCE_DIR/" "$DEST_DIR/"

echo "Synchronization complete."
