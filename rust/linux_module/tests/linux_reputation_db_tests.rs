use linux_module::supervisor::linux_reputation_db::ReputationDatabase;
use std::env;
use std::time::{Duration, Instant};
use tempfile::tempdir;

fn with_temp_base<F, T>(f: F) -> Result<T, Box<dyn std::error::Error>>
where
    F: FnOnce() -> Result<T, Box<dyn std::error::Error>>,
{
    let temp_dir = tempdir()?;
    let base_path = temp_dir.path().to_str().ok_or("Invalid path")?;
    env::set_var("AIOS_BASE_DIR", base_path);
    let result = f();
    env::remove_var("AIOS_BASE_DIR");
    result
}

#[test]
fn test_reputation_db_creation() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let temp_dir = tempdir()?;
        let db_path = temp_dir.path().join("reputation.db");
        let hmac_key = [0x42u8; 32];
        let db = ReputationDatabase::new(db_path.to_str().ok_or("Invalid path")?, hmac_key);
        assert!(db.is_ok());
        Ok(())
    })
}

#[test]
fn test_update_and_get_reputation() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let temp_dir = tempdir()?;
        let db_path = temp_dir.path().join("reputation.db");
        let hmac_key = [0x42u8; 32];
        let db = ReputationDatabase::new(db_path.to_str().ok_or("Invalid path")?, hmac_key)?;

        db.update_reputation("linux", true)?;
        let rep = db.get_reputation("linux").ok_or("reputation not found")?;
        assert_eq!(rep.supervisor_id, "linux");
        assert_eq!(rep.total_votes, 1);
        assert_eq!(rep.successful_votes, 1);
        assert_eq!(rep.score, 1.0);

        db.update_reputation("linux", false)?;
        let rep = db.get_reputation("linux").ok_or("reputation not found")?;
        assert_eq!(rep.total_votes, 2);
        assert_eq!(rep.successful_votes, 1);
        assert_eq!(rep.score, 0.5);
        Ok(())
    })
}

#[test]
fn test_get_all_reputations() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let temp_dir = tempdir()?;
        let db_path = temp_dir.path().join("reputation.db");
        let hmac_key = [0x42u8; 32];
        let db = ReputationDatabase::new(db_path.to_str().ok_or("Invalid path")?, hmac_key)?;

        db.update_reputation("linux", true)?;
        db.update_reputation("windows", false)?;
        db.update_reputation("android", true)?;

        let all = db.get_all_reputations();
        assert_eq!(all.len(), 3);
        assert!(all.contains_key("linux"));
        assert!(all.contains_key("windows"));
        assert!(all.contains_key("android"));
        assert_eq!(
            all.get("linux")
                .ok_or("Linux reputation should exist")?
                .score,
            1.0
        );
        assert_eq!(
            all.get("windows")
                .ok_or("Windows reputation should exist")?
                .score,
            0.0
        );
        assert_eq!(
            all.get("android")
                .ok_or("Android reputation should exist")?
                .score,
            1.0
        );
        Ok(())
    })
}

#[test]
fn test_reputation_persistence() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let temp_dir = tempdir()?;
        let db_path = temp_dir.path().join("reputation.db");
        let hmac_key = [0x42u8; 32];

        {
            let db = ReputationDatabase::new(db_path.to_str().ok_or("Invalid path")?, hmac_key)?;
            db.update_reputation("linux", true)?;
            db.update_reputation("linux", false)?;
            drop(db);
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        {
            let db = ReputationDatabase::new(db_path.to_str().ok_or("Invalid path")?, hmac_key)?;
            let rep = db.get_reputation("linux").ok_or("reputation not found")?;
            assert_eq!(rep.total_votes, 2);
            assert_eq!(rep.score, 0.5);
        }
        Ok(())
    })
}

#[test]
fn test_hmac_integrity() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let temp_dir = tempdir()?;
        let db_path = temp_dir.path().join("reputation.db");
        let hmac_key = [0x42u8; 32];

        {
            let db = ReputationDatabase::new(db_path.to_str().ok_or("Invalid path")?, hmac_key)?;
            db.update_reputation("linux", true)?;
            drop(db);
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        let conn = rusqlite::Connection::open(&db_path)?;
        conn.execute(
            "UPDATE reputations SET score = 0.99 WHERE supervisor_id = 'linux'",
            [],
        )?;

        let db = ReputationDatabase::new(db_path.to_str().ok_or("Invalid path")?, hmac_key)?;
        let rep = db.get_reputation("linux");
        assert!(
            rep.is_none(),
            "HMAC verification should reject tampered data"
        );

        db.update_reputation("linux", true)?;
        let rep = db.get_reputation("linux").ok_or("reputation not found")?;
        assert_eq!(rep.total_votes, 1);
        assert_eq!(rep.score, 1.0);
        Ok(())
    })
}

#[test]
fn test_sqlite_async_path_non_blocking() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let temp_dir = tempdir()?;
        let db_path = temp_dir.path().join("reputation.db");
        let hmac_key = [0x42u8; 32];
        let db = ReputationDatabase::new(db_path.to_str().ok_or("Invalid path")?, hmac_key)?;

        let start = Instant::now();

        for i in 0..100 {
            let id = format!("module_{}", i % 10);
            let success = i % 2 == 0;
            db.update_reputation(&id, success)?;
        }

        let elapsed = start.elapsed();

        assert!(
            elapsed < Duration::from_millis(500),
            "SQLite async path should be fast, took {:?}",
            elapsed
        );

        for i in 0..10 {
            let id = format!("module_{}", i);
            if let Some(rep) = db.get_reputation(&id) {
                assert!(rep.total_votes > 0);
            }
        }
        Ok(())
    })
}

#[test]
fn test_read_not_blocked_by_write() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let temp_dir = tempdir()?;
        let db_path = temp_dir.path().join("reputation.db");
        let hmac_key = [0x42u8; 32];
        let db = ReputationDatabase::new(db_path.to_str().ok_or("Invalid path")?, hmac_key)?;

        for i in 0..20 {
            db.update_reputation(&format!("module_{}", i), true)?;
        }

        let start = Instant::now();

        for _ in 0..1000 {
            let _ = db.get_reputation("module_0");
            let _ = db.get_all_reputations();
        }

        let elapsed = start.elapsed();

        assert!(
            elapsed < Duration::from_millis(100),
            "Reads from DashMap cache should be fast, took {:?}",
            elapsed
        );
        Ok(())
    })
}
