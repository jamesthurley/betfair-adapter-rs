//! Helpers for working with Scores API dynamic values.

use std::collections::HashMap;
use std::sync::Arc;

use crate::types::scores_aping::{Incident, Score, TennisIncidentValues, TennisScoreValues};

/// The raw dynamic values map returned in Scores API score and incident updates.
pub type ScoresValuesMap = HashMap<Arc<String>, Arc<String>>;

macro_rules! extract_tennis_values {
    ($values:expr, $type:ident { $($field:ident: $key:literal),+ $(,)? }) => {{
        let mut typed = $type {
            $($field: None,)+
        };

        for (key, value) in $values {
            match key.as_str() {
                $($key => typed.$field = Some(Arc::clone(value)),)+
                _ => {}
            }
        }

        typed
    }};
}

/// Deserializes a Scores API dynamic values map into a documented values view.
pub trait ScoresValuesExt {
    /// Deserialize this dynamic values map as tennis score values.
    fn tennis_score_values(&self) -> serde_json::Result<TennisScoreValues>;

    /// Deserialize this dynamic values map as tennis incident values.
    fn tennis_incident_values(&self) -> serde_json::Result<TennisIncidentValues>;
}

impl ScoresValuesExt for ScoresValuesMap {
    fn tennis_score_values(&self) -> serde_json::Result<TennisScoreValues> {
        Ok(TennisScoreValues::from_scores_values(self))
    }

    fn tennis_incident_values(&self) -> serde_json::Result<TennisIncidentValues> {
        Ok(TennisIncidentValues::from_scores_values(self))
    }
}

impl TryFrom<&ScoresValuesMap> for TennisScoreValues {
    type Error = serde_json::Error;

    fn try_from(values: &ScoresValuesMap) -> Result<Self, Self::Error> {
        Ok(Self::from_scores_values(values))
    }
}

impl TryFrom<&ScoresValuesMap> for TennisIncidentValues {
    type Error = serde_json::Error;

    fn try_from(values: &ScoresValuesMap) -> Result<Self, Self::Error> {
        Ok(Self::from_scores_values(values))
    }
}

impl TennisScoreValues {
    /// Extract known tennis score keys from a Scores API dynamic values map.
    pub fn from_scores_values(values: &ScoresValuesMap) -> Self {
        extract_tennis_values!(
            values,
            TennisScoreValues {
                player1: "player1",
                player2: "player2",
                best_of_sets: "bestOfSets",
                player1_sets_won: "player1SetsWon",
                player2_sets_won: "player2SetsWon",
                player1_games_won: "player1GamesWon",
                player2_games_won: "player2GamesWon",
                player1_points_won: "player1PointsWon",
                player2_points_won: "player2PointsWon",
                player1_set_scores: "player1SetScores",
                player2_set_scores: "player2SetScores",
                is_live: "isLive",
                event_type_status: "eventTypeStatus",
                start_date: "startDate",
                tie_break_type: "tieBreakType",
            }
        )
    }
}

impl TennisIncidentValues {
    /// Extract known tennis incident keys from a Scores API dynamic values map.
    pub fn from_scores_values(values: &ScoresValuesMap) -> Self {
        extract_tennis_values!(
            values,
            TennisIncidentValues {
                r_type: "type",
                matchid: "matchid",
                id: "id",
                packet_id: "packet_id",
                utc_timestamp: "utc_timestamp",
                doubles: "doubles",
                nb_set: "nb_set",
                serve_first: "serve_first",
                on_court: "on_court",
                num_court: "num_court",
                type_tie_break: "type_tie_break",
                scoring_type: "scoring_type",
                toss_chooser: "toss_chooser",
                toss_choice: "tossChoice",
                toss_winner: "tossWinner",
                first_to_serve: "firstToServe",
                match_state: "match_state",
                finished: "finished",
                surname1: "surname1",
                surname2: "surname2",
                surname1a: "surname1a",
                surname2a: "surname2a",
                number1: "number1",
                number2: "number2",
                number1a: "number1a",
                number2a: "number2a",
                country1: "country1",
                country2: "country2",
                country1a: "country1a",
                country2a: "country2a",
                umpire: "umpire",
                umpire_code: "umpire_code",
                umpire_country: "umpire_country",
                undo: "undo",
                last_stroke_id: "lastStrokeId",
                server: "server",
                stroke_index: "strokeIndex",
                stroke_type: "strokeType",
                stroke_id: "strokeId",
                utc_time: "utcTime",
                current_set: "currentSet",
                current_game: "currentGame",
                player1: "player1",
                player2: "player2",
                player1_sets_won: "player1SetsWon",
                player2_sets_won: "player2SetsWon",
                player1_games_won: "player1GamesWon",
                player2_games_won: "player2GamesWon",
                player1_points_won: "player1PointsWon",
                player2_points_won: "player2PointsWon",
                player1_set_scores: "player1SetScores",
                player2_set_scores: "player2SetScores",
                team1_points_after: "team1PointsAfter",
                team2_points_after: "team2PointsAfter",
                won_by: "wonBy",
                match_time: "matchTime",
                is_live: "isLive",
                event_type_status: "eventTypeStatus",
                start_date: "startDate",
            }
        )
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
