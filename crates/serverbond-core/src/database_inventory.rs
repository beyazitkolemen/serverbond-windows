//! Explicit MySQL databases and local accounts. No project or .env is modified.
use crate::Manager;
use anyhow::{bail, ensure, Result};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

const RESERVED: &[&str] = &["mysql", "information_schema", "performance_schema", "sys"];
const LOCAL_HOST: &str = "127.0.0.1";

fn valid_name(name: &str, max: usize) -> bool {
    name.len() <= max
        && name.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

fn database_name(name: &str) -> Result<()> {
    ensure!(
        valid_name(name, 64) && !RESERVED.contains(&name),
        "Geçersiz veritabanı adı."
    );
    Ok(())
}

fn user_name(name: &str) -> Result<()> {
    ensure!(
        valid_name(name, 32) && name != "root",
        "Geçersiz MySQL kullanıcı adı."
    );
    Ok(())
}

fn rows(output: &str, fields: usize) -> Result<Vec<Vec<&str>>> {
    output
        .lines()
        .map(|line| {
            let columns: Vec<_> = line.split('\t').collect();
            ensure!(columns.len() == fields, "MySQL envanter yanıtı geçersiz.");
            Ok(columns)
        })
        .collect()
}

// A database-level GRANT interprets a bare underscore as a wildcard.
// Only literal database scopes can be represented by this inventory.
fn grant_scope(raw: &str) -> Option<String> {
    let mut name = String::new();
    let mut chars = raw.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' if chars.next() == Some('_') => name.push('_'),
            'a'..='z' | '0'..='9' => name.push(ch),
            _ => return None,
        }
    }
    database_name(&name).ok()?;
    Some(name)
}

fn inventory_from_queries(schemas: &str, grants: &str) -> Result<Value> {
    let mut databases = BTreeSet::new();
    for row in rows(schemas, 1)? {
        let name = row[0];
        if database_name(name).is_ok() {
            databases.insert(name.to_string());
        }
    }
    ensure!(
        databases.len() <= 100,
        "En fazla 100 veritabanı gösterilebilir."
    );

    let mut users: BTreeMap<String, (BTreeSet<String>, bool)> = BTreeMap::new();
    for row in rows(grants, 4)? {
        let [grantee, schema, privilege, grantable] = [row[0], row[1], row[2], row[3]];
        let Some(name) = grantee
            .strip_suffix("'@'127.0.0.1'")
            .and_then(|part| part.strip_prefix('\''))
        else {
            continue;
        };
        let Some(schema) = grant_scope(schema) else {
            continue;
        };
        if user_name(name).is_err() || !databases.contains(&schema) {
            continue;
        }
        let entry = users
            .entry(name.to_string())
            .or_insert_with(|| (BTreeSet::new(), true));
        entry.0.insert(schema);
        if privilege != "SELECT" || grantable != "NO" {
            entry.1 = false;
        }
    }
    ensure!(
        users.len() <= 100,
        "En fazla 100 MySQL kullanıcısı gösterilebilir."
    );
    Ok(json!({
        "databases": databases.into_iter().map(|name| json!({"name":name})).collect::<Vec<_>>(),
        "users": users.into_iter().map(|(name, (databases, read_only))| json!({
            "name":name,"databases":databases,"readOnly":read_only
        })).collect::<Vec<_>>()
    }))
}

impl Manager {
    pub fn database_inventory(&self) -> Result<Value> {
        let _guard = self.gate()?;
        let schemas = self.mysql_query(
            "SELECT SCHEMA_NAME FROM information_schema.SCHEMATA ORDER BY SCHEMA_NAME",
        )?;
        let grants = self.mysql_query("SELECT GRANTEE, TABLE_SCHEMA, PRIVILEGE_TYPE, IS_GRANTABLE FROM information_schema.SCHEMA_PRIVILEGES ORDER BY GRANTEE, TABLE_SCHEMA, PRIVILEGE_TYPE")?;
        inventory_from_queries(&schemas, &grants)
    }

    pub fn create_named_database(&self, name: &str) -> Result<()> {
        database_name(name)?;
        let _guard = self.gate()?;
        let existing = self.mysql_query(&format!(
            "SELECT SCHEMA_NAME FROM information_schema.SCHEMATA WHERE SCHEMA_NAME = '{name}'"
        ))?;
        ensure!(existing.is_empty(), "Bu veritabanı zaten var.");
        let collation = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .settings
            .mysql
            .collation
            .clone();
        ensure!(
            !collation.is_empty()
                && collation
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_'),
            "MySQL sıralama ayarı geçersiz."
        );
        self.mysql_query(&format!(
            "CREATE DATABASE `{name}` CHARACTER SET utf8mb4 COLLATE {collation}"
        ))?;
        self.log(format!("Veritabanı oluşturuldu: {name}"));
        Ok(())
    }

    pub fn create_database_user(
        &self,
        username: &str,
        password: &str,
        databases: &[String],
        read_only: bool,
        confirm: bool,
    ) -> Result<()> {
        ensure!(confirm, "MySQL kullanıcısı oluşturma onayı gerekli.");
        user_name(username)?;
        crate::model::validate_mysql_password(password)?;
        ensure!(
            !databases.is_empty() && databases.len() <= 20,
            "1–20 veritabanı seçin."
        );
        let mut unique = BTreeSet::new();
        for database in databases {
            database_name(database)?;
            ensure!(
                unique.insert(database.as_str()),
                "Veritabanı birden fazla seçildi."
            );
        }

        let _guard = self.gate()?;
        let existing = self.mysql_query(&format!(
            "SELECT User FROM mysql.user WHERE User = '{username}' AND Host = '{LOCAL_HOST}'"
        ))?;
        ensure!(existing.is_empty(), "Bu MySQL kullanıcısı zaten var.");
        let list = unique
            .iter()
            .map(|name| format!("'{name}'"))
            .collect::<Vec<_>>()
            .join(",");
        let found = self.mysql_query(&format!(
            "SELECT SCHEMA_NAME FROM information_schema.SCHEMATA WHERE SCHEMA_NAME IN ({list})"
        ))?;
        let found = found.lines().collect::<BTreeSet<_>>();
        ensure!(
            found == unique,
            "Seçilen veritabanlarından biri bulunamadı."
        );

        // mysql_query sends SQL through a temporary stdin file, never argv.
        // A failed query may include input in stderr, so keep secret SQL errors generic.
        let account = format!("'{username}'@'{LOCAL_HOST}'");
        self.mysql_query(&format!("CREATE USER {account} IDENTIFIED BY '{password}'"))
            .map_err(|_| anyhow::anyhow!("MySQL kullanıcısı oluşturulamadı."))?;
        for database in databases {
            let privilege = if read_only {
                "SELECT"
            } else {
                "ALL PRIVILEGES"
            };
            let escaped = database.replace('_', "\\_");
            if self
                .mysql_query(&format!("GRANT {privilege} ON `{escaped}`.* TO {account}"))
                .is_err()
            {
                if self.mysql_query(&format!("DROP USER {account}")).is_err() {
                    bail!("MySQL yetkisi verilemedi; oluşturulan kullanıcı otomatik silinemedi. MySQL hesabını denetleyin.");
                }
                bail!("MySQL yetkisi verilemedi; oluşturulan kullanıcı geri alındı.");
            }
        }
        self.log(format!("MySQL kullanıcısı oluşturuldu: {username}"));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_and_inventory_reject_system_names_and_hide_other_hosts() {
        assert!(database_name("app_db").is_ok());
        assert!(database_name("mysql").is_err());
        assert!(database_name("app`db").is_err());
        assert!(user_name("app_user").is_ok());
        assert!(user_name("root").is_err());
        let data = inventory_from_queries(
            "app_db\nlog_db\nmysql",
            "'app_user'@'127.0.0.1'\tapp\\_db\tSELECT\tNO\n'app_user'@'127.0.0.1'\tlog\\_db\tINSERT\tNO\n'other'@'%'\tapp\\_db\tSELECT\tNO\n'root'@'127.0.0.1'\tapp\\_db\tALL PRIVILEGES\tYES",
        )
        .unwrap();
        assert_eq!(data["databases"].as_array().unwrap().len(), 2);
        assert_eq!(data["users"].as_array().unwrap().len(), 1);
        assert_eq!(data["users"][0]["name"], "app_user");
        assert_eq!(data["users"][0]["readOnly"], false);
        assert_eq!(data["users"][0]["databases"], json!(["app_db", "log_db"]));
        assert!(grant_scope("app_db").is_none());
        assert_eq!(grant_scope("app\\_db"), Some("app_db".into()));
    }

    #[test]
    fn invalid_user_request_does_not_touch_mysql() {
        let home = tempfile::tempdir().unwrap();
        let manager = Manager::new(home.path().into()).unwrap();
        let databases = vec!["app_db".to_string()];
        assert!(manager
            .create_database_user("app_user", "Safe-pass-123!", &databases, false, false)
            .is_err());
        assert!(manager
            .create_database_user("root", "Safe-pass-123!", &databases, false, true)
            .is_err());
        assert!(manager
            .create_database_user("app_user", "bad'pass123", &databases, false, true)
            .is_err());
        assert!(manager
            .create_database_user(
                "app_user",
                "Safe-pass-123!",
                &["app_db".into(), "app_db".into()],
                false,
                true
            )
            .is_err());
        assert!(manager.create_named_database("bad`name").is_err());
        assert!(!home.path().join("config/mysql-password.dpapi").exists());
    }
}
