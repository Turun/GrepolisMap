use core::fmt;
use std::{collections::HashSet, path::PathBuf, sync::Arc};

use crate::emptyconstraint::EmptyConstraint;
use crate::emptyselection::EmptyTownSelection;
use crate::town::Town;
use crate::view::preferences::CacheSize;

/// This is a file for the messages passed between the view and the presenter.
/// message passing communication allows them to be on separate threads. Also it's good code hygene

pub enum PresenterReady {
    AlwaysHasBeen,
    WaitingForAPI,
    NewlyReady,
}

#[allow(clippy::module_name_repetitions)]
pub enum MessageToServer {
    LoadServer(String),
    StoredConfig(String),
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone)]
pub enum MessageToView {
    Loading(Progress),
    GotServer,
    AllTowns(Arc<Vec<Town>>),
    GhostTowns(Arc<Vec<Town>>),
    TownListForSelection(EmptyTownSelection, Arc<Vec<Town>>),
    ValueListForConstraint(EmptyConstraint, EmptyTownSelection, Arc<Vec<String>>),
    BackendCrashed(String),
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
        }
    }
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone)]
pub enum MessageToModel {
    FetchAll,
    FetchGhosts,
    FetchTowns(
        EmptyTownSelection,
        HashSet<EmptyConstraint>,
        Vec<EmptyTownSelection>,
    ),
    LoadDataFromFile(PathBuf, egui::Context),
    MaxCacheSize(CacheSize),
}

impl fmt::Display for MessageToModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageToModel::FetchTowns(selection, constraints, selections) => {
                write!(
                    f,
                    "MessageToModel::FetchTowns({selection}, {} Constraints currently edited, {} total selections)",
                    constraints.len(),
                    selections.len()
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
            MessageToModel::MaxCacheSize(x) => {
                write!(f, "MessageToModel::MaxCacheSize({})", x.to_string())
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Progress {
    None,
    BackendCrashed(String),
    Fetching,
    LoadingFile,
}

#[derive(Debug, Clone)]
pub struct Server {
    pub id: String,
}
