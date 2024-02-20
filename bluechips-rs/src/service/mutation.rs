use std::collections::{HashSet, HashMap};

use crate::entities::{prelude::*, *};
use sea_orm::{prelude::*, *};
use sea_orm::ActiveValue::{Set, NotSet, Unchanged};
use rand::seq::IteratorRandom;

fn nonzero_splits<'v>(splits: &HashMap<i32, Currency>) -> rocket::form::Result<'v, ()> {
    if splits.values().sum::<Currency>().is_zero() {
        Err(rocket::form::Error::validation("splits cannot sum to zero"))?;
    }
    Ok(())
}


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
    #[field(validate=nonzero_splits())]
    pub splits: HashMap<i32, Currency>,
}

#[derive(FromForm, Clone, PartialEq, Eq)]
pub struct TransferForm {
    pub debtor_id: i32,
    pub creditor_id: i32,
    pub amount: Currency,
    pub description: String,
    pub date: DateField,
}

pub struct Mutation;

impl Mutation {
    /// Split up an expenditure.
    ///
    /// split_dict should be a dict mapping from user IDs
    /// to a `Currency` object representing the percentage
    /// that user is responsible for.
    ///
    /// Percentages will be normalized to sum to 100%.
    ///
    /// If the split leaks or gains money due to rounding errors, the
    /// pennies will be randomly distributed to a subset of the users.
    ///
    /// I mean, come on. You're already living together. Are you really
    /// going to squabble over a few pennies?
    async fn set_splits<'a, C: ConnectionTrait>(db: &'a C, expenditure_id: i32, amount: Currency, splits: HashMap<i32, Currency>) -> Result<(), DbErr> {
        // Remove any old splits.
        Split::delete_many()
            .filter(split::Column::ExpenditureId.eq(expenditure_id))
            .exec(db)
            .await?;
        let splits: HashMap<i32, i32> = splits
            .into_iter()
            .filter(|(_, share)| !share.is_zero())
            .map(|(user_id, share)| (user_id, i32::from(share)))
            .collect();
        let splits_total: i32 = splits.values().sum();
        trace!("amount = {}, splits_total = {}", &amount, splits_total);
        let splits: HashMap<_, _> = splits
            .into_iter()
            .map(|(user_id, share)| (user_id, amount.clone() * share / splits_total))
            .collect();
        // splits now represents the portion of the amount that each user owes, but it might not add up to the total amount.
        let difference = amount.clone() - splits.values().sum();
        let winners: HashSet<i32> = splits.keys().choose_multiple(&mut rand::thread_rng(), i32::from(difference.abs()) as usize).into_iter().map(|user_id| *user_id).collect();
        let splits: HashMap<_, _> = splits
            .into_iter()
            .map(|(user_id, share)| (user_id, share + Currency::from(if winners.contains(&user_id) { if difference.is_positive() { 1 } else { -1 } } else { 0 })))
            .collect();
        assert_eq!(amount.clone(), splits.values().sum());
        Split::insert_many(
            splits.into_iter().map(|(user_id, amount)| split::ActiveModel {
            id: NotSet,
            expenditure_id: Set(expenditure_id),
            user_id: Set(user_id),
            share: Set(amount),
        }))
            .exec(db)
            .await?;
        Ok(())
    }
    pub async fn save_expenditure(db: &DbConn, id: Option<i32>, form_data: ExpenditureForm) -> Result<expenditure::Model, TransactionError<DbErr>> {
        db.transaction::<_, expenditure::Model, DbErr>(|txn| {
            Box::pin(async move {
                let expenditure = expenditure::ActiveModel {
                    id: match id {
                        Some(id) => Unchanged(id),
                        None => NotSet,
                    },
                    spender_id: Set(form_data.spender_id),
                    amount: Set(form_data.amount.clone()),
                    description: Set(Some(form_data.description)),
                    date: Set(Some(form_data.date.0)),
                    ..Default::default()
                };
                let expenditure = match id {
                    Some(_) => expenditure.update(txn),
                    None => expenditure.insert(txn),
                }
                    .await?;
                Self::set_splits(txn, expenditure.id, form_data.amount.clone(), form_data.splits).await?;
                Ok(expenditure)
            })
        })
        .await
    }
    pub async fn save_transfer(db: &DbConn, id: Option<i32>, form_data: TransferForm) -> Result<transfer::Model, DbErr> {
        let mut model = transfer::ActiveModel {
            debtor_id: Set(form_data.debtor_id),
            creditor_id: Set(form_data.creditor_id),
            amount: Set(form_data.amount.clone()),
            description: Set(Some(form_data.description)),
            date: Set(Some(form_data.date.0)),
            ..Default::default()
        };
        match id {
            Some(id) => {
               model.id = Unchanged(id);
               model.update(db)
            }
            None => model.insert(db),
        }
            .await
    }
    pub async fn ensure_user(db: &DbConn, mut user: user::ActiveModel) -> Result<user::Model, TransactionError<DbErr>> {
        db.transaction::<_, user::Model, DbErr>(|txn| {
            Box::pin(async move {
                let existing = User::find()
                    .filter(user::Column::Username.eq(user.username.as_ref()))
                    .one(txn)
                    .await?;
                match existing {
                    Some(existing) => {
                        user.id = Unchanged(existing.id);
                        user.update(txn)
                    }
                    None => user.insert(txn),
                }
                .await
            })
        })
        .await
    }
}
