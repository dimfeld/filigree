use super::{bindings, query_builder::QueryBuilder, SqlBuilder};
use crate::templates::sql_string;

impl<'a> SqlBuilder<'a> {
    /// A subquery that checks if a user has any of the given permissions
    pub fn permissions_check_where_clause(&self, q: &mut QueryBuilder, perms: &[impl AsRef<str>]) {
        let organization = q.create_binding(bindings::ORGANIZATION);
        let actor_ids = q.create_binding(bindings::ACTOR_IDS);

        let perms = perms
            .iter()
            .map(|s| sql_string(s.as_ref()))
            .collect::<Vec<_>>()
            .join(", ");

        q.push(&format!(
            r##"EXISTS (
            SELECT 1
            FROM {auth_schema}.permissions
            WHERE
            organization_id = {organization}
            AND actor_id = ANY({actor_ids})
            AND permission IN ({perms})
          )"##,
            auth_schema = self.context.auth_schema
        ));
    }

    /// Look up if the user has owner or user permissions on an object type.
    pub fn permissions_lookup_query(
        &self,
        q: &mut QueryBuilder,
        owner_perm: &str,
        user_perm: &str,
    ) {
        let owner_perm = sql_string(owner_perm);
        let user_perm = sql_string(user_perm);
        let organization = q.create_binding(bindings::ORGANIZATION);
        let actor_ids = q.create_binding(bindings::ACTOR_IDS);
        let s = format!(
            r##"
            SELECT
              COALESCE(bool_or(permission in ('org_admin', {owner_perm}) ), false) as is_owner,
              COALESCE(bool_or(permission in ('org_admin', {owner_perm}, {user_perm}) ), false) as is_user
            FROM {auth_schema}.permissions
            WHERE
                organization_id = {organization}
                AND actor_id = ANY({actor_ids})
                AND permission in ('org_admin', {owner_perm}, {user_perm})
           "##,
            auth_schema = self.context.auth_schema
        );

        q.push(&s);
    }

    /// A subquery that looks up an object's permissions when using an object auth model
    fn object_permissions_value_query(
        &self,
        q: &mut QueryBuilder,
        owner_perm: &str,
        write_perm: &str,
        read_perm: &str,
    ) {
        let owner_perm = sql_string(owner_perm);
        let write_perm = sql_string(write_perm);
        let read_perm = sql_string(read_perm);

        let organization = q.create_binding(bindings::ORGANIZATION);
        let actor_ids = q.create_binding(bindings::ACTOR_IDS);
        let object_id = q.create_binding(bindings::ID);

        let s = format!(
            r##"
            SELECT
              CASE
                WHEN bool_or(permission in ('org_admin', {owner_perm})) THEN 'owner'
                WHEN bool_or(permission = {write_perm}) THEN 'write'
                WHEN bool_or(permission = {read_perm}) THEN 'read'
                ELSE NULL
              END _permission
            FROM {auth_schema}.object_permissions
            WHERE
                organization_id = {organization}
                AND actor_id = ANY({actor_ids})
                AND object_id = {object_id}
                AND permission in ('org_admin', {owner_perm}, {write_perm}, {read_perm})
        "##,
            auth_schema = self.context.auth_schema
        );

        q.push(&s);
    }
}
