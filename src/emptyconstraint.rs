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
            ConstraintTypeType::StringLike => true,
            ConstraintTypeType::Number => self.value.parse::<f64>().is_ok(),
            ConstraintTypeType::IslandRessource => {
                let value_lower_case = self.value.to_lowercase();
                match value_lower_case.as_str() {
                    "iron" | "stone" | "wood" => true,
                    _ => false,
                }
            }
            ConstraintTypeType::Selection => all_selections.iter().any(|s| s.name == self.value),
        }
    }

    /// given a set of towns, modify said set to only include towns for which the constraint matches.
    pub fn matching_towns(
        &self,
        towns: &mut HashSet<Rc<BackendTown>>,
        all_selections: &[EmptyTownSelection],
        join_mode: AndOr, // TODO: this is a stopgap solution until we have generic fault input handling
    ) {
        let value_f64: Option<f64> = self.value.parse().ok();
        match self.comparator {
            Comparator::LessThan
            | Comparator::Equal
            | Comparator::GreaterThan
            | Comparator::NotEqual => match self.constraint_type {
                ConstraintType::PlayerID => {
                    towns.retain(|t| {
                        // TODO: think about this some more. We probably want the full 4 case match statement for player and input parsing.
                        // Also, we may want to do a join mode distinction for the failure case
                        if let Some(id) = t.player.as_ref().map(|(id, _)| id) {
                            if let Some(value) = value_f64 {
                                self.comparator.compare(*id as f64, value)
                            } else {
                                false
                            }
                        } else {
                            false
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
                            if let Some(value) = value_f64 {
                                self.comparator.compare(points as f64, value)
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    });
                }
                ConstraintType::PlayerRank => {
                    towns.retain(|t| {
                        if let Some(rank) = t.player.as_ref().map(|(_id, player)| player.rank) {
                            if let Some(value) = value_f64 {
                                self.comparator.compare(rank as f64, value)
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    });
                }
                ConstraintType::PlayerTowns => {
                    towns.retain(|t| {
                        if let Some(towns) = t.player.as_ref().map(|(_id, player)| player.towns) {
                            if let Some(value) = value_f64 {
                                self.comparator.compare(towns as f64, value)
                            } else {
                                false
                            }
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
                            .map(|(_id, player)| player.alliance.clone())
                            .flatten()
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
                            .map(|(_id, player)| player.alliance.clone())
                            .flatten()
                            .map(|(_id, alliance)| alliance.points)
                        {
                            if let Some(value) = value_f64 {
                                self.comparator.compare(points as f64, value)
                            } else {
                                false
                            }
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
                            .map(|(_id, player)| player.alliance.clone())
                            .flatten()
                            .map(|(_id, alliance)| alliance.towns)
                        {
                            if let Some(value) = value_f64 {
                                self.comparator.compare(towns as f64, value)
                            } else {
                                false
                            }
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
                            .map(|(_id, player)| player.alliance.clone())
                            .flatten()
                            .map(|(_id, alliance)| alliance.members)
                        {
                            if let Some(value) = value_f64 {
                                self.comparator.compare(members as f64, value)
                            } else {
                                false
                            }
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
                            .map(|(_id, player)| player.alliance.clone())
                            .flatten()
                            .map(|(_id, alliance)| alliance.rank)
                        {
                            if let Some(value) = value_f64 {
                                self.comparator.compare(rank as f64, value)
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    });
                }
                ConstraintType::TownID => {
                    towns.retain(|t| {
                        if let Some(value) = value_f64 {
                            self.comparator.compare(t.id as f64, value)
                        } else {
                            false
                        }
                    });
                }
                ConstraintType::TownName => {
                    towns.retain(|t| self.comparator.compare(&t.name, &self.value));
                }
                ConstraintType::TownPoints => {
                    towns.retain(|t| {
                        if let Some(value) = value_f64 {
                            self.comparator.compare(t.points as f64, value)
                        } else {
                            false
                        }
                    });
                }
                ConstraintType::IslandID => {
                    towns.retain(|t| {
                        let (_x, _y, island) = &t.island;
                        if let Some(value) = value_f64 {
                            self.comparator.compare(island.id as f64, value)
                        } else {
                            false
                        }
                    });
                }
                ConstraintType::IslandX => {
                    towns.retain(|t| {
                        let (x, _y, _island) = &t.island;
                        if let Some(value) = value_f64 {
                            self.comparator.compare(*x as f64, value)
                        } else {
                            false
                        }
                    });
                }
                ConstraintType::IslandY => {
                    towns.retain(|t| {
                        let (_x, y, _island) = &t.island;
                        if let Some(value) = value_f64 {
                            self.comparator.compare(*y as f64, value)
                        } else {
                            false
                        }
                    });
                }
                ConstraintType::IslandType => {
                    towns.retain(|t| {
                        let (_x, _y, island) = &t.island;
                        if let Some(value) = value_f64 {
                            self.comparator.compare(island.typ as f64, value)
                        } else {
                            false
                        }
                    });
                }
                ConstraintType::IslandTowns => {
                    towns.retain(|t| {
                        let (_x, _y, island) = &t.island;
                        if let Some(value) = value_f64 {
                            self.comparator.compare(island.towns as f64, value)
                        } else {
                            false
                        }
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
                if let Some(selection) = opt_selection {
                    let given_towns = towns.clone();
                    let towns_in_referenced_selection = database::matching_towns_for_selection(
                        given_towns,
                        selection,
                        all_selections,
                    );
                    towns.retain(|t| towns_in_referenced_selection.contains(t));
                } else {
                    match join_mode {
                        AndOr::And => {
                            // if we have AND, the town lists from different constraints are combined as intersections. (reducing the number)
                            // -> do nothing, so "selector" "in" "non existent selection" does not change the townlist (bool &= true)
                        }
                        AndOr::Or => {
                            // if we have OR, the town lists from different constraints are combined as unions. (increasing the number of towns)
                            // -> clear the complete set, so "selector" "in" "non existent selection" does not change the townlist (bool |= false)
                            towns.clear();
                        }
                    }
                }
            }
            Comparator::NotInSelection => {
                let opt_selection = all_selections.iter().find(|s| s.name == self.value);
                if let Some(selection) = opt_selection {
                    let given_towns = towns.clone();
                    let towns_in_referenced_selection = database::matching_towns_for_selection(
                        given_towns,
                        selection,
                        all_selections,
                    );
                    towns.retain(|t| !towns_in_referenced_selection.contains(t));
                } else {
                    match join_mode {
                        AndOr::And => {
                            // if we have AND, the town lists from different constraints are combined as intersections. (reducing the number)
                            // -> clear the complete set, so "selector" "not in" "non existent selection" does not change the townlist (bool &= true)
                            towns.clear();
                        }
                        AndOr::Or => {
                            // if we have OR, the town lists from different constraints are combined as unions. (increasing the number of towns)
                            // -> do nothing, so "selector" "not in" "non existent selection" does not change the townlist (bool |= false)
                        }
                    }
                }
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
