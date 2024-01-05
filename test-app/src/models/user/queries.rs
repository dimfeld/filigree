use std::str::FromStr;

use error_stack::ResultExt;
use filigree::{
    errors::OrderByError,
    sql::{BindingOperator, FilterBuilder},
};
use serde::Deserialize;
use sqlx::{query_file, query_file_as, PgExecutor, PgPool};

use super::{types::*, UserId};
use crate::{auth::AuthInfo, models::organization::OrganizationId, Error};

type QueryAs<'q, T> = sqlx::query::QueryAs<
    'q,
    sqlx::Postgres,
    T,
    <sqlx::Postgres as sqlx::database::HasArguments<'q>>::Arguments,
>;

/// Get a User from the database
pub async fn get(
    db: impl PgExecutor<'_>,
    auth: &AuthInfo,
    id: UserId,
) -> Result<User, error_stack::Report<Error>> {
    let actor_ids = auth.actor_ids();
    let object = query_file_as!(
        User,
        "src/models/user/select_one.sql",
        id.as_uuid(),
        auth.organization_id.as_uuid(),
        &actor_ids
    )
    .fetch_optional(db)
    .await
    .change_context(Error::Db)?
    .ok_or(Error::NotFound("User"))?;

    Ok(object)
}

#[derive(Debug, Default)]
enum OrderByField {
    UpdatedAt,
    CreatedAt,
    #[default]
    Name,
}

impl OrderByField {
    fn as_str(&self) -> &str {
        match self {
            Self::UpdatedAt => "updated_at",
            Self::CreatedAt => "created_at",
            Self::Name => "name",
        }
    }

    fn allowed_direction(&self, descending: bool) -> bool {
        match self {
            _ => true,
        }
    }
}

impl std::str::FromStr for OrderByField {
    type Err = OrderByError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value = match s {
            "updated_at" => OrderByField::UpdatedAt,

            "created_at" => OrderByField::CreatedAt,

            "name" => OrderByField::Name,

            _ => return Err(OrderByError::InvalidField),
        };

        Ok(value)
    }
}

fn parse_order_by(field: &str) -> Result<(bool, OrderByField), OrderByError> {
    let descending = field.starts_with('-');
    let field = if descending { &field[1..] } else { field };

    let value = OrderByField::from_str(field)?;
    if !value.allowed_direction(descending) {
        return Err(OrderByError::InvalidDirection);
    }
    Ok((descending, value))
}

#[derive(Deserialize, Debug)]
pub struct ListQueryFilters {
    pub page: Option<u32>,
    pub per_page: Option<u32>,

    pub order_by: Option<String>,
    #[serde(default)]
    pub id: Vec<UserId>,
    pub updated_at_lte: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at_gte: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at_lte: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at_gte: Option<chrono::DateTime<chrono::Utc>>,
}

impl ListQueryFilters {
    fn build_where_clause(&self) -> String {
        let mut bindings = FilterBuilder::new(5);

        if !self.id.is_empty() {
            bindings.add_vec("id", &self.id);
        }

        if self.updated_at_lte.is_some() {
            bindings.add_option("updated_at", &self.updated_at_lte, BindingOperator::Lte);
        }

        if self.updated_at_gte.is_some() {
            bindings.add_option("updated_at", &self.updated_at_gte, BindingOperator::Gte);
        }

        if self.created_at_lte.is_some() {
            bindings.add_option("created_at", &self.created_at_lte, BindingOperator::Lte);
        }

        if self.created_at_gte.is_some() {
            bindings.add_option("created_at", &self.created_at_gte, BindingOperator::Gte);
        }

        bindings.to_string()
    }

    fn bind_to_query<'a, T>(&'a self, mut query: QueryAs<'a, T>) -> QueryAs<'a, T> {
        const MAX_PER_PAGE: u32 = 200;
        const DEFAULT_PER_PAGE: u32 = 50;
        let per_page = self
            .per_page
            .unwrap_or(DEFAULT_PER_PAGE)
            .min(MAX_PER_PAGE)
            .max(1);
        let offset = self.page.unwrap_or(0) * per_page;
        query = query.bind(per_page as i32).bind(offset as i32);

        if !self.id.is_empty() {
            query = query.bind(&self.id);
        }

        if self.updated_at_lte.is_some() {
            query = query.bind(&self.updated_at_lte);
        }

        if self.updated_at_gte.is_some() {
            query = query.bind(&self.updated_at_gte);
        }

        if self.created_at_lte.is_some() {
            query = query.bind(&self.created_at_lte);
        }

        if self.created_at_gte.is_some() {
            query = query.bind(&self.created_at_gte);
        }

        query
    }
}

pub async fn list(
    db: impl PgExecutor<'_>,
    auth: &AuthInfo,
    filters: &ListQueryFilters,
) -> Result<Vec<User>, error_stack::Report<Error>> {
    let q = include_str!("list.sql");

    let (descending, order_by_field) =
        parse_order_by(filters.order_by.as_deref().unwrap_or("name"))
            .change_context(Error::Filter)?;
    let order_direction = if descending { "DESC" } else { "ASC" };

    let q = q.replace(
        "<order_by>",
        &format!("{} {}", order_by_field.as_str(), order_direction),
    );

    let q = q.replace("<filters>", &filters.build_where_clause());

    let mut query = sqlx::query_as::<_, User>(q.as_str());

    let actor_ids = auth.actor_ids();
    query = query.bind(&auth.organization_id).bind(&actor_ids);

    query = filters.bind_to_query(query);

    let results = query.fetch_all(db).await.change_context(Error::Db)?;

    Ok(results)
}

pub async fn create(
    db: impl PgExecutor<'_>,
    auth: &AuthInfo,
    payload: &UserCreatePayload,
) -> Result<User, error_stack::Report<Error>> {
    // TODO create permissions auth check
    let id = UserId::new();
    create_raw(db, id, auth.organization_id, payload).await
}

/// Create a new User in the database, allowing the ID to be explicitly specified.
pub async fn create_raw(
    db: impl PgExecutor<'_>,
    id: UserId,
    organization_id: OrganizationId,
    payload: &UserCreatePayload,
) -> Result<User, error_stack::Report<Error>> {
    let result = query_file_as!(
        User,
        "src/models/user/insert.sql",
        id.as_uuid(),
        organization_id.as_uuid(),
        &payload.name,
        &payload.email,
        &payload.verified,
    )
    .fetch_one(db)
    .await
    .change_context(Error::Db)?;

    Ok(result)
}

pub async fn update(
    db: impl PgExecutor<'_>,
    auth: &AuthInfo,
    id: UserId,
    payload: &UserUpdatePayload,
) -> Result<(), error_stack::Report<Error>> {
    let actor_ids = auth.actor_ids();
    query_file!(
        "src/models/user/update.sql",
        id.as_uuid(),
        auth.organization_id.as_uuid(),
        &actor_ids,
        &payload.name as _,
        &payload.email as _,
        &payload.verified as _,
    )
    .execute(db)
    .await
    .change_context(Error::Db)?;
    Ok(())
}

pub async fn delete(
    db: impl PgExecutor<'_>,
    auth: &AuthInfo,
    id: UserId,
) -> Result<(), error_stack::Report<Error>> {
    let actor_ids = auth.actor_ids();
    query_file!(
        "src/models/user/delete.sql",
        id.as_uuid(),
        auth.organization_id.as_uuid(),
        &actor_ids
    )
    .execute(db)
    .await
    .change_context(Error::Db)?;
    Ok(())
}
