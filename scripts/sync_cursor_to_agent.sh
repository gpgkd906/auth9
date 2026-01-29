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

# Convert rules to skills
RULES_DIR="./.cursor/rules"
echo "Converting rules from '$RULES_DIR' to '$DEST_DIR'..."

if [ -d "$RULES_DIR" ]; then
  for rule_file in "$RULES_DIR"/*.mdc; do
    if [ -f "$rule_file" ]; then
      filename=$(basename -- "$rule_file")
      skill_name="${filename%.*}"
      skill_dir="$DEST_DIR/$skill_name"
      
      echo "Processing rule: $skill_name"
      
      # Create skill directory
      mkdir -p "$skill_dir"
      
      # Read the file line by line to inject the name into the frontmatter
      awk -v name="$skill_name" '
      BEGIN { in_frontmatter = 0; name_injected = 0; }
      /^---$/ {
          print $0;
          if (in_frontmatter == 0) {
              in_frontmatter = 1;
              # Inject name immediately after the first ---
              print "name: " name;
              name_injected = 1;
          } else {
              in_frontmatter = 0;
          }
          next;
      }
      {
          # skip existing name field if present to avoid duplication (simple check)
          if (in_frontmatter == 1 && $1 == "name:") {
              next; 
          }
          print $0;
      }
      ' "$rule_file" > "$skill_dir/SKILL.md"
      
    fi
  done
else
  echo "Rules directory '$RULES_DIR' does not exist. Skipping rule conversion."
fi

echo "Synchronization complete."
