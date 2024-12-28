use crate::{
    constraint::{Comparator, Constraint, ConstraintType},
    emptyselection::EmptyTownSelection,
    model::database::BackendTown,
};
use std::{fmt, hash::Hash};

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

    pub fn matches(&self, t: &&BackendTown, all_selections: &[EmptyTownSelection]) -> bool {
        // TODO: instead of one town we should take a hashset of towns and return (said set).retain(|t| (the current code))
        //       this is a schema that makes it much easier to incorporate the inselection/notinselection comparators as well.
        let value_f64: Option<f64> = self.value.parse().ok();
        match self.comparator {
            Comparator::LessThan
            | Comparator::Equal
            | Comparator::GreaterThan
            | Comparator::NotEqual => match self.constraint_type {
                ConstraintType::PlayerID => {
                    if let Some(id) = t.player.as_ref().map(|(id, _)| id) {
                        if let Some(value) = value_f64 {
                            self.comparator.compare(*id as f64, value)
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }
                ConstraintType::PlayerName => {
                    if let Some(name) = t.player.as_ref().map(|(_id, player)| &player.name) {
                        self.comparator.compare(name, &self.value)
                    } else {
                        false
                    }
                }
                ConstraintType::PlayerPoints => {
                    if let Some(points) = t.player.as_ref().map(|(_id, player)| player.points) {
                        if let Some(value) = value_f64 {
                            self.comparator.compare(points as f64, value)
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }
                ConstraintType::PlayerRank => {
                    if let Some(rank) = t.player.as_ref().map(|(_id, player)| player.rank) {
                        if let Some(value) = value_f64 {
                            self.comparator.compare(rank as f64, value)
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }
                ConstraintType::PlayerTowns => {
                    if let Some(towns) = t.player.as_ref().map(|(_id, player)| player.towns) {
                        if let Some(value) = value_f64 {
                            self.comparator.compare(towns as f64, value)
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }
                ConstraintType::AllianceName => {
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
                }
                ConstraintType::AlliancePoints => {
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
                }
                ConstraintType::AllianceTowns => {
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
                }
                ConstraintType::AllianceMembers => {
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
                }
                ConstraintType::AllianceRank => {
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
                }
                ConstraintType::TownID => {
                    if let Some(value) = value_f64 {
                        self.comparator.compare(t.id as f64, value)
                    } else {
                        false
                    }
                }
                ConstraintType::TownName => self.comparator.compare(&t.name, &self.value),
                ConstraintType::TownPoints => {
                    if let Some(value) = value_f64 {
                        self.comparator.compare(t.points as f64, value)
                    } else {
                        false
                    }
                }
                ConstraintType::IslandID => {
                    let (_x, _y, island) = &t.island;
                    if let Some(value) = value_f64 {
                        self.comparator.compare(island.id as f64, value)
                    } else {
                        false
                    }
                }
                ConstraintType::IslandX => {
                    let (x, _y, _island) = &t.island;
                    if let Some(value) = value_f64 {
                        self.comparator.compare(*x as f64, value)
                    } else {
                        false
                    }
                }
                ConstraintType::IslandY => {
                    let (_x, y, _island) = &t.island;
                    if let Some(value) = value_f64 {
                        self.comparator.compare(*y as f64, value)
                    } else {
                        false
                    }
                }
                ConstraintType::IslandType => {
                    let (_x, _y, island) = &t.island;
                    if let Some(value) = value_f64 {
                        self.comparator.compare(island.typ as f64, value)
                    } else {
                        false
                    }
                }
                ConstraintType::IslandTowns => {
                    let (_x, _y, island) = &t.island;
                    if let Some(value) = value_f64 {
                        self.comparator.compare(island.towns as f64, value)
                    } else {
                        false
                    }
                }
                ConstraintType::IslandResMore => {
                    let (_x, _y, island) = &t.island;
                    self.comparator.compare(&island.ressource_plus, &self.value)
                }
                ConstraintType::IslandResLess => {
                    let (_x, _y, island) = &t.island;
                    self.comparator
                        .compare(&island.ressource_minus, &self.value)
                }
            },
            Comparator::InSelection | Comparator::NotInSelection => todo!(),
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
