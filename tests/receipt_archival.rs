//! Round 4 WD — receipt cold-storage archival.
//!
//! Insert 5 receipts with `granted_at = now - 400 days`; run
//! `archive_receipts(365, tmp_dir)`; assert hot-table rows are gone and
//! `lookup_archived(hash) == Some(_)`. Then run `verify_artifact` against
//! a TTL stamped with one of the archived receipt hashes, with the
//! `OPEN_ONTOLOGIES_RECEIPT_ARCHIVE_DIR` env var pointed at the archive,
//! and assert the verdict is `Admitted` with `source: "archive"`.

use open_ontologies::receipt_archive::{archive_receipts, lookup_archived};
use open_ontologies::state::StateDb;
use open_ontologies::verify::{verify_artifact, Verdict};
use tempfile::tempdir;

fn fresh_db() -> (StateDb, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let path = dir.path().join("archive-test.db");
    let db = StateDb::open(&path).expect("open StateDb");
    (db, dir)
}

fn long_ago() -> String {
    (chrono::Utc::now() - chrono::Duration::days(400)).to_rfc3339()
}

fn insert_old_receipt(db: &StateDb, hash: &str, sequence: i64) {
    let conn = db.conn();
    conn.execute(
        "INSERT INTO receipts (
            receipt_hash, scope_token, session_id,
            artifact_hash, declared_powl_hash, ocel_canonical_hash,
            gate_config_hash, prior_receipt_hash,
            production_law_version, granted_at, sequence, tenant_id,
            key_valid_at
         ) VALUES (?1,'scope-x','session-x',?1,?1,?1,?1,NULL,'ontostar-1.0.0',?2,?3,'default','')",
        rusqlite::params![hash, long_ago(), sequence],
    )
    .unwrap();
}

#[test]
fn archive_then_lookup_round_trip() {
    let (db, _g) = fresh_db();
    let archive = tempdir().unwrap();

    let hashes: Vec<String> = (0..5)
        .map(|i| {
            let h = format!("{:064x}", 0xdead_beefu64.wrapping_add(i as u64));
            insert_old_receipt(&db, &h, i + 1);
            h
        })
        .collect();
    let count_before: i64 = db
        .conn()
        .query_row("SELECT COUNT(*) FROM receipts", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count_before, 5);

    let stats = archive_receipts(&db, 365, archive.path()).expect("archive");
    assert_eq!(stats.rows_archived, 5);
    assert_eq!(stats.rows_pruned_from_hot, 5);
    assert!(stats.shards_written >= 1);

    let count_after: i64 = db
        .conn()
        .query_row("SELECT COUNT(*) FROM receipts", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count_after, 0, "hot table must be empty after archive");

    for h in &hashes {
        let found = lookup_archived(archive.path(), h)
            .expect("lookup")
            .expect("hash present in archive");
        assert_eq!(found.receipt_hash, *h);
        assert_eq!(found.tenant_id, "default");
    }
}

#[test]
fn lookup_returns_none_when_archive_empty() {
    let archive = tempdir().unwrap();
    let result = lookup_archived(archive.path(), "deadbeef").expect("lookup");
    assert!(result.is_none());
}

#[test]
fn archive_skips_recent_receipts() {
    let (db, _g) = fresh_db();
    let archive = tempdir().unwrap();
    // Insert one recent receipt (now) — it must NOT be archived.
    {
        let conn = db.conn();
        conn.execute(
            "INSERT INTO receipts (
                receipt_hash, scope_token, session_id,
                artifact_hash, declared_powl_hash, ocel_canonical_hash,
                gate_config_hash, prior_receipt_hash,
                production_law_version, granted_at, sequence, tenant_id,
                key_valid_at
             ) VALUES ('recent','s','s','a','d','o','g',NULL,'v',?1,1,'default','')",
            rusqlite::params![chrono::Utc::now().to_rfc3339()],
        )
        .unwrap();
    }
    let stats = archive_receipts(&db, 365, archive.path()).expect("archive");
    assert_eq!(stats.rows_archived, 0);
    assert_eq!(stats.rows_pruned_from_hot, 0);
    let count_after: i64 = db
        .conn()
        .query_row("SELECT COUNT(*) FROM receipts", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count_after, 1, "recent receipts must remain hot");
}

#[test]
fn verify_falls_through_to_archive_on_hot_miss() {
    // Doctrine: §29 retirement closure — onto_verify must resolve archived
    // receipts so the chain walker still works for old artifacts.
    let (db, _g) = fresh_db();
    let archive = tempdir().unwrap();
    let hash = "ab".repeat(32); // 64-char lowercase hex
    insert_old_receipt(&db, &hash, 1);
    let _stats = archive_receipts(&db, 365, archive.path()).expect("archive");

    // Stamp a TTL test artifact with that receipt hash + scope token.
    let artifact_dir = tempdir().unwrap();
    let ttl_path = artifact_dir.path().join("artifact.ttl");
    let body = "@prefix ex: <http://example.org/> .\nex:s a ex:T .\n";
    let body_hash = blake3::hash(body.as_bytes()).to_hex().to_string();
    let header = format!(
        "# ostar-production-law: ontostar-1.0.0\n\
         # ostar-defects-taxonomy: 4.3.0\n\
         # ostar-receipt-hash: {hash}\n\
         # ostar-artifact-hash: {body_hash}\n\
         # ostar-scope-token: scope-x\n\
         # ostar-prior-receipt: none\n",
    );
    let mut buf = Vec::new();
    buf.extend_from_slice(header.as_bytes());
    buf.extend_from_slice(body.as_bytes());
    std::fs::write(&ttl_path, &buf).unwrap();

    // Point env var at the archive dir.
    let prev = std::env::var("OPEN_ONTOLOGIES_RECEIPT_ARCHIVE_DIR").ok();
    // SAFETY: tests in one binary serialize via cargo's runner.
    unsafe {
        std::env::set_var(
            "OPEN_ONTOLOGIES_RECEIPT_ARCHIVE_DIR",
            archive.path().display().to_string(),
        );
    }

    let verdict = verify_artifact(&ttl_path, Some(&db));
    match &verdict {
        Verdict::Admitted {
            receipt_hash,
            source,
            ..
        } => {
            assert_eq!(receipt_hash, &hash);
            assert_eq!(source, "archive", "source must be \"archive\" on hot-miss + archive-hit");
        }
        other => panic!("expected Admitted with source=archive, got {other:?}"),
    }

    // Restore env var.
    // SAFETY: see above.
    unsafe {
        match prev {
            Some(v) => std::env::set_var("OPEN_ONTOLOGIES_RECEIPT_ARCHIVE_DIR", v),
            None => std::env::remove_var("OPEN_ONTOLOGIES_RECEIPT_ARCHIVE_DIR"),
        }
    }
}
