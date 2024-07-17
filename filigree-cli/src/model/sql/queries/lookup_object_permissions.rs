use std::fmt::Write;

use super::{bindings, QueryBuilder, SqlBuilder, SqlQueryContext};

pub fn create_query(data: &SqlBuilder) -> SqlQueryContext {
    /*
    {{ macros::permissions_value(
      auth_schema=auth_schema,
      auth_scope=auth_scope,
      organization="$1",
      actor_ids="$2",
      object_id="$3",
      owner_perm=owner_permission,
      read_perm=read_permission,
      write_perm=write_permission)
    }}
    */
    todo!();
}
