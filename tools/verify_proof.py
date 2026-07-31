#!/usr/bin/env python3
import os
import hashlib
import sys

def get_file_sha256(filepath):
    h = hashlib.sha256()
    with open(filepath, 'rb') as f:
        while True:
            chunk = f.read(65536)
            if not chunk:
                break
            h.update(chunk)
    return h.hexdigest()

def verify_proof(proof_path, artifacts_dir):
    if not os.path.exists(proof_path):
        print(f"Error: {proof_path} does not exist.")
        return False

    with open(proof_path, 'r', encoding='utf-8') as f:
        content = f.read()

    lines = content.splitlines()
    
    # Parse final_proof.txt values
    proof_hashes = {}
    proof_report_hash = None
    
    for line in lines:
        line_strip = line.strip()
        if line_strip.startswith("- expected OCEL manifest:"):
            proof_hashes["EXPECTED_OCEL_MANIFEST.v26.5.21.json"] = line_strip.split(":")[-1].strip()
        elif line_strip.startswith("- observed OCEL manifest:"):
            proof_hashes["OBSERVED_OCEL_MANIFEST.v26.5.21.json"] = line_strip.split(":")[-1].strip()
        elif line_strip.startswith("- alignment manifest:"):
            proof_hashes["ALIGNMENT_MANIFEST.v26.5.21.json"] = line_strip.split(":")[-1].strip()
        elif line_strip.startswith("- receipt bundle:"):
            proof_hashes["AUTORECEIPT_BUNDLE.v26.5.21.json"] = line_strip.split(":")[-1].strip()
        elif line_strip.startswith("- final proof report:"):
            proof_report_hash = line_strip.split(":")[-1].strip()

    # 1. Verify manifest/bundle hashes against files on disk
    print("--- Verifying Manifest/Bundle Hashes ---")
    all_matched = True
    for filename, expected_hash in proof_hashes.items():
        filepath = os.path.join(artifacts_dir, filename)
        if not os.path.exists(filepath):
            print(f"FAIL: File {filename} not found at {filepath}")
            all_matched = False
            continue
        actual_hash = get_file_sha256(filepath)
        if actual_hash == expected_hash:
            print(f"PASS: {filename} hash matches ({actual_hash[:16]}...)")
        else:
            print(f"FAIL: {filename} hash mismatch!")
            print(f"      Expected: {expected_hash}")
            print(f"      Actual:   {actual_hash}")
            all_matched = False

    # 2. Verify self-referential report hash
    print("\n--- Verifying Self-Referential Proof Report Hash ---")
    filtered_lines = [l for l in lines if not l.strip().startswith("- final proof report:")]
    # Join back with newlines and add trailing newline
    filtered_content = "\n".join(filtered_lines) + "\n"
        
    computed_report_hash = hashlib.sha256(filtered_content.encode('utf-8')).hexdigest()
    
    if proof_report_hash == computed_report_hash:
        print(f"PASS: final proof report hash matches ({computed_report_hash})")
    else:
        print("FAIL: final proof report hash mismatch!")
        print(f"      In file:  {proof_report_hash}")
        print(f"      Computed: {computed_report_hash}")
        all_matched = False

    # Also print full file hash for documentation
    full_file_hash = hashlib.sha256(content.encode('utf-8')).hexdigest()
    print(f"\nInfo: Full final_proof.txt hash is: {full_file_hash}")

    return all_matched

if __name__ == "__main__":
    proof_file = "final_proof.txt"
    artifacts_path = "artifacts/autoreceipt"
    
    if len(sys.argv) > 1:
        proof_file = sys.argv[1]
    if len(sys.argv) > 2:
        artifacts_path = sys.argv[2]

    success = verify_proof(proof_file, artifacts_path)
    sys.exit(0 if success else 1)
