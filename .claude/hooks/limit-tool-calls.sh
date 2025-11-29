#!/usr/bin/env bash
# Hook: Limit tool calls for orchestrator child agents
# Prevents infinite loops by enforcing max 50 tool calls per session

set -euo pipefail

# Read hook context from stdin
CONTEXT=$(cat)

# Extract session ID and project directory
SESSION_ID=$(echo "$CONTEXT" | jq -r '.session_id // "unknown"')
PROJECT_DIR=$(echo "$CONTEXT" | jq -r '.cwd // "."')

# State file to track tool calls per session
STATE_DIR="${PROJECT_DIR}/.claude/hook-state"
mkdir -p "$STATE_DIR"
STATE_FILE="${STATE_DIR}/tool-calls-${SESSION_ID}.count"

# Clean up old state files (older than 10 minutes)
find "$STATE_DIR" -name "tool-calls-*.count" -type f -mmin +10 -delete 2>/dev/null || true

# Initialize counter if file doesn't exist
if [[ ! -f "$STATE_FILE" ]]; then
    echo "0" > "$STATE_FILE"
fi

# Read current count
CURRENT_COUNT=$(cat "$STATE_FILE")

# Increment counter
NEW_COUNT=$((CURRENT_COUNT + 1))
echo "$NEW_COUNT" > "$STATE_FILE"

# Check if limit exceeded (50 tool calls)
MAX_CALLS=50
if [[ $NEW_COUNT -gt $MAX_CALLS ]]; then
    # Deny the tool call and provide feedback
    cat <<EOF
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "deny",
    "permissionDecisionReason": "LOOP DETECTED: Tool call limit exceeded ($NEW_COUNT/$MAX_CALLS). You have made $NEW_CALLS tool calls without completing your task. This indicates an infinite loop. Update the spec with blockers and EXIT immediately. Do NOT retry."
  }
}
EOF
    exit 0
fi

# Allow the tool call
cat <<EOF
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "allow",
    "permissionDecisionReason": "Tool call $NEW_COUNT/$MAX_CALLS allowed"
  }
}
EOF
exit 0
