use crate::{
    constraint::{Comparator, Constraint, ConstraintType, ConstraintTypeType},
    emptyselection::EmptyTownSelection,
    model::database::{self, BackendTown},
    selection::AndOr,
};
use std::{collections::HashSet, fmt, hash::Hash, rc::Rc};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

    /// checks if the constraint has input that can be considered "valid". That means that number
    ///constraints can parse their userinput as numbers, in/notin constraints have input that is a
    ///name of another selection and for ressource constraints the strings match exactly to one of
    ///the options (ignoring case). For other string like constraints we always return true.
    pub fn has_valid_input(&self, all_selections: &[EmptyTownSelection]) -> bool {
        // TODO: do this check in the frontend and highlight invalid input
        let constraint_type_type: ConstraintTypeType = self.into();
        match constraint_type_type {
            ConstraintTypeType::StringLike => !self.value.is_empty(),
            ConstraintTypeType::Number => self.value.parse::<f64>().is_ok(),
            ConstraintTypeType::IslandRessource => {
                matches!(
                    self.value.to_lowercase().as_str(),
                    "iron" | "stone" | "wood"
                )
            }
            ConstraintTypeType::Selection => all_selections.iter().any(|s| s.name == self.value),
        }
    }

    /// given a set of towns, modify said set to only include towns for which the constraint matches.
    #[allow(clippy::too_many_lines)]
    pub fn matching_towns(
        &self,
        towns: &mut HashSet<Rc<BackendTown>>,
        all_selections: &[EmptyTownSelection],
        join_mode: AndOr, // NOTE: this could be dropped, since database.rs ensures this method is never called for constraints that do not have a valid input.
    ) {
        // ensure that we have valid input and return a result that does not change the resulting list if we do no have valid input.
        if !self.has_valid_input(all_selections) {
            match join_mode {
                AndOr::And => {
                    // do nothing, `list &= list` does not change it
                }
                AndOr::Or => {
                    // clear all towns, `list |= nothing` does not change list
                    towns.clear();
                }
            }
            return;
        }

        let constraint_type_type: ConstraintTypeType = self.into();
        let value_f64: f64 =        match constraint_type_type {
            ConstraintTypeType::Number => {
                        self.value.parse().expect("we ran EmptyConstraint::has_valid_input just before this. So unwrap _must_ be fine here!")
            },
            ConstraintTypeType::StringLike |
            ConstraintTypeType::IslandRessource |
            ConstraintTypeType::Selection => {
                0f64 // should not matter at all
            }
        };
        match self.comparator {
            Comparator::LessThan
            | Comparator::Equal
            | Comparator::GreaterThan
            | Comparator::NotEqual => match self.constraint_type {
                ConstraintType::PlayerID => {
                    towns.retain(|t| {
                        if let Some(id) = t.player.as_ref().map(|(id, _)| id) {
                            self.comparator.compare(f64::from(*id), value_f64)
                        } else {
                            // Use empty player ID as a sentinel for ghost towns.
                            // TODO: make it so that the empty selection filtering in the presenter
                            //   does not block the usage of this. At the moment this selection is
                            //   impossible, because the constraint is filtered from being passed to
                            //   database. Or maybe we have some more bogus self.value.is_empty() in
                            //   database.rs as well.
                            self.value.is_empty()
                        }
                    });
                }
                ConstraintType::PlayerName => {
                    towns.retain(|t| {
                        if let Some(name) = t.player.as_ref().map(|(_id, player)| &player.name) {
                            self.comparator.compare(name, &self.value)
                        } else {
                            false
                        }
                    });
                }
                ConstraintType::PlayerPoints => {
                    towns.retain(|t| {
                        if let Some(points) = t.player.as_ref().map(|(_id, player)| player.points) {
                            self.comparator.compare(f64::from(points), value_f64)
                        } else {
                            false
                        }
                    });
                }
                ConstraintType::PlayerRank => {
                    towns.retain(|t| {
                        if let Some(rank) = t.player.as_ref().map(|(_id, player)| player.rank) {
                            self.comparator.compare(f64::from(rank), value_f64)
                        } else {
                            false
                        }
                    });
                }
                ConstraintType::PlayerTowns => {
                    towns.retain(|t| {
                        if let Some(towns) = t.player.as_ref().map(|(_id, player)| player.towns) {
                            self.comparator.compare(f64::from(towns), value_f64)
                        } else {
                            false
                        }
                    });
                }
                ConstraintType::AllianceName => {
                    towns.retain(|t| {
                        if let Some(name) = t
                            .player
                            .as_ref()
                            .and_then(|(_id, player)| player.alliance.clone())
                            .map(|(_id, alliance)| alliance.name.clone())
                        {
                            self.comparator.compare(&name, &self.value)
                        } else {
                            false
                        }
                    });
                }
                ConstraintType::AlliancePoints => {
                    towns.retain(|t| {
                        if let Some(points) = t
                            .player
                            .as_ref()
                            .and_then(|(_id, player)| player.alliance.clone())
                            .map(|(_id, alliance)| alliance.points)
                        {
                            self.comparator.compare(f64::from(points), value_f64)
                        } else {
                            false
                        }
                    });
                }
                ConstraintType::AllianceTowns => {
                    towns.retain(|t| {
                        if let Some(towns) = t
                            .player
                            .as_ref()
                            .and_then(|(_id, player)| player.alliance.clone())
                            .map(|(_id, alliance)| alliance.towns)
                        {
                            self.comparator.compare(f64::from(towns), value_f64)
                        } else {
                            false
                        }
                    });
                }
                ConstraintType::AllianceMembers => {
                    towns.retain(|t| {
                        if let Some(members) = t
                            .player
                            .as_ref()
                            .and_then(|(_id, player)| player.alliance.clone())
                            .map(|(_id, alliance)| alliance.members)
                        {
                            self.comparator.compare(f64::from(members), value_f64)
                        } else {
                            false
                        }
                    });
                }
                ConstraintType::AllianceRank => {
                    towns.retain(|t| {
                        if let Some(rank) = t
                            .player
                            .as_ref()
                            .and_then(|(_id, player)| player.alliance.clone())
                            .map(|(_id, alliance)| alliance.rank)
                        {
                            self.comparator.compare(f64::from(rank), value_f64)
                        } else {
                            false
                        }
                    });
                }
                ConstraintType::TownID => {
                    towns.retain(|t| self.comparator.compare(f64::from(t.id), value_f64));
                }
                ConstraintType::TownName => {
                    towns.retain(|t| self.comparator.compare(&t.name, &self.value));
                }
                ConstraintType::TownPoints => {
                    towns.retain(|t| self.comparator.compare(f64::from(t.points), value_f64));
                }
                ConstraintType::IslandID => {
                    towns.retain(|t| {
                        let (_x, _y, island) = &t.island;
                        self.comparator.compare(f64::from(island.id), value_f64)
                    });
                }
                ConstraintType::IslandX => {
                    towns.retain(|t| {
                        let (x, _y, _island) = &t.island;
                        self.comparator.compare(f64::from(*x), value_f64)
                    });
                }
                ConstraintType::IslandY => {
                    towns.retain(|t| {
                        let (_x, y, _island) = &t.island;
                        self.comparator.compare(f64::from(*y), value_f64)
                    });
                }
                ConstraintType::IslandType => {
                    towns.retain(|t| {
                        let (_x, _y, island) = &t.island;
                        self.comparator.compare(f64::from(island.typ), value_f64)
                    });
                }
                ConstraintType::IslandTowns => {
                    towns.retain(|t| {
                        let (_x, _y, island) = &t.island;
                        self.comparator.compare(f64::from(island.towns), value_f64)
                    });
                }
                ConstraintType::IslandResMore => {
                    towns.retain(|t| {
                        let (_x, _y, island) = &t.island;
                        self.comparator.compare(&island.ressource_plus, &self.value)
                    });
                }
                ConstraintType::IslandResLess => {
                    towns.retain(|t| {
                        let (_x, _y, island) = &t.island;
                        self.comparator
                            .compare(&island.ressource_minus, &self.value)
                    });
                }
            },
            Comparator::InSelection => {
                let opt_selection = all_selections.iter().find(|s| s.name == self.value);
                let selection = opt_selection.expect("we ran EmptyConstraint::has_valid_input just before this. So unwrap _must_ be fine here!");
                let towns_in_referenced_selection =
                    database::matching_towns_for_selection(towns, selection, all_selections);
                towns.retain(|t| towns_in_referenced_selection.contains(t));
            }
            Comparator::NotInSelection => {
                let opt_selection = all_selections.iter().find(|s| s.name == self.value);
                let selection = opt_selection.expect("we ran EmptyConstraint::has_valid_input just before this. So unwrap _must_ be fine here!");
                let towns_in_referenced_selection =
                    database::matching_towns_for_selection(towns, selection, all_selections);
                towns.retain(|t| !towns_in_referenced_selection.contains(t));
            }
        };
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
    impl Visitor<'_> for EmptyConstraintVisitor {
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
