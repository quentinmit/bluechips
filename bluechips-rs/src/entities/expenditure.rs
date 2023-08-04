//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.1

use sea_orm::entity::prelude::*;
use super::currency::Currency;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "expenditures")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub spender_id: i32,
    pub amount: Currency,
    #[sea_orm(column_type = "Text", nullable)]
    pub description: Option<String>,
    pub date: Option<Date>,
    pub entered_time: Option<DateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}