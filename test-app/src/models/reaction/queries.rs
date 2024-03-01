#![allow(unused_imports, unused_variables, dead_code)]
use std::str::FromStr;

use error_stack::ResultExt;
use filigree::{
    auth::ObjectPermission,
    errors::OrderByError,
    sql::{BindingOperator, FilterBuilder, ValuesBuilder},
};
use serde::Deserialize;
use sqlx::{
    postgres::PgRow, query_file, query_file_as, query_file_scalar, PgConnection, PgExecutor,
};
use tracing::{event, instrument, Level};

use super::{types::*, ReactionId};
use crate::{
    auth::AuthInfo,
    models::{organization::OrganizationId, post::PostId},
    Error,
};

type QueryAs<'q, T> = sqlx::query::QueryAs<
    'q,
    sqlx::Postgres,
    T,
    <sqlx::Postgres as sqlx::database::HasArguments<'q>>::Arguments,
>;

/// Get a Reaction from the database
#[instrument(skip(db))]
pub async fn get(
    db: impl PgExecutor<'_>,
    auth: &AuthInfo,
    id: ReactionId,
) -> Result<Reaction, error_stack::Report<Error>> {
    let actor_ids = auth.actor_ids();
    let object = query_file_as!(
        Reaction,
        "src/models/reaction/select_one.sql",
        id.as_uuid(),
        auth.organization_id.as_uuid(),
        &actor_ids
    )
    .fetch_optional(db)
    .await
    .change_context(Error::Db)?
    .ok_or(Error::NotFound("Reaction"))?;

    Ok(object)
}

#[derive(Debug, Default)]
enum OrderByField {
    #[default]
    UpdatedAt,
    CreatedAt,
}

impl OrderByField {
    fn as_str(&self) -> &str {
        match self {
            Self::UpdatedAt => "updated_at",
            Self::CreatedAt => "created_at",
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
    pub id: Vec<ReactionId>,
    #[serde(default)]
    pub post_id: Vec<PostId>,
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

        if !self.post_id.is_empty() {
            bindings.add_vec("post_id", &self.post_id);
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

        let query = bindings.to_string();
        event!(Level::DEBUG, %query);
        query
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
        event!(Level::DEBUG, %per_page, %offset);
        query = query.bind(per_page as i32).bind(offset as i32);

        if !self.id.is_empty() {
            event!(Level::DEBUG, id = ?self.id);
            query = query.bind(&self.id);
        }

        if !self.post_id.is_empty() {
            event!(Level::DEBUG, post_id = ?self.post_id);
            query = query.bind(&self.post_id);
        }

        if self.updated_at_lte.is_some() {
            event!(Level::DEBUG, updated_at_lte = ?self.updated_at_lte);
            query = query.bind(&self.updated_at_lte);
        }

        if self.updated_at_gte.is_some() {
            event!(Level::DEBUG, updated_at_gte = ?self.updated_at_gte);
            query = query.bind(&self.updated_at_gte);
        }

        if self.created_at_lte.is_some() {
            event!(Level::DEBUG, created_at_lte = ?self.created_at_lte);
            query = query.bind(&self.created_at_lte);
        }

        if self.created_at_gte.is_some() {
            event!(Level::DEBUG, created_at_gte = ?self.created_at_gte);
            query = query.bind(&self.created_at_gte);
        }

        query
    }
}

#[instrument(skip(db))]
pub async fn list(
    db: impl PgExecutor<'_>,
    auth: &AuthInfo,
    filters: &ListQueryFilters,
) -> Result<Vec<Reaction>, error_stack::Report<Error>> {
    let q = include_str!("list.sql");
    list_internal(q, db, auth, filters).await
}

async fn list_internal<T>(
    query_template: &str,
    db: impl PgExecutor<'_>,
    auth: &AuthInfo,
    filters: &ListQueryFilters,
) -> Result<Vec<T>, error_stack::Report<Error>>
where
    T: for<'r> sqlx::FromRow<'r, PgRow> + Send + Unpin,
{
    let (descending, order_by_field) =
        parse_order_by(filters.order_by.as_deref().unwrap_or("-updated_at"))
            .change_context(Error::Filter)?;
    let order_direction = if descending { "DESC" } else { "ASC" };

    let q = query_template.replace(
        "__insertion_point_order_by",
        &format!("{} {}", order_by_field.as_str(), order_direction),
    );

    let q = q.replace("__insertion_point_filters", &filters.build_where_clause());

    let mut query = sqlx::query_as::<_, T>(q.as_str());

    let actor_ids = auth.actor_ids();
    event!(Level::DEBUG, organization_id=?auth.organization_id, actor_ids=?actor_ids);
    query = query.bind(&auth.organization_id).bind(&actor_ids);

    query = filters.bind_to_query(query);

    let results = query.fetch_all(db).await.change_context(Error::Db)?;

    Ok(results)
}

/// Create a new Reaction in the database.
pub async fn create(
    db: &mut PgConnection,
    auth: &AuthInfo,
    payload: ReactionCreatePayload,
) -> Result<ReactionCreateResult, error_stack::Report<Error>> {
    // TODO create permissions auth check
    let id = ReactionId::new();
    create_raw(&mut *db, id, auth.organization_id, payload).await
}

/// Create a new Reaction in the database, allowing the ID to be explicitly specified.
#[instrument(skip(db))]
pub async fn create_raw(
    db: &mut PgConnection,
    id: ReactionId,
    organization_id: OrganizationId,
    payload: ReactionCreatePayload,
) -> Result<ReactionCreateResult, error_stack::Report<Error>> {
    let result = query_file_as!(
        Reaction,
        "src/models/reaction/insert.sql",
        id.as_uuid(),
        organization_id.as_uuid(),
        &payload.typ,
        &payload.post_id as _,
    )
    .fetch_one(&mut *db)
    .await
    .change_context(Error::Db)?;

    Ok(result)
}

#[instrument(skip(db))]
pub async fn update(
    db: &mut PgConnection,
    auth: &AuthInfo,
    id: ReactionId,
    payload: ReactionUpdatePayload,
) -> Result<bool, error_stack::Report<Error>> {
    let actor_ids = auth.actor_ids();
    let result = query_file_scalar!(
        "src/models/reaction/update.sql",
        id.as_uuid(),
        auth.organization_id.as_uuid(),
        &actor_ids,
        &payload.typ as _,
        &payload.post_id as _,
    )
    .fetch_optional(&mut *db)
    .await
    .change_context(Error::Db)?;

    let Some(is_owner) = result else {
        return Ok(false);
    };

    Ok(true)
}

#[instrument(skip(db))]
pub async fn delete(
    db: impl PgExecutor<'_>,
    auth: &AuthInfo,
    id: ReactionId,
) -> Result<bool, error_stack::Report<Error>> {
    let actor_ids = auth.actor_ids();
    let result = query_file!(
        "src/models/reaction/delete.sql",
        id.as_uuid(),
        auth.organization_id.as_uuid(),
        &actor_ids
    )
    .execute(db)
    .await
    .change_context(Error::Db)?;
    Ok(result.rows_affected() > 0)
}

#[instrument(skip(db))]
pub async fn lookup_object_permissions(
    db: impl PgExecutor<'_>,
    auth: &AuthInfo,
    #[allow(unused_variables)] id: ReactionId,
) -> Result<Option<ObjectPermission>, error_stack::Report<Error>> {
    let actor_ids = auth.actor_ids();
    let result = query_file_scalar!(
        "src/models/reaction/lookup_object_permissions.sql",
        auth.organization_id.as_uuid(),
        &actor_ids,
    )
    .fetch_one(db)
    .await
    .change_context(Error::Db)?;

    let perm = result.and_then(|r| ObjectPermission::from_str_infallible(&r));
    Ok(perm)
}

#[instrument(skip(db))]
pub async fn update_one_with_parent(
    db: impl PgExecutor<'_>,
    auth: &AuthInfo,
    is_owner: bool,
    parent_id: PostId,
    id: ReactionId,
    mut payload: ReactionUpdatePayload,
) -> Result<bool, error_stack::Report<Error>> {
    payload.post_id = parent_id;

    let actor_ids = auth.actor_ids();
    let result = query_file!(
        "src/models/reaction/update_one_with_parent.sql",
        id.as_uuid(),
        parent_id.as_uuid(),
        auth.organization_id.as_uuid(),
        &actor_ids,
        &payload.typ as _,
    )
    .execute(db)
    .await
    .change_context(Error::Db)?;

    Ok(result.rows_affected() > 0)
}

/// Update the children of the given parent.
#[instrument(skip(db))]
pub async fn update_with_parent(
    db: &mut PgConnection,
    organization_id: OrganizationId,
    is_owner: bool,
    parent_id: PostId,
    payload: &[ReactionUpdatePayload],
) -> Result<Vec<Reaction>, error_stack::Report<Error>> {
    if payload.is_empty() {
        delete_all_children_of_parent(db, organization_id, parent_id).await?;
        Ok(Vec::new())
    } else {
        // First, we upsert the existing children.
        let q = include_str!("upsert_children.sql");
        let bindings = ValuesBuilder {
            first_parameter: 4,
            num_values: payload.len(),
            num_columns: 2 + 2,
        };
        let q = q.replace("__insertion_point_insert_values", &bindings.to_string());

        let mut query = sqlx::query_as::<_, Reaction>(q.as_str());

        query = query
            .bind(is_owner)
            .bind(organization_id.as_uuid())
            .bind(parent_id.as_uuid());

        for p in payload {
            let id = p.id.unwrap_or_else(|| ReactionId::new());
            query = query
                .bind(id)
                .bind(organization_id)
                .bind(&p.typ)
                .bind(&p.post_id)
        }

        let results = query.fetch_all(&mut *db).await.change_context(Error::Db)?;

        // Delete any of the children that were not sent in.
        let ids = results
            .iter()
            .map(|o| o.id.as_uuid().clone())
            .collect::<Vec<_>>();
        query_file!(
            "src/models/reaction/delete_removed_children.sql",
            organization_id.as_uuid(),
            parent_id.as_uuid(),
            &ids
        )
        .execute(db)
        .await
        .change_context(Error::Db)?;

        Ok(results)
    }
}

/// Delete a child object, making sure that its parent ID matches.
#[instrument(skip(db))]
pub async fn delete_with_parent(
    db: impl PgExecutor<'_>,
    auth: &AuthInfo,
    parent_id: PostId,
    child_id: ReactionId,
) -> Result<(), error_stack::Report<Error>> {
    query_file!(
        "src/models/reaction/delete_with_parent.sql",
        auth.organization_id.as_uuid(),
        parent_id.as_uuid(),
        child_id.as_uuid()
    )
    .execute(db)
    .await
    .change_context(Error::Db)?;
    Ok(())
}

/// Delete all children of the given parent. This function does not do permissions checks.
#[instrument(skip(db))]
pub async fn delete_all_children_of_parent(
    db: impl PgExecutor<'_>,
    organization_id: OrganizationId,
    parent_id: PostId,
) -> Result<(), error_stack::Report<Error>> {
    query_file!(
        "src/models/reaction/delete_all_children.sql",
        organization_id.as_uuid(),
        parent_id.as_uuid()
    )
    .execute(db)
    .await
    .change_context(Error::Db)?;
    Ok(())
}
