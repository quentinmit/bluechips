use crate::entities::{prelude::*, *};
use sea_orm::{prelude::*, *};
use sea_orm::sea_query::{Cond, SimpleExpr, IntoCondition};

#[derive(FromQueryResult)]
pub struct ExpenditureDisplay {
    pub id: i32,
    pub amount: Currency,
    pub mine: bool,
    pub involved: bool,
    pub description: Option<String>,
    pub date: Option<Date>,
    pub spender_name: Option<String>,
    pub share_amount: Currency,
}

#[derive(FromQueryResult)]
pub struct TransferDisplay {
    pub id: i32,
    pub amount: Currency,
    pub involved: bool,
    pub description: Option<String>,
    pub date: Option<Date>,
    pub debtor_name: Option<String>,
    pub creditor_name: Option<String>,
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
            .column_as::<SimpleExpr, _>(
                Expr::case(
                    user::Column::Id.eq(user_id),
                    None::<String>,
                ).finally(Expr::col(user::Column::Name)).into(),
                "spender_name")
            .column_as(user::Column::Id.eq(user_id), "mine")
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

    fn annotate_transfers(user_id: i32, select: Select<transfer::Entity>) -> Selector<SelectModel<TransferDisplay>> {
        #[derive(DeriveIden)]
        struct Debtor;
        #[derive(DeriveIden)]
        struct Creditor;
        select
            .select_only()
            .columns([
                transfer::Column::Id,
                transfer::Column::Amount,
                transfer::Column::Description,
                transfer::Column::Date,
            ])
            .join_as(JoinType::InnerJoin, transfer::Relation::Debtor.def(), Debtor)
            .join_as(JoinType::InnerJoin, transfer::Relation::Creditor.def(), Creditor)
            .column_as::<SimpleExpr, _>(
                Expr::case(
                    Expr::col((Debtor, user::Column::Id)).eq(user_id),
                    None::<String>,
                ).finally(Expr::col((Debtor, user::Column::Name))).into(),
                "debtor_name")
            .column_as::<SimpleExpr, _>(
                Expr::case(
                    Expr::col((Creditor, user::Column::Id)).eq(user_id),
                    None::<String>,
                ).finally(Expr::col((Creditor, user::Column::Name))).into(),
                "creditor_name")
            .column_as(transfer::Column::DebtorId.eq(user_id).or(transfer::Column::CreditorId.eq(user_id)), "involved")
            .into_model()
    }

    pub async fn find_my_recent_transfers(db: &DbConn, user_id: i32) -> Result<Vec<TransferDisplay>, DbErr> {
        Self::annotate_transfers(
            user_id,
            Transfer::find()
                .filter(
                    Cond::any()
                        .add(transfer::Column::DebtorId.eq(user_id))
                        .add(transfer::Column::CreditorId.eq(user_id))
                )
                .order_by_desc(transfer::Column::Date)
                .limit(10)
        )
            .all(db)
            .await
    }

    pub async fn find_all_transfers(db: &DbConn, user_id: i32) -> Result<Vec<TransferDisplay>, DbErr> {
        Self::annotate_transfers(
            user_id,
            Transfer::find()
                .order_by_desc(transfer::Column::Date)
        )
            .all(db)
            .await
    }

    pub async fn find_user_by_username(db: &DbConn, username: &str) -> Result<Option<user::Model>, DbErr> {
        User::find()
            .filter(user::Column::Username.eq(username))
            .one(db)
            .await
    }

    pub async fn get_user_by_id(db: &DbConn, id: i32) -> Result<Option<user::Model>, DbErr> {
        User::find_by_id(id)
            .one(db)
            .await
    }
}
