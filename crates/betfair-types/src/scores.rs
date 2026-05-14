//! Helpers for working with Scores API dynamic values.

use std::collections::HashMap;
use std::sync::Arc;

use serde::de::DeserializeOwned;

use crate::types::scores_aping::{Incident, Score, TennisIncidentValues, TennisScoreValues};

/// The raw dynamic values map returned in Scores API score and incident updates.
pub type ScoresValuesMap = HashMap<Arc<String>, Arc<String>>;

/// Deserializes a Scores API dynamic values map into a documented values view.
pub trait ScoresValuesExt {
    /// Deserialize this dynamic values map into any compatible generated values type.
    fn deserialize_values<T>(&self) -> serde_json::Result<T>
    where
        T: DeserializeOwned;

    /// Deserialize this dynamic values map as tennis score values.
    fn tennis_score_values(&self) -> serde_json::Result<TennisScoreValues>;

    /// Deserialize this dynamic values map as tennis incident values.
    fn tennis_incident_values(&self) -> serde_json::Result<TennisIncidentValues>;
}

impl ScoresValuesExt for ScoresValuesMap {
    fn deserialize_values<T>(&self) -> serde_json::Result<T>
    where
        T: DeserializeOwned,
    {
        deserialize_values_map(self)
    }

    fn tennis_score_values(&self) -> serde_json::Result<TennisScoreValues> {
        deserialize_values_map(self)
    }

    fn tennis_incident_values(&self) -> serde_json::Result<TennisIncidentValues> {
        deserialize_values_map(self)
    }
}

impl TryFrom<&ScoresValuesMap> for TennisScoreValues {
    type Error = serde_json::Error;

    fn try_from(values: &ScoresValuesMap) -> Result<Self, Self::Error> {
        deserialize_values_map(values)
    }
}

impl TryFrom<&ScoresValuesMap> for TennisIncidentValues {
    type Error = serde_json::Error;

    fn try_from(values: &ScoresValuesMap) -> Result<Self, Self::Error> {
        deserialize_values_map(values)
    }
}

impl Score {
    /// Deserialize this score's dynamic values as tennis score values.
    pub fn tennis_values(&self) -> serde_json::Result<Option<TennisScoreValues>> {
        self.values
            .as_ref()
            .map(TennisScoreValues::try_from)
            .transpose()
    }
}

impl Incident {
    /// Deserialize this incident's dynamic values as tennis incident values.
    pub fn tennis_values(&self) -> serde_json::Result<Option<TennisIncidentValues>> {
        self.values
            .as_ref()
            .map(TennisIncidentValues::try_from)
            .transpose()
    }
}

fn deserialize_values_map<T>(values: &ScoresValuesMap) -> serde_json::Result<T>
where
    T: DeserializeOwned,
{
    serde_json::from_value(serde_json::to_value(values)?)
}
