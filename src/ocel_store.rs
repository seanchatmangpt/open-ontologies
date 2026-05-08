use crate::state::StateDb;
use anyhow::Result;
use chrono::FixedOffset;
use wasm4pm_types::{OCELObject, OCEL};
use std::collections::BTreeSet;

pub struct OcelStore {
    db: StateDb,
}

/// Receipt-backed exemplar row returned by [`OcelStore::exemplars_for_domain`].
/// Loop 4 surface — see `feedback::exemplars` (Loop 1) for how rows arrive.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Exemplar {
    pub id: String,
    pub domain: String,
    pub problem_context: String,
    pub powl_string: String,
    pub build_order: Option<String>,
    pub fitness: f64,
    pub source_session: Option<String>,
    pub receipt_hash: String,
    pub mined_at: String,
}

impl OcelStore {
    pub fn new(db: StateDb) -> Self {
        Self { db }
    }

    /// Borrow the underlying state database.
    pub fn db(&self) -> &StateDb {
        &self.db
    }

    /// Loop 4 (cross-session retrieval). Return exemplars for `domain` whose
    /// `mined_exemplars.receipt_hash` JOINs successfully against `receipts`.
    /// The JOIN is the integrity proof: an exemplar without a matching receipt
    /// row never escapes this function. Ordered by fitness DESC then mined_at DESC.
    pub fn exemplars_for_domain(
        &self,
        domain: &str,
        min_fitness: f64,
        limit: usize,
    ) -> Result<Vec<Exemplar>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT m.id, m.domain, m.problem_context, m.powl_string, m.build_order,
                    m.fitness, m.source_session, m.receipt_hash, m.mined_at
             FROM mined_exemplars m
             JOIN receipts r ON m.receipt_hash = r.receipt_hash
             WHERE m.domain = ?1 AND m.fitness >= ?2
             ORDER BY m.fitness DESC, m.mined_at DESC
             LIMIT ?3",
        )?;
        let rows = stmt
            .query_map(
                rusqlite::params![domain, min_fitness, limit as i64],
                |r| {
                    Ok(Exemplar {
                        id: r.get(0)?,
                        domain: r.get(1)?,
                        problem_context: r.get(2)?,
                        powl_string: r.get(3)?,
                        build_order: r.get(4)?,
                        fitness: r.get(5)?,
                        source_session: r.get(6)?,
                        receipt_hash: r.get(7)?,
                        mined_at: r.get(8)?,
                    })
                },
            )?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Stream-3 helper: does a declared workflow row exist for the scope?
    pub fn has_declared_workflow(&self, scope_token: &str) -> Result<bool> {
        let conn = self.db.conn();
        let _ = conn.execute_batch(crate::receipts::STREAM3_STUB_MIGRATION);
        let n: i64 = conn.query_row(
            "SELECT COUNT(*) FROM declared_workflows WHERE scope_token = ?1",
            rusqlite::params![scope_token],
            |r| r.get(0),
        ).unwrap_or(0);
        Ok(n > 0)
    }

    /// Stream-3 helper: is the scope closed (closed_at IS NOT NULL)?
    pub fn is_scope_closed(&self, scope_token: &str) -> Result<bool> {
        let conn = self.db.conn();
        let _ = conn.execute_batch(crate::receipts::STREAM3_STUB_MIGRATION);
        let closed: Option<String> = conn.query_row(
            "SELECT closed_at FROM declared_workflows WHERE scope_token = ?1",
            rusqlite::params![scope_token],
            |r| r.get(0),
        ).unwrap_or(None);
        Ok(closed.is_some())
    }

    /// Stream-3 helper: does a conforming replay exist for the scope?
    pub fn has_conforming_replay(&self, scope_token: &str) -> Result<bool> {
        let conn = self.db.conn();
        let _ = conn.execute_batch(crate::receipts::STREAM3_STUB_MIGRATION);
        let n: i64 = conn.query_row(
            "SELECT COUNT(*) FROM conformance_runs WHERE scope_token = ?1 AND verdict = 'conform'",
            rusqlite::params![scope_token],
            |r| r.get(0),
        ).unwrap_or(0);
        Ok(n > 0)
    }

    /// Stream-3 helper: is the session in revoked_sessions (and not cleared)?
    pub fn session_is_revoked(&self, session_id: &str) -> Result<bool> {
        let conn = self.db.conn();
        let _ = conn.execute_batch(crate::receipts::STREAM3_STUB_MIGRATION);
        let n: i64 = conn.query_row(
            "SELECT COUNT(*) FROM revoked_sessions WHERE session_id = ?1 AND cleared_at IS NULL",
            rusqlite::params![session_id],
            |r| r.get(0),
        ).unwrap_or(0);
        Ok(n > 0)
    }

    /// Stream 2: project OCEL events for `scope_token` to an ordered trace
    /// (event_type values in `time` order) and replay against the parsed POWL
    /// `root` via [`crate::powl_bridge::PowlBridge`].
    ///
    /// **No PM math here.** All fitness/replay numbers come from
    /// `wasm4pm::powl::conformance::token_replay` via the bridge. Persists a
    /// row in `conformance_runs` and returns the typed
    /// [`crate::powl_bridge::ConformanceResult`].
    pub fn replay_against_powl(
        &self,
        scope_token: &str,
        bridge: &crate::powl_bridge::PowlBridge,
        root: u32,
    ) -> Result<crate::powl_bridge::ConformanceResult> {
        // Make sure the conformance_runs table exists. The Stream-3 stub
        // migration is idempotent so cheap to run.
        let conn = self.db.conn();
        let _ = conn.execute_batch(crate::receipts::STREAM3_STUB_MIGRATION);

        // Project event_type values for scope_token in time order.
        let trace: Vec<String> = {
            let mut stmt = conn.prepare(
                "SELECT event_type FROM ocel_events
                 WHERE scope_token = ?1
                 ORDER BY time ASC, event_id ASC",
            )?;
            let rows = stmt.query_map(rusqlite::params![scope_token], |r| r.get::<_, String>(0))?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r?);
            }
            out
        };

        // Delegate replay + classification to wasm4pm-backed bridge.
        let replay = bridge
            .replay_trace(root, &trace)
            .map_err(|e| anyhow::anyhow!("powl replay: {e}"))?;
        let result = crate::powl_bridge::classify_replay(bridge, root, &trace, &replay);

        // Persist conformance_runs row.
        let defects_json = serde_json::to_string(
            &result
                .defects
                .iter()
                .map(|(d, _)| d)
                .collect::<Vec<_>>(),
        )
        .unwrap_or_else(|_| "[]".to_string());
        let _ = conn.execute(
            "INSERT OR REPLACE INTO conformance_runs (
                run_id, scope_token, fitness, precision, generalization, simplicity,
                verdict, defects_json, trace_canonical_hash, ran_at
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            rusqlite::params![
                result.run_id,
                scope_token,
                result.fitness,
                result.precision,
                result.generalization,
                result.simplicity,
                result.verdict,
                defects_json,
                result.trace_canonical_hash,
                chrono::Utc::now().to_rfc3339(),
            ],
        )?;

        Ok(result)
    }

    /// Replay a scope using only the OCEL stream — no `declared_workflows`
    /// row required. Reads the `workflow_declared` anchor event for the
    /// scope, extracts `powl_string` from its attributes, parses it into a
    /// fresh `PowlBridge`, and replays. This proves the OCEL stream is
    /// self-sufficient: an external observer with only the event log can
    /// reconstruct what should have happened.
    ///
    /// Errors when no anchor event exists (the scope was never properly
    /// declared) or when the embedded POWL fails to parse.
    pub fn replay_from_ocel_alone(
        &self,
        scope_token: &str,
    ) -> Result<crate::powl_bridge::ConformanceResult> {
        // Find the workflow_declared anchor event for this scope and pull
        // its powl_string attribute. Drop the connection lock before the
        // (potentially heavy) parse so we don't serialize replays.
        let powl_string: String = {
            let conn = self.db.conn();
            let anchor_event_id: String = conn
                .query_row(
                    "SELECT event_id FROM ocel_events
                     WHERE scope_token = ?1 AND event_type = 'workflow_declared'
                     ORDER BY time ASC LIMIT 1",
                    rusqlite::params![scope_token],
                    |r| r.get(0),
                )
                .map_err(|_| anyhow::anyhow!(
                    "no workflow_declared anchor event for scope {}",
                    scope_token
                ))?;
            conn.query_row(
                "SELECT value FROM ocel_event_attrs
                 WHERE event_id = ?1 AND name = 'powl_string'",
                rusqlite::params![anchor_event_id],
                |r| r.get(0),
            )
            .map_err(|_| anyhow::anyhow!(
                "anchor event {} missing powl_string attribute",
                anchor_event_id
            ))?
        };

        let mut bridge = crate::powl_bridge::PowlBridge::new();
        let root = bridge
            .parse(&powl_string)
            .map_err(|e| anyhow::anyhow!("anchor powl parse failed: {e}"))?;

        // Reuse the canonical replay path. This re-reads the same events but
        // does NOT touch declared_workflows.
        self.replay_against_powl(scope_token, &bridge, root)
    }

    /// Stream-3 helper: list event_types observed for a session.
    pub fn observed_event_types_for_session(&self, session_id: &str) -> Result<Vec<String>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT DISTINCT event_type FROM ocel_events WHERE session_id = ?1 ORDER BY event_type ASC"
        )?;
        let rows = stmt.query_map(rusqlite::params![session_id], |r| r.get::<_, String>(0))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Idempotent object upsert. Creates or updates an OCEL object and its attributes.
    pub fn upsert_object(
        &self,
        id: &str,
        object_type: &str,
        attrs: &[(&str, &str, &str)],
    ) -> Result<()> {
        let conn = self.db.conn();
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT OR IGNORE INTO ocel_objects (object_id, object_type, created_at)
             VALUES (?1, ?2, ?3)",
            rusqlite::params![id, object_type, &now],
        )?;

        for (name, value, value_type) in attrs {
            conn.execute(
                "INSERT INTO ocel_object_attrs (object_id, name, value, value_type, valid_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![id, name, value, value_type, &now],
            )?;
        }

        Ok(())
    }

    /// Emit one OCEL event with attributes and relationships to objects.
    ///
    /// `scope_token` (Stream 1) tags the event with an open
    /// [`crate::workflows::WorkflowScope`] so OntoStar admission can replay
    /// scoped traces. Pass `None` when the call site has no declared scope —
    /// Stream 3 fills these in for gated handlers.
    pub fn emit_event(
        &self,
        event_id: &str,
        event_type: &str,
        time_iso: &str,
        session_id: &str,
        attrs: &[(&str, &str)],
        objects: &[(&str, &str)],
        scope_token: Option<&str>,
    ) -> Result<()> {
        let conn = self.db.conn();

        conn.execute(
            "INSERT INTO ocel_events (event_id, event_type, time, session_id, scope_token)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![event_id, event_type, time_iso, session_id, scope_token],
        )?;

        for (name, value) in attrs {
            conn.execute(
                "INSERT INTO ocel_event_attrs (event_id, name, value, value_type)
                 VALUES (?1, ?2, ?3, 'string')",
                rusqlite::params![event_id, name, value],
            )?;
        }

        for (object_id, qualifier) in objects {
            conn.execute(
                "INSERT INTO ocel_relationships (event_id, object_id, qualifier)
                 VALUES (?1, ?2, ?3)",
                rusqlite::params![event_id, object_id, qualifier],
            )?;
        }

        Ok(())
    }

    /// Build a complete OCEL 2.0 struct from the stored OCEL data.
    pub fn build_ocel(&self, session_id_filter: Option<&str>) -> Result<OCEL> {
        let conn = self.db.conn();

        // Query objects
        let mut object_type_set = BTreeSet::new();
        let mut objects = Vec::new();

        let mut stmt = conn.prepare(
            "SELECT object_id, object_type FROM ocel_objects ORDER BY object_id ASC",
        )?;

        let obj_rows = stmt.query_map(rusqlite::params![], |row| {
            let id: String = row.get(0)?;
            let otype: String = row.get(1)?;
            Ok((id, otype))
        })?;

        for row_result in obj_rows {
            let (id, otype) = row_result?;
            object_type_set.insert(otype.clone());

            objects.push(OCELObject {
                id,
                object_type: otype,
                attributes: Vec::new(),
                relationships: Vec::new(),
            });
        }

        // Query events
        let mut event_type_set = BTreeSet::new();
        let mut events = Vec::new();

        let event_rows: Vec<(String, String, String, String)> = if let Some(sid) = session_id_filter {
            let mut stmt = conn.prepare(
                "SELECT event_id, event_type, time, session_id FROM ocel_events
                 WHERE session_id = ?1 ORDER BY event_id ASC",
            )?;
            stmt.query_map(rusqlite::params![sid], |row| {
                let eid: String = row.get(0)?;
                let etype: String = row.get(1)?;
                let time_str: String = row.get(2)?;
                let sid: String = row.get(3)?;
                Ok((eid, etype, time_str, sid))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?
        } else {
            let mut stmt = conn.prepare(
                "SELECT event_id, event_type, time, session_id FROM ocel_events ORDER BY event_id ASC",
            )?;
            stmt.query_map(rusqlite::params![], |row| {
                let eid: String = row.get(0)?;
                let etype: String = row.get(1)?;
                let time_str: String = row.get(2)?;
                let sid: String = row.get(3)?;
                Ok((eid, etype, time_str, sid))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?
        };

        let utc_now = chrono::Utc::now();
        let fixed_now = utc_now.with_timezone(&FixedOffset::east_opt(0).unwrap());

        for (eid, etype, time_str, _sid) in event_rows {
            event_type_set.insert(etype.clone());

            let time = chrono::DateTime::parse_from_rfc3339(&time_str).unwrap_or(fixed_now);

            // Query event attributes
            let mut attr_stmt = conn.prepare(
                "SELECT name, value FROM ocel_event_attrs WHERE event_id = ?1",
            )?;

            let attributes = attr_stmt
                .query_map(rusqlite::params![&eid], |row| {
                    let name: String = row.get(0)?;
                    let value: String = row.get(1)?;
                    Ok((name, value))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;

            // Query relationships
            let mut rel_stmt =
                conn.prepare("SELECT object_id, qualifier FROM ocel_relationships WHERE event_id = ?1")?;

            let relationships = rel_stmt
                .query_map(rusqlite::params![&eid], |row| {
                    let oid: String = row.get(0)?;
                    let qual: String = row.get(1)?;
                    Ok((oid, qual))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;

            events.push((eid, etype, time, attributes, relationships));
        }

        // Build event_types
        let event_types: Vec<(String,)> = event_type_set.iter().map(|n| (n.clone(),)).collect();

        Ok(OCEL {
            event_types: event_types.iter().map(|(n,)| {
                use wasm4pm_types::ocel::OCELType;
                OCELType {
                    name: n.clone(),
                    attributes: Vec::new(),
                }
            }).collect(),
            object_types: object_type_set.iter().map(|n| {
                use wasm4pm_types::ocel::OCELType;
                OCELType {
                    name: n.clone(),
                    attributes: Vec::new(),
                }
            }).collect(),
            events: events.into_iter().map(|(eid, etype, time, attrs, rels)| {
                use wasm4pm_types::ocel::{OCELEvent, OCELEventAttribute, OCELAttributeValue, OCELRelationship};
                OCELEvent {
                    id: eid,
                    event_type: etype,
                    time,
                    attributes: attrs.iter().map(|(name, value)| {
                        OCELEventAttribute {
                            name: name.clone(),
                            value: OCELAttributeValue::String(value.clone()),
                        }
                    }).collect(),
                    relationships: rels.iter().map(|(oid, qual)| {
                        OCELRelationship {
                            object_id: oid.clone(),
                            qualifier: qual.clone(),
                        }
                    }).collect(),
                }
            }).collect(),
            objects,
        })
    }

    pub fn insert_seed_exemplar(
        &self,
        domain: &str,
        problem_statement: &str,
        powl_model: &str,
        build_order: &str,
        sequence_diagram: &str,
        fitness: f64,
    ) -> Result<String> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let seed_input = format!("{}|{}|{}", domain, problem_statement, powl_model);
        let mut h = DefaultHasher::new();
        seed_input.hash(&mut h);
        let receipt_hash = format!("seed-v0-{:016x}", h.finish());
        let canonical = serde_json::json!({
            "kind": "seed",
            "domain": domain,
            "problem": problem_statement,
            "powl_model": powl_model,
            "fitness": fitness,
        })
        .to_string();
        let conn = self.db.conn();
        conn.execute(
            "INSERT OR IGNORE INTO receipts (receipt_hash, scope_token, production_law_version, canonical_record, parent_hash)
             VALUES (?1, NULL, 'seed-v0', ?2, NULL)",
            rusqlite::params![receipt_hash, canonical],
        )?;
        conn.execute(
            "INSERT INTO mined_exemplars (domain, problem_statement, powl_model, build_order, sequence_diagram, fitness, source_scope_token, receipt_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7)",
            rusqlite::params![
                domain,
                problem_statement,
                powl_model,
                build_order,
                sequence_diagram,
                fitness,
                receipt_hash
            ],
        )?;
        Ok(receipt_hash)
    }

    pub fn seed_from_ocel_bytes(&self, bytes: &[u8], default_domain: &str) -> Result<u64> {
        let doc: serde_json::Value = serde_json::from_slice(bytes)?;
        let events: Vec<serde_json::Value> = if let Some(arr) =
            doc.get("events").and_then(|v| v.as_array())
        {
            arr.clone()
        } else if let Some(obj) = doc.get("ocel:events").and_then(|v| v.as_object()) {
            obj.values().cloned().collect()
        } else if let Some(arr) = doc.get("ocel:events").and_then(|v| v.as_array()) {
            arr.clone()
        } else {
            return Ok(0);
        };
        let mut inserted = 0u64;
        for ev in events {
            let etype = ev
                .get("ocel:activity")
                .or_else(|| ev.get("activity"))
                .or_else(|| ev.get("ocel:type"))
                .or_else(|| ev.get("type"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if etype != "build_order_generated" {
                continue;
            }
            let attrs = ev
                .get("ocel:attributes")
                .or_else(|| ev.get("attributes"))
                .or_else(|| ev.get("vmap"))
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let s = |k: &str| -> String {
                attrs.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string()
            };
            let problem = s("problem_statement");
            let powl_model = if !s("powl_model").is_empty() {
                s("powl_model")
            } else {
                s("powl_string")
            };
            if powl_model.is_empty() {
                continue;
            }
            let domain = if !s("domain").is_empty() {
                s("domain")
            } else {
                default_domain.to_string()
            };
            let fitness = attrs.get("fitness").and_then(|v| v.as_f64()).unwrap_or(1.0);
            self.insert_seed_exemplar(
                &domain,
                &problem,
                &powl_model,
                &s("build_order"),
                &s("sequence_diagram"),
                fitness,
            )?;
            inserted += 1;
        }
        Ok(inserted)
    }
}
