use sea_orm_migration::prelude::*;
use sea_orm::Schema;
use sea_orm_migration::sea_orm::EntityName;
use bluechips_rs::entities::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        let schema = Schema::new(manager.get_database_backend());
        manager
            .create_table(schema.create_table_from_entity(user::Entity))
            .await?;
        manager
            .create_table(schema.create_table_from_entity(expenditure::Entity))
            .await?;
        manager
            .create_table(schema.create_table_from_entity(split::Entity))
            .await?;
        manager
            .create_table(schema.create_table_from_entity(subitem::Entity))
            .await?;
        manager
            .create_table(schema.create_table_from_entity(transfer::Entity))
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        manager
            .drop_table(Table::drop().table(transfer::Entity).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(subitem::Entity).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(split::Entity).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(expenditure::Entity).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(user::Entity).to_owned())
            .await
    }
}
