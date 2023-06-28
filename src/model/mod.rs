//the entry point for model

pub mod download;

pub enum Model {
    Uninitialized,
    Loaded { db: download::Database },
}
