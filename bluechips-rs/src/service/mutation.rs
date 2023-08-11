use std::collections::HashMap;
use std::ops::{RangeBounds, Bound};

use crate::entities::{prelude::*, *};
use sea_orm::{prelude::*, *};
use sea_orm::ActiveValue::{Set, NotSet, Unchanged};

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct DateField(pub chrono::NaiveDate);
#[rocket::async_trait]
impl<'v> rocket::form::FromFormField<'v> for DateField {
    fn from_value(field: rocket::form::ValueField<'v>) -> rocket::form::Result<'v, Self> {
        chrono::NaiveDate::parse_from_str(field.value, "%m/%d/%Y").map(|v| Self(v)).map_err(|e| rocket::form::Error::validation(format!("failed to parse date: {:?}", e)).into())
    }
}
#[derive(FromForm, Clone, PartialEq, Eq)]
pub struct ExpenditureForm {
    pub spender_id: i32,
    pub amount: Currency,
    pub description: String,
    pub date: DateField,
    pub splits: HashMap<i32, Currency>,
}

pub struct Mutation;

impl Mutation {
    pub async fn create_expenditure(db: &DbConn, form_data: ExpenditureForm) -> Result<expenditure::Model, TransactionError<DbErr>> {
        db.transaction::<_, expenditure::Model, DbErr>(|txn| {
            Box::pin(async move {
                let expenditure = expenditure::ActiveModel {
                    spender_id: Set(form_data.spender_id),
                    amount: Set(form_data.amount),
                    description: Set(Some(form_data.description)),
                    date: Set(Some(form_data.date.0)),
                    ..Default::default()
                }
                    .insert(txn)
                    .await?;
                Split::insert_many(form_data.splits.into_iter().filter(|(_, amount)| !amount.is_zero()).map(|(user_id, amount)| split::ActiveModel {
                    id: NotSet,
                    expenditure_id: Set(expenditure.id),
                    user_id: Set(user_id),
                    share: Set(amount),
                }))
                    .exec(txn)
                    .await?;
                Ok(expenditure)
            })
        })
        .await
    }

    pub async fn update_expenditure(db: &DbConn, id: i32, form_data: ExpenditureForm) -> Result<expenditure::Model, TransactionError<DbErr>> {
        db.transaction::<_, expenditure::Model, DbErr>(|txn| {
            Box::pin(async move {
                let expenditure = expenditure::ActiveModel {
                    id: Unchanged(id),
                    spender_id: Set(form_data.spender_id),
                    amount: Set(form_data.amount),
                    description: Set(Some(form_data.description)),
                    date: Set(Some(form_data.date.0)),
                    ..Default::default()
                }
                    .update(txn)
                    .await?;
                Split::delete_many()
                    .filter(split::Column::ExpenditureId.eq(id))
                    .exec(txn)
                    .await?;
                Split::insert_many(form_data.splits.into_iter().map(|(user_id, amount)| split::ActiveModel {
                    id: NotSet,
                    expenditure_id: Set(expenditure.id),
                    user_id: Set(user_id),
                    share: Set(amount),
                }))
                .exec(txn)
                .await?;
                Ok(expenditure)
            })
        })
        .await
    }
}