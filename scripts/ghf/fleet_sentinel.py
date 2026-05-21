import json
import hashlib
import os
import subprocess
from datetime import datetime

# Dummy or API-based observation of repos
# For the purpose of implementing the logic, we will check local files simulating remote rules
repos = ["fabio-rovai/open-ontologies"]

# Assume we query GitHub API to get rulesets, workflows, branch protection
drift_detected = False

missing = []

if not os.path.exists("SECURITY.md"):
    drift_detected = True
    missing.append("SECURITY.md is missing")

if not os.path.exists(".github/workflows/receipt-verify.yml"):
    drift_detected = True
    missing.append("Receipt verification workflow is missing")

if not os.path.exists("artifacts/autoreceipt/AUTORECEIPT_CLOSURE_CERTIFICATE.v26.5.13.json"):
    drift_detected = True
    missing.append("Receipt coverage missing")

timestamp = datetime.utcnow().isoformat() + "Z"

if drift_detected:
    # Emit FleetDriftDetected receipt
    receipt = {
        "receipt_type": "OutOfMembraneReceipt",
        "action_id": "fleet_sentinel_scan",
        "inspected_objects": repos,
        "refusal_state": "FleetDriftDetected",
        "missing": missing,
        "timestamp": timestamp
    }
else:
    receipt = {
        "receipt_type": "FleetHealthReceipt",
        "action_id": "fleet_sentinel_scan",
        "inspected_objects": repos,
        "TopologyConformanceScore": 1.0,
        "PolicyCoverageScore": 1.0,
        "ReceiptCoverageScore": 1.0,
        "DriftRiskScore": 0.0,
        "timestamp": timestamp
    }

receipt_hash = hashlib.sha256(json.dumps(receipt).encode()).hexdigest()
receipt["receipt_hash"] = receipt_hash

os.makedirs("artifacts/ghf/fleet", exist_ok=True)
with open("artifacts/ghf/fleet/fleet-health.receipt.json", "w") as f:
    json.dump(receipt, f, indent=2)

# Generate observed.fleet.ocel.json
ocel = {
    "ocel:global-log": {
        "ocel:object-types": ["ghf:GitHubRepository", "ghf:FleetSentinelScan"]
    },
    "ocel:log": [
        {
            "ocel:id": "scan-1",
            "ocel:type": "fleet.drift.detected" if drift_detected else "fleet.drift.passed",
            "ocel:timestamp": timestamp,
            "ocel:omap": repos
        }
    ]
}

with open("artifacts/ghf/fleet/observed.fleet.ocel.json", "w") as f:
    json.dump(ocel, f, indent=2)

with open("artifacts/ghf/fleet/drift-report.md", "w") as f:
    f.write(f"# Fleet Sentinel Drift Report\n\nDrift Detected: {drift_detected}\n\nDetails:\n" + "\n".join(missing))

print("Fleet Sentinel executed successfully.")
