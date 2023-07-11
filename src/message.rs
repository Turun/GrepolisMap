use core::fmt;
use std::{collections::HashSet, path::PathBuf, sync::Arc};

use crate::{
    storage::SavedDB,
    towns::{Constraint, Town, TownSelection},
};

/// This is a file for the messages passed between the view and the presenter.
/// message passing communication allows them to be on separate threads. Also it's good code hygene

#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub enum MessageToView {
    Loading(Progress),
    GotServer,
    AllTowns(Arc<Vec<Town>>),
    GhostTowns(Arc<Vec<Town>>),
    TownListForSelection(TownSelection, Arc<Vec<Town>>),
    ValueListForConstraint(Constraint, TownSelection, Arc<Vec<String>>),
    BackendCrashed(anyhow::Error),
    FoundSavedDatabases(Vec<SavedDB>),
    RemovedDuplicateFiles(Vec<SavedDB>),
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
                    "MessageToView::ValueListForConstraint({}, {}, {} Values)",
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
            MessageToView::BackendCrashed(err) => {
                write!(f, "MessageToView::BackendCrashed({err:?})")
            }
            MessageToView::FoundSavedDatabases(db_paths) => {
                write!(f, "MessageToView::FoundSavedDatabases({})", db_paths.len())
            }
            MessageToView::RemovedDuplicateFiles(removed_paths) => {
                write!(
                    f,
                    "MessageToView::RemovedDuplicateFiles({})",
                    removed_paths.len()
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
    FetchTowns(TownSelection, HashSet<Constraint>),
    LoadDataFromFile(PathBuf, egui::Context),
    DiscoverSavedDatabases,
}

impl fmt::Display for MessageToModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageToModel::SetServer(server, _frame) => {
                write!(f, "MessageToMode::SetServer({})", server.id)
            }
            MessageToModel::FetchTowns(selection, constraints) => {
                write!(
                    f,
                    "MessageToModel::FetchTowns({selection}, {} Constraints currently edited)",
                    constraints.len()
                )
            }
            MessageToModel::FetchAll => {
                write!(f, "MessageToModel::FetchAll")
            }
            MessageToModel::FetchGhosts => {
                write!(f, "MessageToModel::FetchGhosts")
            }
            MessageToModel::LoadDataFromFile(path, _ctx) => {
                write!(f, "MessageToModel::LoadDataFromFile({path:?})")
            }
            MessageToModel::DiscoverSavedDatabases => {
                write!(f, "MessageToModel::DiscoverSavedDatabases")
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Progress {
    None,
    BackendCrashed,
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
