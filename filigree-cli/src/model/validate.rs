use crate::{Error, ModelMap};

pub fn validate_model_configuration(models: &ModelMap) -> Result<(), Error> {
    for (_, model) in &models.0 {
        for has in &model.has {
            let child = models.get(&has.model, &model.name, "has")?;

            if let Some(through) = &has.through {
                // When using a through model we don't need a belongs_to on the other side,
                // but validate that the through model properly references this model.
                let through_model = models.get(through, &model.name, "through")?;

                if let Some(joins) = &through_model.joins {
                    if joins.0 != model.name && joins.1 != model.name {
                        return Err(Error::BadJoin(
                            model.name.clone(),
                            through_model.name.clone(),
                            has.model.clone(),
                            model.name.clone(),
                        ));
                    }

                    if joins.0 != has.model && joins.1 != has.model {
                        return Err(Error::BadJoin(
                            model.name.clone(),
                            through_model.name.clone(),
                            has.model.clone(),
                            has.model.clone(),
                        ));
                    }
                } else {
                    return Err(Error::MissingJoin(
                        model.name.clone(),
                        through_model.name.clone(),
                    ));
                }
            } else if let Some(belongs_to) = &child.belongs_to {
                if belongs_to.model() != model.name {
                    return Err(Error::BelongsToMismatch {
                        parent: model.name.clone(),
                        child: has.model.clone(),
                        child_belongs_to: belongs_to.model().to_string(),
                    });
                }
            } else {
                return Err(Error::MissingBelongsTo(
                    model.name.clone(),
                    has.model.clone(),
                ));
            }
        }
    }

    Ok(())
}
