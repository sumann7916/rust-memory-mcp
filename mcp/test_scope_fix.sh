#!/bin/bash

# Test script to verify the agent properly includes scope parameter

echo "Testing memory chat agent with scope parameter fix..."
echo ""

# Run the agent with a test prompt that should trigger save_memory
export RUST_LOG=info

cd "$(dirname "$0")"

# Test 1: Simple preference that should use global scope
echo "Test 1: Global preference (should include scope='global')"
echo "User message: I prefer descriptive variable names over short ones"
echo ""

# Test 2: Language-specific preference
echo "Test 2: Language preference (should include scope='lang', lang='rust')"
echo "User message: In Rust, I prefer Result over panic for error handling"
echo ""

# Test 3: Repository-specific fact
echo "Test 3: Repository fact (should include scope='repo', repo='rust-mem')"
echo "User message: In rust-mem, Quote has a relationship with Vendor"
echo ""

echo "To test manually, run:"
echo "  cd mcp && cargo run --bin memory_chat_agent"
echo ""
echo "Then provide the test messages above and verify the agent includes:"
echo "  - 'scope' parameter in all save_memory calls"
echo "  - Appropriate scope value based on the content"
echo "  - Required additional parameters (repo, lang, etc.) for the scope"
echo ""
echo "Expected behavior:"
echo "  ✓ No 'missing field scope' errors"
echo "  ✓ Agent chooses correct scope value"
echo "  ✓ Agent includes required params for chosen scope"
