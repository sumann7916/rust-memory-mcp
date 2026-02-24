#!/bin/bash

# Test script for MCP server
# This sends JSON-RPC messages to the MCP server via stdin/stdout

cd /Users/sumankhadka/Portpro/rust-mem/mcp

# Set the LLM model to one that exists
export LLM_MODEL="gemma3:4b"

# Build and run with a simple test
echo "Building MCP server..."
cargo build --release 2>&1

echo ""
echo "Testing MCP server initialization..."

# Create a temp file for the test
cat > /tmp/mcp_test_input.json << 'EOF'
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}
EOF

# Run the server with the test input (with timeout)
echo "Sending initialize request..."
timeout 10s bash -c 'cat /tmp/mcp_test_input.json | ./target/release/memory-mcp' 2>&1 | head -20

echo ""
echo "Test complete."
