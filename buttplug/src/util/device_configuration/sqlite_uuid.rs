// Code idea taken from https://github.com/diesel-rs/diesel/issues/364, but updated for diesel v2.

use uuid;
use diesel::{
    deserialize::{self, FromSql, FromSqlRow},
    serialize::{self, IsNull, Output, ToSql},
    sql_types::{SqlType, Binary},
    sqlite::Sqlite,
    backend::Backend,
    expression::AsExpression
};
use std::fmt::{Display, Formatter};
use std::fmt;

#[derive(Debug, Clone, Copy, FromSqlRow, SqlType, AsExpression, Hash, Eq, PartialEq)]
#[diesel(sql_type = Binary)]
pub struct SqlUuid(pub uuid::Uuid);

impl SqlUuid {
    pub fn random() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl From<SqlUuid> for uuid::Uuid {
    fn from(s: SqlUuid) -> Self {
        s.0
    }
}

impl Display for SqlUuid {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromSql<Binary, Sqlite> for SqlUuid {
    fn from_sql(bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        // Changing this to a vec is definitely not optimal, it'd be nicer to work with the slice,
        // but I can't get that to work right now and don't really care.
        let value = <Vec<u8> as deserialize::FromSql<Binary, Sqlite>>::from_sql(bytes)?;
        uuid::Uuid::from_slice(&value).map(SqlUuid).map_err(|e| e.into())
    }
}

impl ToSql<Binary, Sqlite> for SqlUuid {
    fn to_sql(&self, out: &mut Output<Sqlite>) -> serialize::Result {
        out.set_value(self.0.as_bytes().to_vec());
        Ok(IsNull::No)
    }
}

