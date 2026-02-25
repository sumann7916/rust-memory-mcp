#!/usr/bin/env python3
"""
Test script for the Memory MCP server with get_memory_index tool.
Tests the new get_memory_index functionality.
"""

import json
import subprocess
import sys

def send_mcp_request(method, params):
    """Send a JSON-RPC request to the MCP server."""
    request = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params
    }
    
    # Start the MCP server
    process = subprocess.Popen(
        ["./target/release/memory-mcp"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        cwd="/Users/sumankhadka/Portpro/rust-mem/mcp"
    )
    
    # Send request
    request_str = json.dumps(request) + "\n"
    stdout, stderr = process.communicate(input=request_str, timeout=5)
    
    if stderr:
        print(f"Server stderr: {stderr}", file=sys.stderr)
    
    # Parse response
    try:
        response = json.loads(stdout)
        return response
    except json.JSONDecodeError as e:
        print(f"Failed to parse response: {stdout}", file=sys.stderr)
        raise

def test_get_memory_index():
    """Test the get_memory_index tool."""
    print("Testing get_memory_index tool...")
    print("-" * 50)
    
    # First, let's initialize the server
    init_response = send_mcp_request("initialize", {
        "protocolVersion": "2024-11-05",
        "capabilities": {},
        "clientInfo": {
            "name": "test-client",
            "version": "1.0.0"
        }
    })
    
    print("Initialize response:")
    print(json.dumps(init_response, indent=2))
    print("\n" + "-" * 50 + "\n")
    
    # Now test get_memory_index
    index_params = {
        "name": "get_memory_index",
        "arguments": {
            "user_id": "sumankhadka"
        }
    }
    
    try:
        index_response = send_mcp_request("tools/call", index_params)
        
        print("get_memory_index response:")
        print(json.dumps(index_response, indent=2))
        
        # Parse the result
        if "result" in index_response and "content" in index_response["result"]:
            content = index_response["result"]["content"]
            if content and len(content) > 0:
                # The content is a list with text items
                text_content = content[0].get("text", "")
                if text_content:
                    memory_index = json.loads(text_content)
                    print("\n" + "-" * 50)
                    print("Memory Index Summary:")
                    print("-" * 50)
                    print(f"Total memories: {memory_index.get('total', 0)}")
                    print(f"Topics ({len(memory_index.get('topics', []))}): {', '.join(memory_index.get('topics', [])[:10])}")
                    print(f"Languages: {', '.join(memory_index.get('langs', []))}")
                    print(f"Repositories: {', '.join(memory_index.get('repos', []))}")
                    print(f"\nScope distribution:")
                    for scope, count in memory_index.get('scopes', {}).items():
                        print(f"  {scope}: {count}")
                    print(f"\nRecent memories:")
                    for i, recent in enumerate(memory_index.get('recent', []), 1):
                        print(f"  {i}. {recent}")
        
        return True
        
    except Exception as e:
        print(f"Error testing get_memory_index: {e}", file=sys.stderr)
        import traceback
        traceback.print_exc()
        return False

if __name__ == "__main__":
    print("Memory MCP Server - get_memory_index Test")
    print("=" * 50)
    print()
    
    success = test_get_memory_index()
    
    print("\n" + "=" * 50)
    if success:
        print("✅ Test completed successfully!")
        sys.exit(0)
    else:
        print("❌ Test failed!")
        sys.exit(1)
