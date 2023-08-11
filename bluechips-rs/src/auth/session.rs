use std::time::Duration;
use chashmap::CHashMap;
use super::Result;
use rocket::serde::{Serialize, Deserialize};

#[rocket::async_trait]
pub trait SessionManager: Send + Sync {
    async fn insert(&self, id: i32, key: String) -> Result<()>;
    async fn insert_for(&self, id: i32, key: String, time: Duration) -> Result<()>;
    async fn remove(&self, id: i32) -> Result<()>;
    async fn get(&self, id: i32) -> Option<String>;
    async fn clear_all(&self) -> Result<()>;
    async fn clear_expired(&self) -> Result<()>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthKey {
    expires: i64,
    secret: String,
}

impl From<String> for AuthKey {
    fn from(secret: String) -> AuthKey {
        AuthKey {
            expires: 31536000,
            secret
        }
    }
}

impl From<&str> for AuthKey {
    fn from(secret: &str) -> AuthKey {
        AuthKey {
            expires: 31536000,
            secret: secret.into()
        }
    }
}

#[rocket::async_trait]
impl SessionManager for CHashMap<i32, AuthKey> {
    async fn insert(&self, id: i32, key: String) -> Result<()> {
        self.insert(id, key.into());
        Ok(())
    }

    async fn remove(&self, id: i32) -> Result<()> {
        self.remove(&id);
        Ok(())
    }

    async fn get(&self, id: i32) -> Option<String> {
        let key = self.get(&id)?;
        Some(key.secret.clone())
    }

    async fn clear_all(&self) -> Result<()> {
        self.clear();
        Ok(())
    }

    async fn insert_for(&self, id: i32, key: String, time: Duration) -> Result<()> {
        let key = AuthKey {
            expires: time.as_secs() as i64,
            secret: key,
        };
        self.insert(id, key);
        Ok(())
    }

    async fn clear_expired(&self) -> Result<()> {
        let time = super::now();
        self.retain(|_, auth_key| auth_key.expires > time);
        Ok(())
    }
}
