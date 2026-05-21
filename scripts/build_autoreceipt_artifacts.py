import json
import os
import hashlib
import subprocess
from datetime import datetime

# Capture git state BEFORE writing new artifacts
commit = subprocess.check_output(['git', 'rev-parse', 'HEAD']).decode().strip()
tree = subprocess.check_output(['git', 'status', '--short']).decode().strip()

VERSION = "26.5.21"
BASE_DIR = "artifacts/autoreceipt"

with open(f"{BASE_DIR}/JTBD_EXECUTION_BINDING_REGISTRY.v{VERSION}.json", "r") as f:
    registry = json.load(f)

# Count metrics
total_jtbds = len(registry)
ready_jtbds = 0

bundle = {"jtbds": []}

for r in registry:
    jid = r["jtbd_id"]
    align_path = f"{BASE_DIR}/alignment/{jid}.alignment.receipt.json"
    obs_path = f"{BASE_DIR}/observed-ocel/{jid}.observed.ocel.json"
    
    align_data = {}
    if os.path.exists(align_path):
        with open(align_path, "r") as f:
            align_data = json.load(f)
    
    obs_valid = False
    if os.path.exists(obs_path):
        with open(obs_path, "r") as f:
            obs_data = json.load(f)
            has_plan = bool(obs_data.get("actuation_plan_id"))
            has_exp_hash = bool(obs_data.get("expected_ocel_hash"))
            has_real_boundary = bool(obs_data.get("real_boundary_evidence"))
            has_obj_refs = "ocel:object-types" in obs_data.get("ocel:global-log", {})
            has_actor_basis = bool(obs_data.get("actor_basis8"))
            is_smoke = obs_data.get("execution_mode") == "synthetic_or_command_smoke"
            is_valid_flag = obs_data.get("valid_for_autoreceipt_closure", False)
            
            if has_plan and has_exp_hash and has_real_boundary and has_actor_basis and has_obj_refs and is_valid_flag and not is_smoke:
                obs_valid = True

    is_simulated = "simulated" in align_data.get("reason", "").lower()
    
    if obs_valid and align_data.get("alignment_status") == "OcelAlignmentPassed" and not is_simulated:
        state = "AutoReceiptReady"
    else:
        state = "EvidenceIncomplete"

    if state == "AutoReceiptReady": ready_jtbds += 1
    
    bundle["jtbds"].append({
        "id": jid,
        "state": state,
        "alignment_receipt": align_path
    })

with open(f"{BASE_DIR}/AUTORECEIPT_BUNDLE.v{VERSION}.json", "w") as f:
    json.dump(bundle, f, indent=2)

matrix_md = "# AutoReceipt Matrix\n\n| JTBD | State | Alignment |\n|---|---|---|\n"
for j in bundle["jtbds"]:
    align_path = j["alignment_receipt"]
    align_status = "Missing"
    if os.path.exists(align_path):
        with open(align_path, "r") as f:
            align_status = json.load(f).get("alignment_status", "Missing")
    matrix_md += f"| {j['id']} | {j['state']} | {align_status} |\n"
    
with open(f"{BASE_DIR}/AUTORECEIPT_MATRIX.v{VERSION}.md", "w") as f:
    f.write(matrix_md)

cert = {
    "status": "EvidenceIncomplete" if ready_jtbds < total_jtbds else "AutoReceiptReady",
    "timestamp": datetime.utcnow().isoformat() + "Z",
    "total_jtbds": total_jtbds,
    "ready_jtbds": ready_jtbds,
    "blocked_jtbds": total_jtbds - ready_jtbds
}
with open(f"{BASE_DIR}/AUTORECEIPT_CLOSURE_CERTIFICATE.v{VERSION}.json", "w") as f:
    json.dump(cert, f, indent=2)

def sha256(path):
    if not os.path.exists(path): return "none"
    return hashlib.sha256(open(path, 'rb').read()).hexdigest()

idx_hash = sha256(f"{BASE_DIR}/persona-jtbd-index.v{VERSION}.json")
ttl_hash = sha256(f"{BASE_DIR}/persona-jtbd-public-alignment.ttl")
shacl_hash = sha256(f"{BASE_DIR}/public-validation-report.v{VERSION}.json")
manifest_hash = sha256(f"{BASE_DIR}/EXPECTED_OCEL_MANIFEST.v{VERSION}.json")
bundle_hash = sha256(f"{BASE_DIR}/AUTORECEIPT_BUNDLE.v{VERSION}.json")
matrix_hash = sha256(f"{BASE_DIR}/AUTORECEIPT_MATRIX.v{VERSION}.md")
cert_hash = sha256(f"{BASE_DIR}/AUTORECEIPT_CLOSURE_CERTIFICATE.v{VERSION}.json")

# Calculate observed/alignment counts matching actual valid state
obs_count = 0
align_count = 0
for j in bundle["jtbds"]:
    if j["state"] == "AutoReceiptReady":
        obs_count += 1
        align_count += 1

final_state = "AutoReceiptClosed" if ready_jtbds == total_jtbds and not tree else "EvidenceIncomplete"
if tree and ready_jtbds == total_jtbds: final_state = "DirtyTreeUnclassified"

print(f"""State:
{final_state}

Version:
{VERSION}

Commit:
{commit}
Tree:
{tree}
Counts:
- personas: 8
- JTBDs: {total_jtbds}
- expected OCEL files: {total_jtbds}
- observed OCEL files (real): {obs_count}
- alignment receipts (passed): {align_count}
- AutoReceipt-ready JTBDs: {ready_jtbds}
- blocked JTBDs: {total_jtbds - ready_jtbds}

Artifacts:
- persona-jtbd-index: {idx_hash}
- public alignment TTL: {ttl_hash}
- SHACL validation report: {shacl_hash}
- expected OCEL manifest: {manifest_hash}
- AutoReceipt bundle: {bundle_hash}
- AutoReceipt matrix: {matrix_hash}
- closure certificate: {cert_hash}

Verifier Output:
- expected OCEL count = {total_jtbds}
- observed OCEL count from real execution = {obs_count}
- alignment receipts valid for closure = {align_count}
- AutoReceiptReady = {ready_jtbds == total_jtbds}
- EvidenceIncomplete = {ready_jtbds < total_jtbds}
- DirtyTree = {bool(tree)}

Remaining Blockers:
- {total_jtbds - ready_jtbds} JTBDs require execution traces (Observed OCEL) to align with Expected OCEL
- {'Dirty git tree prevents total closure' if tree else 'None'}

Next Command:
pnpm run validate:receipts""")
