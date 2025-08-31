use anyhow::anyhow;
use eframe::epaint::ahash::HashMap;

use crate::emptyconstraint::EmptyConstraint;
use crate::emptyselection::EmptyTownSelection;
use crate::model::database::DataTable;
use crate::model::{APIResponse, Model};
use crate::town::Town;
use crate::view::preferences::CacheSize;
use std::sync::{Arc, Mutex};

#[cfg(not(target_arch = "wasm32"))]
use crate::storage::{self, SavedDB};

#[allow(clippy::module_name_repetitions)]
pub enum PresenterReady {
    AlwaysHasBeen,
    WaitingForAPI,
    NewlyReady,
}

pub struct Presenter {
    model: Model,
    max_cache_size: CacheSize,
}

impl Default for Presenter {
    fn default() -> Self {
        Self {
            model: Model::Uninitialized(Arc::new(Mutex::new(APIResponse::new(String::new())))),
            max_cache_size: CacheSize::Normal,
        }
    }
}

impl Presenter {
    /// Return all possible selection names that can be used in the `DropDownValues` for the
    /// Constraint. Returns None if the Constraint is not a IN/NOT IN type of constraint. In which
    /// case it is up to the caller to determine which drop down values are appropriate.
    fn possible_ddv_selections_or(
        constraint: &EmptyConstraint,
        selection: &EmptyTownSelection,
        all_selections: &[EmptyTownSelection],
    ) -> Option<Arc<Vec<String>>> {
        constraint
            .referenced_selection()
            .map(|_referenced_selection| {
                Arc::new(
                    all_selections
                        .iter()
                        .map(|s| s.name.clone())
                        .filter(|name| name != &selection.name)
                        .filter(|name| {
                            let mut test_selection = selection.clone();
                            test_selection.constraints.push(EmptyConstraint {
                                constraint_type: crate::constraint::ConstraintType::PlayerName,
                                comparator: crate::constraint::Comparator::InSelection,
                                value: name.clone(),
                            });
                            !test_selection.contains_circular_reference(all_selections)
                        })
                        .collect(),
                )
            })
    }

    /// triggers the server loading, which is handled asynchronously
    /// This is deliberately its own method, because the self.model = `Model::Uninit` needs to be triggered before the
    /// normal message processing.
    pub fn load_server(&mut self, server: String) {
        let api_response = Arc::new(Mutex::new(APIResponse::new(server)));
        self.model = Model::Uninitialized(Arc::clone(&api_response));
        DataTable::get_api_results(&Arc::clone(&api_response));
    }

    /// triggers the server loading, which is handled asynchronously
    /// This is deliberately its own method, because the self.model = `Model::Uninit` needs to be triggered before the
    /// normal message processing.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_server_from_file(&mut self, saved_db: SavedDB) {
        let api_response = Arc::new(Mutex::new(APIResponse::new(String::new())));
        self.model = Model::Uninitialized(Arc::clone(&api_response));
        APIResponse::load_from_file(saved_db, api_response);
    }

    /// return a list of all towns in the current model with no constraints applied.
    pub fn get_all_towns(&mut self) -> Arc<Vec<Town>> {
        self.model.get_all_towns()
    }

    /// return a list of all ghost towns in the current model
    pub fn get_ghost_towns(&mut self) -> Arc<Vec<Town>> {
        self.model.get_ghost_towns()
    }

    /// return a list of all the towns that match a given selection with all its constraints.
    pub fn towns_for_selection(
        &mut self,
        selection: &EmptyTownSelection,
        all_selections: &[EmptyTownSelection],
    ) -> anyhow::Result<Arc<Vec<Town>>> {
        let filled_constraints: Vec<EmptyConstraint> = selection
            .constraints
            .iter()
            .filter(|c| !c.value.is_empty())
            .cloned()
            .collect();

        return self.model.get_towns_for_constraints(
            selection,
            &filled_constraints,
            all_selections,
        );
    }

    pub fn drop_down_values_for_constraint(
        &mut self,
        constraint: &EmptyConstraint,
        selection: &EmptyTownSelection,
        all_selections: &[EmptyTownSelection],
    ) -> anyhow::Result<Arc<Vec<String>>> {
        // a list of filled constraints. For each one, filter the ddv list by all _other_ filled constratins
        let constraints_filled: Vec<EmptyConstraint> = selection
            .constraints
            .iter()
            .filter(|c| !c.value.is_empty())
            .cloned()
            .collect();

        #[allow(clippy::redundant_else)]
        if constraint.value.is_empty() {
            // drop down value for an empty constraint, filter the ddv list by all filled constratins
            let possible_ddv =
                Self::possible_ddv_selections_or(constraint, selection, all_selections)
                    .ok_or(Err(0))
                    .or_else(|_error_value: Result<Arc<Vec<String>>, i32>| {
                        self.model.get_names_for_constraint_with_constraints(
                            selection,
                            constraint.constraint_type,
                            &constraints_filled,
                            all_selections,
                        )
                    });
            return possible_ddv;
        } else {
            // drop down values for a filled constraint, filter the ddv list by all _other_ filled constratins
            let mut other_constraints = constraints_filled.clone();
            let index = other_constraints
                .iter()
                .position(|x| x == constraint)
                .unwrap_or_else(||
                    panic!("The constraint passed to Presenter::drop_down_values_for_constraint() ({constraint:?}) is not part of the selection that is passed in the same method call ({selection:?})")
                );
            let _this_constraint = other_constraints.swap_remove(index);

            let possible_ddv =
                Self::possible_ddv_selections_or(constraint, selection, all_selections)
                    .ok_or(Err(0))
                    .or_else(|_error_value: Result<Arc<Vec<String>>, i32>| {
                        self.model.get_names_for_constraint_with_constraints(
                            selection,
                            constraint.constraint_type,
                            &other_constraints,
                            all_selections,
                        )
                    });

            return possible_ddv;
        }
    }

    /// returns how many of the api requests already completed. i.e. 1/4 -> 0.25
    /// if model is not in the loading state we return a flat 1.0
    pub fn loading_progress(&self) -> f32 {
        match &self.model {
            Model::Uninitialized(arc) => f32::from(arc.lock().unwrap().count_completed()) / 4.0,
            Model::Loaded { .. } => 1.0,
        }
    }

    /// returns Some(true) if the model is initialized and the presenter can start answering requests.
    /// returns None if the backend crashed trying to parse the complete api response.
    /// returns Some(false) if the api data is still being fetched.
    pub fn ready_for_requests(&mut self) -> anyhow::Result<PresenterReady> {
        match &self.model {
            Model::Uninitialized(api_response) => {
                let api_response = api_response.lock().unwrap().clone();
                if !api_response.is_complete() {
                    return Ok(PresenterReady::WaitingForAPI);
                }

                #[cfg(not(target_arch = "wasm32"))]
                api_response.save_to_file();

                let db = DataTable::create_for_world(api_response);
                self.model = Model::Loaded {
                    db,
                    cache_strings: HashMap::default(),
                    cache_towns: HashMap::default(),
                };
                return Ok(PresenterReady::NewlyReady);
            }
            Model::Loaded { .. } => return Ok(PresenterReady::AlwaysHasBeen),
        }
    }

    pub fn set_max_cache_size(&mut self, cache_size: CacheSize) {
        self.max_cache_size = cache_size;
    }

    /// age the cache of the model by one, slowly forgetting the responses to old requests.
    pub fn age_cache(&mut self) {
        self.model.age_cache(self.max_cache_size.value());
    }
}
