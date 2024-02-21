use std::collections::HashMap;
use std::ops::{RangeBounds, Bound};

use crate::entities::{prelude::*, *};
use sea_orm::{prelude::*, *};
use sea_orm::sea_query::{Cond, SimpleExpr, IntoCondition, ConditionType, TableRef, IntoIden, SelectStatement};
use chrono::{Local, NaiveDate, Datelike, Duration, Months};

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

#[derive(Debug)]
pub enum SettleError {
    CollectiveDebt(Vec<(i32, Currency)>),
    CollectiveCredit(Vec<(i32, Currency)>),
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

    pub async fn get_one_expenditure(db: &DbConn, id: i32, user_id: i32) -> Result<Option<ExpenditureDisplay>, DbErr> {
        Self::annotate_expenditures(
            user_id,
            Expenditure::find_by_id(id)
        )
            .one(db)
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

    pub async fn get_one_transfer(db: &DbConn, id: i32, user_id: i32) -> Result<Option<TransferDisplay>, DbErr> {
        Self::annotate_transfers(
            user_id,
            Transfer::find_by_id(id)
        )
            .one(db)
            .await
    }

    pub async fn find_users(db: &DbConn) -> Result<Vec<user::Model>, DbErr> {
        User::find()
            .order_by_asc(user::Column::Id)
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

    pub async fn get_debts(db: &DbConn) -> Result<HashMap<i32, Currency>, DbErr> {
        #[derive(DeriveIden)]
        struct TotalSpend;
        #[derive(DeriveIden)]
        struct TotalSplit;
        #[derive(DeriveIden)]
        struct TotalDebits;
        #[derive(DeriveIden)]
        struct TotalCredits;
        let total_spend = Expenditure::find()
            .select_only()
            .column(expenditure::Column::SpenderId)
            .column_as(expenditure::Column::Amount.sum(), "total")
            .group_by(expenditure::Column::SpenderId);
        let total_split = Split::find()
            .select_only()
            .column(split::Column::UserId)
            .column_as(split::Column::Share.sum(), "total")
            .group_by(split::Column::UserId);
        let transfer_query = Transfer::find()
            .select_only()
            .column_as(transfer::Column::Amount.sum(), "total");
        let total_debits = transfer_query.clone()
            .column(transfer::Column::DebtorId)
            .group_by(transfer::Column::DebtorId);
        let total_credits = transfer_query.clone()
            .column(transfer::Column::CreditorId)
            .group_by(transfer::Column::CreditorId);
        let query = User::find()
            .select_only()
            .column(user::Column::Id)
            .join(
                JoinType::LeftJoin,
                expenditure::Relation::Spender.def().rev().to_subquery_one(total_spend.into_query(), TotalSpend),
            )
            .join(
                JoinType::LeftJoin,
                split::Relation::User.def().rev().to_subquery_one(total_split.into_query(), TotalSplit),
            )
            .join(
                JoinType::LeftJoin,
                transfer::Relation::Debtor.def().rev().to_subquery_one(total_debits.into_query(), TotalDebits),
            )
            .join(
                JoinType::LeftJoin,
                transfer::Relation::Creditor.def().rev().to_subquery_one(total_credits.into_query(), TotalCredits)
            );
        const ZERO: i32 = 0;
        trace!("debts = {:?}",
            query
                .clone()
                .exprs([
                    Expr::col((TotalSplit, "total".into_identity())).if_null(ZERO),
                    Expr::col((TotalCredits, "total".into_identity())).if_null(ZERO),
                    Expr::col((TotalSpend, "total".into_identity())).if_null(ZERO),
                    Expr::col((TotalDebits, "total".into_identity())).if_null(ZERO),
                ])
                .into_tuple::<(i32, Currency, Currency, Currency, Currency)>()
                .all(db)
                .await
                .map(|v| v.into_iter().map(|(id, a, b, c, d)| (id, a.to_string(), b.to_string(), c.to_string(), d.to_string())).collect::<Vec<_>>())
        );
        query
            .column_as(
                Expr::col((TotalSplit, "total".into_identity())).if_null(ZERO)
                .add(Expr::col((TotalCredits, "total".into_identity())).if_null(ZERO))
                .sub(Expr::col((TotalSpend, "total".into_identity())).if_null(ZERO))
                .sub(Expr::col((TotalDebits, "total".into_identity())).if_null(ZERO)),
                "amount"
            )
            .into_tuple()
            .all(db)
            .await
            .map(|v: Vec<(i32, Currency)>| v.into_iter().collect())
    }

    pub fn settle(debts: HashMap<i32, Currency>) -> Result<Vec<(i32, i32, Currency)>, SettleError> {
        // This algorithm has been shamelessly stolen from Nelson Elhage's
        // <nelhage@mit.edu> implementation for our 2008 summer apartment.
        let (mut owes_list, mut owed_list): (Vec<_>, Vec<_>) = debts.into_iter().filter(|(_, v)| *v != 0.into()).partition(|(_, v)| *v > 0.into());

        let mut settle_list: Vec<(i32, i32, Currency)> = Vec::new();

        while owes_list.len() > 0 && owed_list.len() > 0 {
            owes_list.sort_by_key(|(_, v)| v.clone());
            owed_list.sort_by_key(|(_, v)| v.clone());

            let owes = owes_list.pop().unwrap();
            let owed = owed_list.pop().unwrap();

            let sum = owes.1.clone() + owed.1.clone();

            let val = if sum.is_zero() {
                owes.1
            } else if sum.is_positive() {
                owes_list.push((owes.0, owes.1 + owed.1.clone()));
                -owed.1
            } else {
                owed_list.push((owed.0, owed.1 + owes.1.clone()));
                owes.1
            };

            settle_list.push((owes.0, owed.0, val));
        }

        if owes_list.len() > 0 {
            Err(SettleError::CollectiveDebt(owes_list))
        } else if owed_list.len() > 0 {
            Err(SettleError::CollectiveCredit(owed_list))
        } else {
            Ok(settle_list)
        }
    }

    async fn get_totals_for_date_range(db: &DbConn, user_id: i32, range: impl RangeBounds<NaiveDate>) -> Result<(Currency, Currency), DbErr> {
        let query = Expenditure::find()
            .select_only();
        let query = match range.start_bound() {
            Bound::Included(d) => query.filter(expenditure::Column::Date.gte(*d)),
            Bound::Excluded(d) => query.filter(expenditure::Column::Date.gt(*d)),
            Bound::Unbounded => query,
        };
        let query = match range.end_bound() {
            Bound::Included(d) => query.filter(expenditure::Column::Date.lte(*d)),
            Bound::Excluded(d) => query.filter(expenditure::Column::Date.lt(*d)),
            Bound::Unbounded => query,
        };
        query
            .join(JoinType::LeftJoin, expenditure::Relation::Split.def().on_condition(move |_, right| {Expr::col((right, split::Column::UserId)).eq(user_id).into_condition()}))
            .column_as(Expr::expr(expenditure::Column::Amount.sum()).if_null(0), "total")
            .column_as(Expr::expr(split::Column::Share.sum()).if_null(0), "mine")
            .into_tuple::<(Currency, Currency)>()
            .one(db)
            .await
            .map(|v| v.unwrap_or((0.into(), 0.into())))
    }

    pub async fn get_totals(db: &DbConn, user_id: i32) -> Result<Totals, DbErr> {
        let today = Local::now().date_naive();
        let first_of_month = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
        Ok(Totals {
            total: Self::get_totals_for_date_range(db, user_id, ..).await?,
            past_year: Self::get_totals_for_date_range(db, user_id, today-Duration::days(365)..).await?,
            year_to_date: Self::get_totals_for_date_range(db, user_id, NaiveDate::from_yo_opt(today.year(), 1).unwrap()..).await?,
            month_to_date: Self::get_totals_for_date_range(db, user_id, first_of_month..).await?,
            last_month: Self::get_totals_for_date_range(db, user_id, first_of_month-Months::new(1)..first_of_month).await?,
        })
    }
}

pub struct Totals {
    pub total: (Currency, Currency),
    pub past_year: (Currency, Currency),
    pub year_to_date: (Currency, Currency),
    pub month_to_date: (Currency, Currency),
    pub last_month: (Currency, Currency),
}

trait RelationDefExt {
    fn to_subquery_one(self, sq: SelectStatement, iden: impl IntoIden) -> Self;
}
impl RelationDefExt for RelationDef {
    fn to_subquery_one(mut self, sq: SelectStatement, iden: impl IntoIden) -> Self {
        self.rel_type = RelationType::HasOne;
        self.to_tbl = TableRef::SubQuery(sq, iden.into_iden());
        self.on_delete = None;
        self.on_update = None;
        self.fk_name = None;
        self.condition_type = ConditionType::All;
        self
    }
}
