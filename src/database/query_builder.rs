use sea_query::{Condition, Expr, Alias, Query, MysqlQueryBuilder, PostgresQueryBuilder};
// use async_trait::async_trait;
use sqlx::FromRow;
use crate::database::connection::{DatabaseConnection, OrmError, DatabaseType};
use crate::models::base_model::Model;

pub struct QueryBuilder<T: Model + for<'r> FromRow<'r, sqlx::postgres::PgRow>> {
    conditions: Vec<Condition>,
    limit: Option<u64>,
    offset: Option<u64>,
    order_by: Option<(String, bool)>,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Model + for<'r> FromRow<'r, sqlx::postgres::PgRow>> QueryBuilder<T> {
    pub fn new() -> Self {
        Self {
            conditions: Vec::new(),
            limit: None,
            offset: None,
            order_by: None,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn where_eq(mut self, column: &str, value: &str) -> Self {
        let column_alias = Alias::new(column);
        let condition = Condition::all().add(Expr::col(column_alias).eq(value));
        self.conditions.push(condition);
        self
    }
    
    pub fn limit(mut self, limit: u64) -> Self {
        self.limit = Some(limit);
        self
    }
    
    pub fn offset(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }
    
    pub fn order_by(mut self, column: &str, descending: bool) -> Self {
        self.order_by = Some((column.to_string(), descending));
        self
    }
    
    pub async fn execute(self, db: &DatabaseConnection) -> Result<Vec<T>, OrmError> {
        let table_name = T::table_name();
        let mut query = Query::select();

        query.from(Alias::new(&table_name));

        for condition in self.conditions {
            query.cond_where(condition);
        }

        if let Some((column, descending)) = self.order_by {
            query.order_by(Alias::new(&column), if descending { sea_query::Order::Desc } else { sea_query::Order::Asc });
        }

        if let Some(limit) = self.limit {
            query.limit(limit);
        }
        if let Some(offset) = self.offset {
            query.offset(offset);
        }

        let sql = match &db.connection {
            DatabaseType::Postgres(_) => query.to_string(PostgresQueryBuilder),
            DatabaseType::MySql(_) => query.to_string(MysqlQueryBuilder),
        };

        match &db.connection {
            DatabaseType::Postgres(pool) => {
                let rows = sqlx::query(&sql)
                    .fetch_all(pool)
                    .await
                    .map_err(|e| OrmError::QueryError(e.to_string()))?
                    .into_iter()
                    .map(|row| T::from_row(&row).expect("Failed to convert row"))
                    .collect();
                Ok(rows)
            },
            DatabaseType::MySql(_) => {
                Err(OrmError::QueryError("MySQL support not fully implemented".to_string()))
            },
        }
    }
}