use serde_json::{json, Value};
use crate::autoreceipt_law::{AutoReceiptPipeline, ExpectedOcelManufactured, ArchitecturalReceiptParsed};

/// OCEL Manufacturer. Generates expected OCEL event sequences from architectural intent.
pub struct OcelManufacturer;

impl OcelManufacturer {
    /// Manufacture expected OCEL events from a structured architectural intent.
    pub fn manufacture(intent: &Value) -> Vec<Value> {
        let mut expected_events = Vec::new();
        
        // Example: Map Level 2 Containers to expected lifecycle events
        if let Some(containers) = intent.get("containers").and_then(|v| v.as_array()) {
            for container in containers {
                let name = container.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
                expected_events.push(json!({
                    "ocel:activity": format!("initialize_{}", name.to_lowercase().replace(" ", "_")),
                    "ocel:timestamp": "2026-05-20T12:00:00Z", // Base timestamp
                    "ocel:omap": [name],
                    "ocel:vmap": {
                        "component_type": "container",
                        "architecture_level": 2
                    }
                }));
            }
        }

        // Example: Map Level 3 Components
        if let Some(components) = intent.get("components").and_then(|v| v.as_array()) {
            for component in components {
                let name = component.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
                expected_events.push(json!({
                    "ocel:activity": format!("activate_{}", name.to_lowercase().replace(" ", "_")),
                    "ocel:timestamp": "2026-05-20T12:01:00Z",
                    "ocel:omap": [name],
                    "ocel:vmap": {
                        "component_type": "component",
                        "architecture_level": 3
                    }
                }));
            }
        }

        expected_events
    }
}

/// Transition wrapper for the AutoReceiptPipeline.
pub fn transition_to_expected_ocel(
    pipeline: AutoReceiptPipeline<ArchitecturalReceiptParsed>,
    intent: &Value,
) -> (AutoReceiptPipeline<ExpectedOcelManufactured>, Vec<Value>) {
    let expected = OcelManufacturer::manufacture(intent);
    (pipeline.transition(), expected)
}
