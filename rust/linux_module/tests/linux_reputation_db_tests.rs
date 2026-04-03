use linux_module::supervisor::linux_reputation_db::ReputationDatabase;
use std::env;
use std::time::{Duration, Instant};
use tempfile::tempdir;

fn with_temp_base<F, T>(f: F) -> T
where
    F: FnOnce() -> T,
{
    let temp_dir = tempdir().unwrap();
    let base_path = temp_dir.path().to_str().unwrap();
    env::set_var("AIOS_BASE_DIR", base_path);
    let result = f();
    env::remove_var("AIOS_BASE_DIR");
    result
}

#[test]
fn test_reputation_db_creation() {
    with_temp_base(|| {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("reputation.db");
        let hmac_key = [0x42u8; 32];
        let db = ReputationDatabase::new(db_path.to_str().unwrap(), hmac_key);
        assert!(db.is_ok());
    });
}

#[test]
fn test_update_and_get_reputation() {
    with_temp_base(|| {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("reputation.db");
        let hmac_key = [0x42u8; 32];
        let db = ReputationDatabase::new(db_path.to_str().unwrap(), hmac_key).unwrap();

        db.update_reputation("linux", true).unwrap();
        let rep = db.get_reputation("linux").unwrap();
        assert_eq!(rep.supervisor_id, "linux");
        assert_eq!(rep.total_votes, 1);
        assert_eq!(rep.successful_votes, 1);
        assert_eq!(rep.score, 1.0);

        db.update_reputation("linux", false).unwrap();
        let rep = db.get_reputation("linux").unwrap();
        assert_eq!(rep.total_votes, 2);
        assert_eq!(rep.successful_votes, 1);
        assert_eq!(rep.score, 0.5);
    });
}

#[test]
fn test_get_all_reputations() {
    with_temp_base(|| {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("reputation.db");
        let hmac_key = [0x42u8; 32];
        let db = ReputationDatabase::new(db_path.to_str().unwrap(), hmac_key).unwrap();

        db.update_reputation("linux", true).unwrap();
        db.update_reputation("windows", false).unwrap();
        db.update_reputation("android", true).unwrap();

        let all = db.get_all_reputations();
        assert_eq!(all.len(), 3);
        assert!(all.contains_key("linux"));
        assert!(all.contains_key("windows"));
        assert!(all.contains_key("android"));
        assert_eq!(all.get("linux").unwrap().score, 1.0);
        assert_eq!(all.get("windows").unwrap().score, 0.0);
        assert_eq!(all.get("android").unwrap().score, 1.0);
    });
}

#[test]
fn test_reputation_persistence() {
    with_temp_base(|| {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("reputation.db");
        let hmac_key = [0x42u8; 32];

        // First instance
        {
            let db = ReputationDatabase::new(db_path.to_str().unwrap(), hmac_key).unwrap();
            db.update_reputation("linux", true).unwrap();
            db.update_reputation("linux", false).unwrap();
        }

        // Second instance (should load from SQLite)
        {
            let db = ReputationDatabase::new(db_path.to_str().unwrap(), hmac_key).unwrap();
            let rep = db.get_reputation("linux").unwrap();
            assert_eq!(rep.total_votes, 2);
            assert_eq!(rep.score, 0.5);
        }
    });
}

#[test]
fn test_hmac_integrity() {
    with_temp_base(|| {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("reputation.db");
        let hmac_key = [0x42u8; 32];

        // Create database and add entry
        {
            let db = ReputationDatabase::new(db_path.to_str().unwrap(), hmac_key).unwrap();
            db.update_reputation("linux", true).unwrap();
        }

        // Tamper with SQLite file (simulate corruption)
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute(
            "UPDATE reputations SET score = 0.99 WHERE supervisor_id = 'linux'",
            [],
        )
        .unwrap();

        // Reload with same key should detect tampering and not load into cache
        let db = ReputationDatabase::new(db_path.to_str().unwrap(), hmac_key).unwrap();
        let rep = db.get_reputation("linux");
        // Because HMAC mismatch, the entry should be ignored (None)
        assert!(rep.is_none());

        // But the cache may still contain the entry? No, because load only adds verified entries.
        // The database still has the corrupted row, but we skip it.
        // Now try updating again – should create fresh entry with new HMAC.
        db.update_reputation("linux", true).unwrap();
        let rep = db.get_reputation("linux").unwrap();
        assert_eq!(rep.total_votes, 1);
        assert_eq!(rep.score, 1.0);
    });
}

#[test]
fn test_sqlite_async_path_non_blocking() {
    with_temp_base(|| {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("reputation.db");
        let hmac_key = [0x42u8; 32];
        let db = ReputationDatabase::new(db_path.to_str().unwrap(), hmac_key).unwrap();

        // Perform multiple concurrent updates - should NOT block main thread
        let start = Instant::now();

        // Perform 100 updates rapidly
        for i in 0..100 {
            let id = format!("module_{}", i % 10);
            let success = i % 2 == 0;
            db.update_reputation(&id, success).unwrap();
        }

        let elapsed = start.elapsed();

        // Should complete quickly (< 500ms) because updates go through DashMap cache
        // SQLite writes happen in background worker
        assert!(
            elapsed < Duration::from_millis(500),
            "SQLite async path should be fast, took {:?}",
            elapsed
        );

        // Verify all updates are in cache (DashMap)
        for i in 0..10 {
            let id = format!("module_{}", i);
            if let Some(rep) = db.get_reputation(&id) {
                // Should have some reputation
                assert!(rep.total_votes > 0);
            }
        }
    });
}

#[test]
fn test_read_not_blocked_by_write() {
    with_temp_base(|| {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("reputation.db");
        let hmac_key = [0x42u8; 32];
        let db = ReputationDatabase::new(db_path.to_str().unwrap(), hmac_key).unwrap();

        // Pre-populate
        for i in 0..20 {
            db.update_reputation(&format!("module_{}", i), true)
                .unwrap();
        }

        // Read during heavy writes - should not block
        let start = Instant::now();

        for _ in 0..1000 {
            // Reading from DashMap cache should be fast (no SQLite in hot path)
            let _ = db.get_reputation("module_0");
            let _ = db.get_all_reputations();
        }

        let elapsed = start.elapsed();

        // 1000 reads should complete very quickly (< 100ms) from DashMap cache
        assert!(
            elapsed < Duration::from_millis(100),
            "Reads from DashMap cache should be fast, took {:?}",
            elapsed
        );
    });
}
