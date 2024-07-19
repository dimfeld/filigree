use crate::{config::Config, write::ModelMap, Error};

pub fn validate_model_configuration(config: &Config, models: &ModelMap) -> Result<(), Error> {
    for (_, model) in &models.0 {
        for file in &model.files {
            file.validate(&model.name, config)?;
        }

        for field in &model.fields {
            if let Some(reference) = &field.references {
                reference.validate(&model.name, &field.name)?;
            }
        }

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
            } else if !child.belongs_to.iter().any(|b| b.model() == model.name) {
                return Err(Error::MissingBelongsTo(
                    model.name.clone(),
                    has.model.clone(),
                ));
            }
        }
    }

    Ok(())
}
