use std::time::Duration;
use sea_orm::DatabaseConnection;
use sea_orm::sea_query::OnConflict;
use sea_orm::{*, prelude::*};

use crate::entities::{auth_session, prelude::AuthSession};
use super::SessionManager;
use super::Result;

#[rocket::async_trait]
impl SessionManager for DatabaseConnection {
    async fn insert(&self, id: i32, key: String) -> Result<()> {
        self.insert_for(id, key, Duration::from_secs(365*24*60*60)).await
    }
    async fn insert_for(&self, id: i32, key: String, expires: Duration) -> Result<()> {
        AuthSession::insert(auth_session::ActiveModel {
            id: Set(id),
            secret: Set(key),
            expires: Set(chrono::Utc::now() + chrono::Duration::from_std(expires).unwrap()),
        })
            .on_conflict(
                OnConflict::column(auth_session::Column::Id)
                .update_columns([auth_session::Column::Secret, auth_session::Column::Expires])
                .to_owned()
            )
            .exec(self)
            .await?;
        Ok(())
    }
    async fn remove(&self, id: i32) -> Result<()> {
        AuthSession::delete_by_id(id).exec(self).await?;
        Ok(())
    }
    async fn get(&self, id: i32) -> Option<String> {
        Some(AuthSession::find_by_id(id).one(self).await.ok()??.secret)
    }
    async fn clear_all(&self) -> Result<()> {
        AuthSession::delete_many().exec(self).await?;
        Ok(())
    }
    async fn clear_expired(&self) -> Result<()> {
        AuthSession::delete_many()
            .filter(auth_session::Column::Expires.lt(chrono::Utc::now()))
            .exec(self)
            .await?;
        Ok(())
    }
}