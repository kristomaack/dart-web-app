use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Player {
    /// Stable identity, so a backend can later link players across matches.
    pub id: String,
    pub name: String,
}
