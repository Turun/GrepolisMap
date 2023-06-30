use core::fmt;
use std::sync::Arc;

use crate::towns::{Constraint, ConstraintType, Town, TownSelection};

/// This is a file for the messages passed between the view and the presenter.
/// message passing communication allows them to be on separate threads. Also it's good code hygene

#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub enum MessageToView {
    Loading(Progress),
    GotServer,
    AllTowns(Arc<Vec<Town>>),
    GhostTowns(Arc<Vec<Town>>),
    DropDownValues(ConstraintType, Arc<Vec<String>>),
    TownListForSelection(TownSelection, Arc<Vec<Town>>),
    ValueListForConstraint(Constraint, TownSelection, Arc<Vec<String>>),
}

impl fmt::Display for MessageToView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageToView::GotServer => {
                write!(f, "MessageToView::GotServer",)
            }
            MessageToView::TownListForSelection(selection, towns) => write!(
                f,
                "MessageToView::TownListForSelection({}, {} towns)",
                selection,
                towns.len()
            ),
            MessageToView::ValueListForConstraint(constraint, selection, towns) => {
                write!(
                    f,
                    "MessageToView::TownListForConstraint({}, {}, {} towns)",
                    constraint,
                    selection,
                    towns.len()
                )
            }
            MessageToView::Loading(progress) => write!(f, "MessageToView::Loading({progress:?})"),
            MessageToView::AllTowns(towns) => {
                write!(f, "MessageToView::AllTowns({} towns)", towns.len())
            }
            MessageToView::GhostTowns(towns) => {
                write!(f, "MessageToView::GhostTowns({} towns)", towns.len())
            }
            MessageToView::DropDownValues(constraint_type, values) => {
                write!(
                    f,
                    "MessageToView::DropDownValues({}: {} entries)",
                    constraint_type,
                    values.len()
                )
            }
        }
    }
}

#[allow(clippy::module_name_repetitions)]
pub enum MessageToModel {
    SetServer(Server, egui::Context),
    FetchAll,
    FetchGhosts,
    FetchDropDownValues(ConstraintType),
    FetchTowns(TownSelection),
}

impl fmt::Display for MessageToModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageToModel::SetServer(server, _frame) => {
                write!(f, "MessageToMode::SetServer({})", server.id)
            }
            MessageToModel::FetchTowns(selection) => {
                write!(f, "MessageToModel::FetchTowns({selection})")
            }
            MessageToModel::FetchAll => {
                write!(f, "MessageToModel::FetchAll")
            }
            MessageToModel::FetchGhosts => {
                write!(f, "MessageToModel::FetchGhosts")
            }
            MessageToModel::FetchDropDownValues(constraint_type) => {
                write!(f, "MessageToModel::FetchDropDownValues({constraint_type})")
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Progress {
    None,
    Started,
    IslandOffsets,
    Alliances,
    Players,
    Towns,
    Islands,
}

#[derive(Debug)]
pub struct Server {
    pub id: String,
}
