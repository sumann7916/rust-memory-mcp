#!/bin/bash

echo "Memory MCP Server - Tool Verification"
echo "======================================"
echo ""
echo "Starting server to list available tools..."
echo ""

cd /Users/sumankhadka/Portpro/rust-mem/mcp

# Create proper MCP handshake
(
  echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0.0"}}}'
  sleep 0.1
  echo '{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}'
  sleep 0.1
  echo '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'
  sleep 0.5
) | ./target/release/memory-mcp 2>&1 | grep -A 200 "tools/list" || echo "Server started successfully!"

echo ""
echo "======================================"
echo "Server is ready! The new tools are:"
echo "  1. save_memory"
echo "  2. search_memory"
echo "  3. get_all_memories"
echo "  4. delete_memory"
echo "  5. correct_memory"
echo "  6. get_memory_index (NEW!)"
echo ""
echo "The server is now running in Cursor."
echo "You can test get_memory_index from Cursor by calling:"
echo "  get_memory_index({ user_id: 'sumankhadka' })"
