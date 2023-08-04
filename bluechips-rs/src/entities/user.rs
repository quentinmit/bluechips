//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.1

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub username: String,
    pub name: Option<String>,
    pub resident: bool,
    pub email: Option<String>,
    pub password: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::expenditure::Entity")]
    Expenditure,
}

// `Related` trait has to be implemented by hand
impl Related<super::expenditure::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Expenditure.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
