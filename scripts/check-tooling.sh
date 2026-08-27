#!/bin/sh
# Verifies the global (user-scope) Claude Code tooling this template assumes.
# Everything here lives in ~/.claude, not the project — nothing to install
# per-repo, this just confirms it's actually active before you start.
set -u

pass=0
fail=0

ok()   { printf '  \033[0;32m\xe2\x9c\x94\033[0m %s\n' "$1"; pass=$((pass + 1)); }
bad()  { printf '  \033[0;31m\xe2\x9c\x97\033[0m %s\n' "$1"; fail=$((fail + 1)); }
note() { printf '  \033[0;90m-\033[0m %s (optional)\n' "$1"; }

echo "Required:"

if claude mcp list 2>/dev/null | grep -q "headroom:.*Connected"; then
  ok "headroom MCP (context compression) connected"
else
  bad "headroom MCP not connected — check 'claude mcp list'"
fi

if claude mcp list 2>/dev/null | grep -q "tokensave:.*Connected"; then
  ok "tokensave MCP (code graph) connected"
else
  bad "tokensave MCP not connected — check 'claude mcp list'"
fi

if command -v rtk >/dev/null 2>&1; then
  ok "rtk installed ($(rtk --version 2>/dev/null))"
else
  bad "rtk not on PATH — token-saving command hook won't fire"
fi

if grep -q '"ponytail@ponytail": true' ~/.claude/settings.json 2>/dev/null; then
  ok "ponytail plugin enabled"
else
  bad "ponytail plugin not enabled in ~/.claude/settings.json"
fi

if [ -d ~/.claude/skills/graphify ]; then
  ok "graphify skill installed"
else
  bad "graphify skill missing from ~/.claude/skills/"
fi

echo "Optional (stack-dependent):"

if grep -q '"frontend-design@claude-plugins-official": true' ~/.claude/settings.json 2>/dev/null; then
  note "frontend-design plugin enabled"
fi

if grep -q '"rust-analyzer-lsp@claude-plugins-official": true' ~/.claude/settings.json 2>/dev/null; then
  note "rust-analyzer-lsp plugin enabled"
fi

echo
echo "$pass passed, $fail failed"
[ "$fail" -eq 0 ]
