use crate::entities::{prelude::*, *};
use sea_orm::{prelude::*, *};
use sea_orm::sea_query::{Cond, IntoCondition};

#[derive(FromQueryResult)]
pub struct ExpenditureDisplay {
    pub id: i32,
    pub amount: Currency,
    pub mine: bool,
    pub involved: bool,
    pub description: Option<String>,
    pub date: Option<Date>,
    pub spender_name: String,
    pub share_amount: Currency,
}

pub struct Query;

impl Query {
    fn annotate_expenditures(user_id: i32, select: Select<expenditure::Entity>) -> Selector<SelectModel<ExpenditureDisplay>> {
        select
            .select_only()
            .columns([
                expenditure::Column::Id,
                expenditure::Column::Amount,
                expenditure::Column::Description,
                expenditure::Column::Date,
            ])
            .join(JoinType::InnerJoin, expenditure::Relation::Spender.def())
            .column_as(user::Column::Id.eq(user_id), "mine")
            .column_as(user::Column::Name, "spender_name")
            .join(JoinType::LeftJoin, expenditure::Relation::Split.def().on_condition(move |_, right| {Expr::col((right, split::Column::UserId)).eq(user_id).into_condition()}))
            .column_as(split::Column::Share.if_null(0), "share_amount")
            .column_as(expenditure::Column::SpenderId.eq(user_id).or(split::Column::Share.is_not_null().and(split::Column::Share.gt(0))), "involved")
            .into_model::<ExpenditureDisplay>()
    }

    pub async fn find_my_recent_expenditures(db: &DbConn, user_id: i32) -> Result<Vec<ExpenditureDisplay>, DbErr> {
        Self::annotate_expenditures(
            user_id,
            Expenditure::find()
            // TODO: spender == user or any (split where user == user and share != 0)
                .filter(
                    Cond::any()
                        .add(expenditure::Column::SpenderId.eq(user_id))
                        .add(split::Column::Id.is_not_null())
                )
                .order_by_desc(expenditure::Column::Date)
                .limit(10)
        )
            .all(db)
            .await
    }

    pub async fn find_all_expenditures(db: &DbConn, user_id: i32) -> Result<Vec<ExpenditureDisplay>, DbErr> {
        Self::annotate_expenditures(
            user_id,
            Expenditure::find()
                .order_by_desc(expenditure::Column::Date)
        )
            .all(db)
            .await
    }
}
