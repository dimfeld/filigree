use super::{query_builder::QueryBuilder, SqlBuilder};
use crate::model::{field::ModelFieldTemplateContext, sql::bindings};

impl<'a> SqlBuilder<'a> {
    /// Return the ID field for the model, or the two parent ID fields for a joining model, and the
    /// bindings names to use for the IDs.
    pub fn id_fields(&self) -> Vec<(&str, &str)> {
        if let Some(join) = &self.context.join {
            vec![
                (join.model_ids.0.as_str(), bindings::JOIN_ID_0),
                (join.model_ids.1.as_str(), bindings::JOIN_ID_1),
            ]
        } else {
            vec![("id", bindings::ID)]
        }
    }

    pub fn id_field_contexts(&self) -> Vec<ModelFieldTemplateContext> {
        if let Some(join) = &self.context.join {
            self.context
                .fields
                .iter()
                .filter(|f| f.name == join.model_ids.0 || f.name == join.model_ids.1)
                .cloned()
                .map(|mut f| {
                    if f.name == join.model_ids.0 {
                        f.override_binding_name = Some(bindings::JOIN_ID_0.to_string());
                    } else if f.name == join.model_ids.1 {
                        f.override_binding_name = Some(bindings::JOIN_ID_1.to_string());
                    }

                    f
                })
                .collect()
        } else {
            self.context
                .fields
                .iter()
                .filter(|f| f.name == "id")
                .cloned()
                .collect()
        }
    }

    pub fn id_and_org_field_contexts(&self) -> Vec<ModelFieldTemplateContext> {
        let include_org = !self.context.global;
        if let Some(join) = &self.context.join {
            self.context
                .fields
                .iter()
                .filter(|f| {
                    f.name == join.model_ids.0
                        || f.name == join.model_ids.1
                        || (include_org && f.name == "organization_id")
                })
                .cloned()
                .map(|mut f| {
                    if f.name == join.model_ids.0 {
                        f.override_binding_name = Some(bindings::JOIN_ID_0.to_string());
                    } else if f.name == join.model_ids.1 {
                        f.override_binding_name = Some(bindings::JOIN_ID_1.to_string());
                    }

                    f
                })
                .collect()
        } else {
            self.context
                .fields
                .iter()
                .filter(|f| f.name == "id" || (include_org && f.name == "organization_id"))
                .cloned()
                .collect()
        }
    }

    /// Add a clause to the query that filters on the ID fields for the model.
    pub fn push_id_where_clause(&self, q: &mut QueryBuilder) {
        let mut where_sep = q.separated(" AND ");
        for id_field in self.id_fields() {
            where_sep.push(id_field.0);
            where_sep.push_unseparated(" = ");
            where_sep.push_binding_unseparated(id_field.1);
        }
    }

    /// Given an ID field, return the other ID field for a joining model. For non-joining models,
    /// this is always "id".
    pub fn other_id_field(&self, id_field: &str) -> &str {
        if let Some(join) = &self.context.join {
            if id_field == join.model_ids.0 {
                join.model_ids.1.as_str()
            } else {
                join.model_ids.0.as_str()
            }
        } else {
            "id"
        }
    }
}
