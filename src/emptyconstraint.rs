use crate::{
    constraint::{Comparator, Constraint, ConstraintType},
    emptyselection::EmptyTownSelection,
    model::database::Database,
};
use std::{
    fmt,
    hash::{Hash, Hasher},
};

#[derive(Clone)]
pub struct EmptyConstraint {
    pub constraint_type: ConstraintType,
    pub comparator: Comparator,
    pub value: String,
}

impl fmt::Debug for EmptyConstraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Constraint({} {} {})",
            self.constraint_type.to_string(),
            self.comparator.to_string(),
            self.value,
        )
    }
}

impl EmptyConstraint {
    pub fn fill(&self) -> Constraint {
        Constraint {
            constraint_type: self.constraint_type,
            comparator: self.comparator,
            value: self.value.clone(),
            drop_down_values: None,
        }
    }

    pub fn referenced_selection(&self) -> Option<String> {
        match self.comparator {
            Comparator::LessThan
            | Comparator::Equal
            | Comparator::GreaterThan
            | Comparator::NotEqual => None,
            Comparator::InSelection | Comparator::NotInSelection => Some(self.value.clone()),
        }
    }

    pub fn get_sql_value(&self, db: &Database, all_selections: &[EmptyTownSelection]) -> String {
        match self.comparator {
            Comparator::LessThan
            | Comparator::Equal
            | Comparator::GreaterThan
            | Comparator::NotEqual => self.value.clone(),
            Comparator::InSelection | Comparator::NotInSelection => {
                let definitely_true = format!(
                    "SELECT {0}.{1} FROM {0}",
                    self.constraint_type.table(),
                    self.constraint_type.property()
                );
                let definitely_false = String::new();

                if self.value.is_empty() {
                    return definitely_true;
                }

                let target_selection = all_selections
                    .iter()
                    .find(|&selection| selection.name == self.value);
                if let Some(selection) = target_selection {
                    // user has typed in a valid name
                    let selection_clause = format!(
                        "{}.{}",
                        self.constraint_type.table(),
                        self.constraint_type.property()
                    );

                    // TODO error handling
                    db.selection_to_sql(&selection_clause, selection, all_selections)
                        .unwrap()
                } else {
                    // The user typed in a selection name that does not exist. If the user wants towns that are IN
                    // this imaginary selection, they'll get none. If they want all town that are NOT IN this imaginary
                    // selection they'll get all possible ones (an empty selection does not restrict the search in any way)
                    if self.comparator == Comparator::NotInSelection {
                        definitely_true
                    } else if self.comparator == Comparator::InSelection {
                        definitely_false
                    } else {
                        unreachable!(
                            "The comparator type somehow changed between the match and the if case"
                        )
                    }
                }
            }
        }
    }
}

impl Default for EmptyConstraint {
    fn default() -> Self {
        Self {
            constraint_type: ConstraintType::PlayerName,
            comparator: Comparator::Equal,
            value: String::new(),
        }
    }
}

impl Eq for EmptyConstraint {}
impl PartialEq for EmptyConstraint {
    fn eq(&self, other: &Self) -> bool {
        self.constraint_type == other.constraint_type
            && self.comparator == other.comparator
            && self.value == other.value
    }
}

impl Hash for EmptyConstraint {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.constraint_type.hash(state);
        self.comparator.hash(state);
        self.value.hash(state);
    }
}

impl fmt::Display for EmptyConstraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Constraint({} {} {})",
            self.constraint_type.to_string(),
            self.comparator.to_string(),
            self.value
        )
    }
}

/// custom serialization for constraints
pub mod short_serialization {
    use std::fmt;

    use anyhow::Context;
    use serde::{
        de::{self, SeqAccess, Visitor},
        ser,
        ser::SerializeSeq,
        Deserialize, Deserializer, Serializer,
    };

    use super::EmptyConstraint;

    pub fn serialize<S>(constraints: &[EmptyConstraint], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(constraints.len()))?;
        for constraint in constraints {
            let a = serde_yaml::to_string(&constraint.constraint_type)
                .with_context(|| {
                    format!("Failed at serializing the Type of {constraint:?} into a string")
                })
                .map_err(ser::Error::custom)?;
            let b = serde_yaml::to_string(&constraint.comparator)
                .with_context(|| {
                    format!("Failed at serializing the comparator of {constraint:?} into a string")
                })
                .map_err(ser::Error::custom)?;
            let c = serde_yaml::to_string(&constraint.value)
                .with_context(|| {
                    format!("Failed at serializing the value of {constraint:?} into a string")
                })
                .map_err(ser::Error::custom)?;
            seq.serialize_element(&format!(
                "{} {} {}",
                a.trim(),
                b.trim(),
                c.trim() //.trim_matches('\'')
            ))?;
        }
        seq.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<EmptyConstraint>, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(EmptyConstraintArrayDeserializer)
    }

    // deserializing is much more complex than serializing,
    // see https://stackoverflow.com/a/62705102
    // and https://serde.rs/impl-deserialize.html
    // and https://serde.rs/deserialize-struct.html
    struct EmptyConstraintArrayDeserializer;

    impl<'de> Visitor<'de> for EmptyConstraintArrayDeserializer {
        type Value = Vec<EmptyConstraint>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("ArrayKeyedMap key value sequence.")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut re = Vec::new();
            while let Some(value) = seq.next_element()? {
                re.push(value);
            }

            Ok(re)
        }
    }

    struct EmptyConstraintVisitor;
    impl<'de> Visitor<'de> for EmptyConstraintVisitor {
        type Value = EmptyConstraint;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("anything in the form <ConstraintType> <Comparator> \"value\"")
        }

        fn visit_str<E>(self, text: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            let mut split = text.splitn(3, ' ');
            let a = split
                .next()
                .with_context(|| format!("Failed at splitting {text} into 1/three parts in order to parse it into a Constraint"))
                .map_err(de::Error::custom)?;
            let b = split
                .next()
                .with_context(|| format!("Failed at splitting {text} into 2/three parts in order to parse it into a Constraint"))
                .map_err(de::Error::custom)?;
            let c = split
                .next()
                .with_context(|| format!("Failed at splitting {text} into 3/three parts in order to parse it into a Constraint"))
                .map_err(de::Error::custom)?;

            // println!(">>{text}<>{a}<>{b}<>{c}<<");
            let constraint_type = serde_yaml::from_str(a).with_context(|| format!("Failed at parsing {a} into a ConstraintType and therefore failed at turing \"{text}\" into a Constraint")).map_err(de::Error::custom)?;
            let comparator = serde_yaml::from_str(b).with_context(|| format!("Failed at parsing {b} into a Comparator and therefore failed at turing \"{text}\" into a Constraint")).map_err(de::Error::custom)?;
            let value: String = serde_yaml::from_str(c).with_context(|| format!("Failed at parsing {c} into a String and therefore failed at turing \"{text}\" into a Constraint")).map_err(de::Error::custom)?;

            let re = EmptyConstraint {
                constraint_type,
                comparator,
                value: value.trim_matches('\'').to_owned(),
            };
            // println!("{re:?}");
            Ok(re)
        }
    }

    impl<'de> Deserialize<'de> for EmptyConstraint {
        fn deserialize<D>(deserializer: D) -> Result<EmptyConstraint, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_string(EmptyConstraintVisitor)
        }
    }
}
