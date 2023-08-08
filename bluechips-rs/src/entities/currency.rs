use core::fmt;

use derive_more::{Add, Sub, Mul, Div};
use sea_orm::entity::prelude::*;
use rusty_money::{Money, MoneyError, Round, FormattableCurrency, iso};

#[derive(Clone, Debug, PartialEq, Eq, Add, Sub, Mul, Div, PartialOrd, Ord)]
pub struct Currency(Money<'static, iso::Currency>);

impl Currency {
    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
    pub fn is_positive(&self) -> bool {
        self.0.is_positive()
    }
    pub fn is_negative(&self) -> bool {
        self.0.is_negative()
    }
    pub fn abs(&self) -> Self {
        if self.is_negative() {
            -self.clone()
        } else {
            self.clone()
        }
    }
}

impl core::ops::Neg for Currency {
    type Output = Currency;
    fn neg(self) -> Self::Output {
        self * -1
    }
}

impl std::iter::Sum for Currency {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(0.into(), |a, b| a + b)
    }
}

impl From<Currency> for Value {
    fn from(source: Currency) -> Self {
        let rounded = source.0.round(source.0.currency().exponent(), Round::HalfEven);
        assert_eq!(rounded.amount().scale(), source.0.currency().exponent());
        (rounded.amount().mantissa() as i32).into()
    }
}

impl From<i32> for Currency {
    fn from(source: i32) -> Self {
        Currency(Money::from_minor(source as i64, iso::USD))
    }
}

impl TryFrom<&str> for Currency {
    type Error = MoneyError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Money::from_str(value, iso::USD).map(|m| Self(m))
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn from_integer() {
        let c = Currency::from(123456);
        assert_eq!("$1,234.56", format!("{}", c));
    }

    #[test]
    fn from_str() {
        let c = Currency::try_from("1234.56").unwrap();
        assert_eq!("$1,234.56", format!("{}", c));
    }

    #[test]
    fn add() {
        let c1 = Currency::from(123);
        let c2 = Currency::from(456);
        let c = c1 + c2;
        assert_eq!("$5.79", format!("{}", c));
    }
}
