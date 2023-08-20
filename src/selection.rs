use anyhow::Context;
use rand::distributions::{Alphanumeric, DistString};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::default::Default;
use std::fmt;
use std::path::PathBuf;
use std::sync::{mpsc, Arc};

use crate::constraint::Constraint;
use crate::message::MessageToModel;
use crate::town::Town;
use crate::view::Change;

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub enum SelectionState {
    Loading,
    Finished,

    #[default]
    NewlyCreated,
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TownSelection {
    #[serde(default = "String::new")]
    pub name: String,

    #[serde(skip)]
    pub state: SelectionState,

    #[serde(default, with = "short_serialization")]
    pub constraints: Vec<Constraint>,

    #[serde(default)]
    pub color: egui::Color32,

    #[serde(skip)]
    pub towns: Arc<Vec<Town>>,
}

impl TownSelection {
    /// Clone the `TownSelection`, but without the list of towns. Less memory
    /// required and we can reconstruct the list of towns anyway, if given
    /// the list of constraints.
    pub fn partial_clone(&self) -> Self {
        Self {
            towns: Arc::new(Vec::new()),
            name: self.name.clone(),
            state: self.state, // implements copy
            constraints: self.constraints.clone(),
            color: self.color, // implements copy
        }
    }

    pub fn try_from_str(text: &str) -> anyhow::Result<Vec<Self>> {
        // Attempt to parse text as a vector of selections, and it that doesn't work, parse it as a single selection.
        let res_parse_as_vec = serde_yaml::from_str(text);
        let res_parse_as_single = serde_yaml::from_str(text);

        match (res_parse_as_vec, res_parse_as_single) {
            (Ok(vec), _) => Ok(vec),
            (Err(_err), Ok(selection)) => Ok(vec![selection]),
            (Err(err_vec), Err(err_single)) => {
                eprintln!("Could not parse text ({text}) as TownSelection (Error: {err_single:?}) or Vec<TownSelection> (Error: {err_vec:?}).");
                Err(
                    anyhow::Error::new(err_vec)
                    .context(err_single)
                    .context("Could not parse text ({text}) as TownSelection (Error: {single_err:?}) or Vec<TownSelection> (Error: {vec_err:?}).")
                )
            }
        }
    }

    pub fn try_from_path(files: &[PathBuf]) -> Vec<anyhow::Result<Vec<Self>>> {
        let mut re = Vec::with_capacity(files.len());
        for file in files {
            let content = std::fs::read_to_string(file);
            re.push(match content {
                Ok(content) => {
                    Self::try_from_str(&content).with_context(|| {format!("Failed to convert content of file {file:?} to an instance of TownSelection. Content of file is: {content}")})
                }
                Err(err) => {
                        Err(err)
                            .with_context(|| format!("Failed to read content of file {file:?}"))
                }
            });
        }
        re
    }

    pub fn refresh(&mut self, channel_tx: &mpsc::Sender<MessageToModel>) {
        self.state = SelectionState::Loading;
        for constraint in &mut self.constraints {
            constraint.drop_down_values = None;
        }

        channel_tx
            .send(MessageToModel::FetchTowns(
                self.partial_clone(),
                HashSet::new(),
            ))
            .expect(&format!(
                "Failed to send Message to Model for Selection {}",
                &self
            ));
    }

    pub fn make_ui(
        &mut self,
        ui: &mut egui::Ui,
        channel_tx: &mpsc::Sender<MessageToModel>,
        selection_index: usize,
    ) -> Option<Change> {
        let mut re = None;

        let _first_row_response = ui.horizontal(|ui| {
            // TODO make the selection collapsible
            if ui.button("+").clicked() {
                re = Some(Change::Add);
            }
            if ui.button("-").clicked() {
                re = Some(Change::Remove(selection_index));
            }
            if ui.button("↑").clicked() {
                re = Some(Change::MoveUp(selection_index));
            }
            if ui.button("↓").clicked() {
                re = Some(Change::MoveDown(selection_index));
            }
            ui.add_sized(
                [
                    ui.style().spacing.interact_size.x * 6.0,
                    ui.style().spacing.interact_size.y,
                ],
                egui::TextEdit::singleline(&mut self.name),
            );
            ui.color_edit_button_srgba(&mut self.color);
            ui.label(format!("{} Towns", self.towns.len()));
            if self.state == SelectionState::Loading {
                ui.spinner();
            }
        });

        let num_constraints = self.constraints.len();
        let mut edited_constraints = HashSet::new();
        let mut constraint_change_action = None;
        for (constraint_index, constraint) in self.constraints.iter_mut().enumerate() {
            let (change, edited) = constraint.make_ui(
                ui,
                selection_index,
                constraint_index,
                constraint_index + 1 == num_constraints,
            );

            if edited {
                edited_constraints.insert(constraint.partial_clone());
            }

            if change.is_some() {
                constraint_change_action = change;
            }
        }
        if let Some(change) = constraint_change_action {
            match change {
                Change::MoveUp(index) => {
                    if index >= 1 {
                        self.constraints.swap(index, index - 1);
                    }
                }
                Change::Remove(index) => {
                    let _element = self.constraints.remove(index);
                    if self.constraints.is_empty() {
                        // ensure there is always at least one constraint
                        self.constraints.push(Constraint::default());
                    }
                }
                Change::MoveDown(index) => {
                    if index + 1 < self.constraints.len() {
                        self.constraints.swap(index, index + 1);
                    }
                }
                Change::Add => self.constraints.push(Constraint::default()),
            }
        }

        let refresh_complete_selection = matches!(
            (self.state, constraint_change_action),
            (SelectionState::NewlyCreated, _)  // reload everything if this selection is newly created (This is probably not needed, but I'll leave it in, just to be save)
                 | (_, Some(Change::Add | Change::Remove(_))) // or if a constraint was added or removed
        );
        if refresh_complete_selection {
            self.towns = Arc::new(Vec::new());
            self.refresh(channel_tx);
        } else if !edited_constraints.is_empty() {
            self.state = SelectionState::Loading;
            for constraint in &mut self
                .constraints
                .iter_mut()
                .filter(|c| !edited_constraints.contains(c))
            {
                // the ddvs of all constraints that were not edited are invalidated.
                constraint.drop_down_values = None;
            }

            channel_tx
                .send(MessageToModel::FetchTowns(
                    self.partial_clone(),
                    edited_constraints,
                ))
                .expect(&format!(
                    "Failed to send Message to Model for selection {}",
                    &self
                ));
        }

        re
    }
}

/// custom serialization for constraints
mod short_serialization {
    use std::fmt;

    use anyhow::Context;
    use serde::{
        de::{self, SeqAccess, Visitor},
        ser,
        ser::SerializeSeq,
        Deserialize, Deserializer, Serializer,
    };

    use crate::constraint::Constraint;

    pub fn serialize<S>(constraints: &[Constraint], serializer: S) -> Result<S::Ok, S::Error>
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

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<Constraint>, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(ConstraintArrayDeserializer)
    }

    // deserializing is much more complex than serializing,
    // see https://stackoverflow.com/a/62705102
    // and https://serde.rs/impl-deserialize.html
    // and https://serde.rs/deserialize-struct.html
    struct ConstraintArrayDeserializer;

    impl<'de> Visitor<'de> for ConstraintArrayDeserializer {
        type Value = Vec<Constraint>;

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

    struct ConstraintVisitor;
    impl<'de> Visitor<'de> for ConstraintVisitor {
        type Value = Constraint;

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

            let re = Constraint {
                constraint_type,
                comparator,
                value: value.trim_matches('\'').to_owned(),
                drop_down_values: None,
            };
            // println!("{re:?}");
            Ok(re)
        }
    }

    impl<'de> Deserialize<'de> for Constraint {
        fn deserialize<D>(deserializer: D) -> Result<Constraint, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_string(ConstraintVisitor)
        }
    }
}

impl Default for TownSelection {
    fn default() -> Self {
        Self {
            name: Alphanumeric.sample_string(&mut rand::thread_rng(), 6), // https://stackoverflow.com/a/72977937
            state: SelectionState::NewlyCreated,
            towns: Arc::new(Vec::new()),
            constraints: vec![Constraint::default()],
            color: egui::Color32::GREEN,
        }
    }
}

impl PartialEq<TownSelection> for &mut TownSelection {
    fn eq(&self, other: &TownSelection) -> bool {
        self.name == other.name
            && self.constraints == other.constraints
            && self.color == other.color
    }
}
impl PartialEq<TownSelection> for TownSelection {
    fn eq(&self, other: &TownSelection) -> bool {
        self.name == other.name
            && self.constraints == other.constraints
            && self.color == other.color
    }
}

impl fmt::Display for TownSelection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TownSelection({} constraints, {} towns)",
            self.constraints.len(),
            self.towns.len()
        )
    }
}
