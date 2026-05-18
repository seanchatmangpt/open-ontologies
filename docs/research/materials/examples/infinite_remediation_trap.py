#!/usr/bin/env python3
"""
Case Study 3: The Infinite Remediation Trap (Exercise 3.1)
SAB 900: The Saboteur Labs

This mock script simulates a poorly configured autonomous agent (e.g., LangChain/AutoGPT) 
that blindly trusts the autonomic developer experience (DX) `hint` fields returned by 
the open-ontologies MCP server.

If the database is locked, this script will starve the server by looping infinitely.
Students must implement a quarantine circuit in `src/health_guardian.rs` to break this loop.
"""

import time
import json
import subprocess

def call_mcp_tool(tool_name: str, args: list) -> dict:
    """Simulates calling the open-ontologies MCP server."""
    cmd = ["open-ontologies", "server", tool_name] + args
    result = subprocess.run(cmd, capture_output=True, text=True)
    try:
        return json.loads(result.stdout)
    except Exception:
        return {"ok": False, "error": "subprocess_failed", "hint": None}

def run_autonomous_agent():
    print("[*] Starting Autonomous Agent...")
    current_tool = "onto_conformance_check"
    current_args = ["--scope", "civic_dispatch"]
    
    loop_count = 0
    while True:
        loop_count += 1
        print(f"[-] Loop {loop_count}: Executing {current_tool}...")
        
        response = call_mcp_tool(current_tool, current_args)
        
        if response.get("ok"):
            print("[+] Success!")
            break
            
        error_class = response.get("error_class")
        hint = response.get("hint")
        
        print(f"[!] Defect Hit: {error_class}")
        
        # The Trap: The agent blindly executes the remediation hint.
        if hint and "onto_declare_workflow" in hint:
            print(f"[*] Following DX hint. Switching tool to 'onto_declare_workflow'.")
            current_tool = "onto_declare_workflow"
            current_args = ["--workflow", "civic_dispatch"]
            # Without the HealthGuardian backoff, this loops forever if the DB is locked.
        else:
            print("[-] No actionable hint. Agent exiting.")
            break
            
        time.sleep(0.1) # Aggressive polling

if __name__ == "__main__":
    run_autonomous_agent()