//! Seule surface IPC exposée au frontend. Pas de logique métier ici — agrégation/délégation
//! uniquement (ARCHITECTURE.md). Les données servies sont mockées (EPICs 1 à 7 à venir).

pub mod alerts;
pub mod dashboard;
pub mod destinations;
pub mod flows;
pub mod killswitch;
pub(crate) mod mock_data;
pub(crate) mod mock_flows;
pub mod processes;
pub mod search;
pub mod settings;
pub mod types;
