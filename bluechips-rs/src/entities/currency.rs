use sea_orm::entity::prelude::*;
use rusty_money::{Money, Round, FormattableCurrency, iso};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Currency(Money<'static, iso::Currency>);

impl From<Currency> for Value {
    fn from(source: Currency) -> Self {
        (source.0.round(source.0.currency().exponent(), Round::HalfEven).amount().mantissa() as i32).into()
    }
}

impl From<i32> for Currency {
    fn from(source: i32) -> Self {
        Currency(Money::from_minor(source as i64, iso::USD))
    }
}

impl sea_orm::TryGetable for Currency {
    fn try_get_by<I: sea_orm::ColIdx>(res: &QueryResult, idx: I) -> Result<Self, sea_orm::TryGetError> {
        <i32 as sea_orm::TryGetable>::try_get_by(res, idx).map(|v| v.into())
    }
}

impl sea_orm::sea_query::ValueType for Currency {
    fn try_from(v: Value) -> Result<Self, sea_orm::sea_query::ValueTypeErr> {
        <i32 as sea_orm::sea_query::ValueType>::try_from(v).map(|v| v.into())
    }

    fn type_name() -> String {
        stringify!(Currency).to_owned()
    }

    fn array_type() -> sea_orm::sea_query::ArrayType {
        sea_orm::sea_query::ArrayType::Int
    }

    fn column_type() -> sea_orm::sea_query::ColumnType {
        sea_orm::sea_query::ColumnType::Integer
    }
}
