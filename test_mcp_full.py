#!/usr/bin/env python3
"""
Full MCP test script - tests initialization, tool listing, and memory operations
"""
import subprocess
import json
import sys
import os

def send_receive(proc, request):
    """Send a JSON-RPC request and receive the response"""
    request_str = json.dumps(request) + "\n"
    proc.stdin.write(request_str)
    proc.stdin.flush()
    
    response_line = proc.stdout.readline()
    if response_line:
        return json.loads(response_line)
    return None

def main():
    os.chdir("/Users/sumankhadka/Portpro/rust-mem/mcp")
    
    env = os.environ.copy()
    env["LLM_MODEL"] = "gemma3:4b"
    
    print("Starting MCP server...")
    proc = subprocess.Popen(
        ["./target/release/memory-mcp"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=env
    )
    
    try:
        # 1. Initialize
        print("\n1. Sending initialize request...")
        init_request = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "1.0"}
            }
        }
        response = send_receive(proc, init_request)
        print(f"   Response: {json.dumps(response, indent=2)}")
        
        # 2. Send initialized notification
        print("\n2. Sending initialized notification...")
        init_notification = {
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }
        proc.stdin.write(json.dumps(init_notification) + "\n")
        proc.stdin.flush()
        print("   Sent.")
        
        # 3. List tools
        print("\n3. Listing tools...")
        list_tools = {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }
        response = send_receive(proc, list_tools)
        print(f"   Tools available:")
        if response and "result" in response:
            for tool in response["result"].get("tools", []):
                print(f"   - {tool['name']}: {tool.get('description', '')[:60]}...")
        
        # 4. List prompts
        print("\n4. Listing prompts...")
        list_prompts = {
            "jsonrpc": "2.0",
            "id": 3,
            "method": "prompts/list",
            "params": {}
        }
        response = send_receive(proc, list_prompts)
        print(f"   Prompts available:")
        if response and "result" in response:
            for prompt in response["result"].get("prompts", []):
                print(f"   - {prompt['name']}: {prompt.get('description', '')}")
        
        # 5. Save a memory
        print("\n5. Saving a memory (this will call Ollama LLM)...")
        save_memory = {
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "save_memory",
                "arguments": {
                    "content": "I prefer using early returns instead of nested if-else statements in my code",
                    "user_id": "test-user",
                    "repo": "rust-mem",
                    "lang": "rust"
                }
            }
        }
        response = send_receive(proc, save_memory)
        print(f"   Response: {json.dumps(response, indent=2)[:500]}...")
        
        # 6. Search memory
        print("\n6. Searching memories...")
        search_memory = {
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "search_memory",
                "arguments": {
                    "query": "coding style preferences",
                    "user_id": "test-user",
                    "repo": "rust-mem",
                    "limit": 5
                }
            }
        }
        response = send_receive(proc, search_memory)
        print(f"   Response: {json.dumps(response, indent=2)[:500]}...")
        
        # 7. Get all memories
        print("\n7. Getting all memories...")
        get_all = {
            "jsonrpc": "2.0",
            "id": 6,
            "method": "tools/call",
            "params": {
                "name": "get_all_memories",
                "arguments": {
                    "user_id": "test-user"
                }
            }
        }
        response = send_receive(proc, get_all)
        print(f"   Response: {json.dumps(response, indent=2)[:500]}...")
        
        print("\n✅ All tests completed!")
        
    except Exception as e:
        print(f"\n❌ Error: {e}")
        stderr = proc.stderr.read()
        if stderr:
            print(f"Server stderr: {stderr}")
    finally:
        proc.terminate()
        proc.wait()

if __name__ == "__main__":
    main()
