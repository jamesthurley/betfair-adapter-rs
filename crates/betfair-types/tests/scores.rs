#[test]
fn parse_scores_fixtures() {
    use betfair_types::types::scores_aping::list_scores;

    for path in [
        "./tests/resources/scores_list_scores_finished.json",
        "./tests/resources/scores_list_scores_in_progress.json",
        "./tests/resources/scores_list_scores_pending.json",
    ] {
        let scores = parse_json_rpc_result::<list_scores::ReturnType>(path);

        assert!(!scores.is_empty(), "{path}");
        let mut converted = 0;
        for score in &scores {
            let Some(values) = score.tennis_values().unwrap() else {
                continue;
            };

            converted += 1;
            assert!(values.player1.is_some(), "{path}");
        }
        assert!(converted > 0, "{path}");
    }
}

#[test]
fn parse_incidents_fixtures() {
    use betfair_types::types::scores_aping::list_incidents;

    for path in [
        "./tests/resources/scores_list_incidents_finished.json",
        "./tests/resources/scores_list_incidents_in_progress.json",
        "./tests/resources/scores_list_incidents_pending.json",
    ] {
        let incidents_by_event = parse_json_rpc_result::<list_incidents::ReturnType>(path);

        assert!(!incidents_by_event.is_empty(), "{path}");
        let mut converted = 0;
        for incident in incidents_by_event
            .iter()
            .flat_map(|incidents| incidents.incidents.values())
        {
            let Some(values) = incident.tennis_values().unwrap() else {
                continue;
            };

            converted += 1;
            assert!(
                values.player1.is_some() || values.stroke_type.is_some(),
                "{path}"
            );
        }
        assert!(converted > 0, "{path}");
    }
}

#[test]
fn parse_available_events_fixture() {
    use betfair_types::types::scores_aping::list_available_events;

    let events = parse_json_rpc_result::<list_available_events::ReturnType>(
        "./tests/resources/scores_list_available_events.json",
    );

    assert!(!events.is_empty());
}

#[test]
fn tennis_values_helpers_match_serde_map_deserialization() {
    use betfair_types::types::scores_aping::{
        TennisIncidentValues, TennisScoreValues, list_incidents, list_scores,
    };

    for path in [
        "./tests/resources/scores_list_scores_finished.json",
        "./tests/resources/scores_list_scores_in_progress.json",
        "./tests/resources/scores_list_scores_pending.json",
    ] {
        let scores = parse_json_rpc_result::<list_scores::ReturnType>(path);

        for score in &scores {
            let Some(values) = score.values.as_ref() else {
                continue;
            };

            let expected: TennisScoreValues =
                serde_json::from_value(serde_json::to_value(values).unwrap()).unwrap();

            assert_eq!(score.tennis_values().unwrap().unwrap(), expected, "{path}");
        }
    }

    for path in [
        "./tests/resources/scores_list_incidents_finished.json",
        "./tests/resources/scores_list_incidents_in_progress.json",
        "./tests/resources/scores_list_incidents_pending.json",
    ] {
        let incidents_by_event = parse_json_rpc_result::<list_incidents::ReturnType>(path);

        for incident in incidents_by_event
            .iter()
            .flat_map(|incidents| incidents.incidents.values())
        {
            let Some(values) = incident.values.as_ref() else {
                continue;
            };

            let expected: TennisIncidentValues =
                serde_json::from_value(serde_json::to_value(values).unwrap()).unwrap();

            assert_eq!(
                incident.tennis_values().unwrap().unwrap(),
                expected,
                "{path}"
            );
        }
    }
}

#[test]
fn generated_scores_requests_are_json_rpc() {
    use betfair_types::types::scores_aping::list_scores;
    use betfair_types::types::{BetfairRpcRequest, BetfairRpcTransport};

    assert_eq!(
        list_scores::Parameters::method(),
        "ScoresAPING/v1.0/listScores"
    );
    assert_eq!(
        list_scores::Parameters::transport(),
        BetfairRpcTransport::JsonRpc
    );
    assert_eq!(
        list_scores::Parameters::endpoint_path(),
        "/exchange/scores/json-rpc/v1"
    );
}

#[test]
fn stroke_type_accepts_unknown_values() {
    use betfair_types::types::scores_aping::TennisIncidentValues;

    let incident_values: TennisIncidentValues =
        serde_json::from_str(r#"{"strokeType":"not_in_the_documentation"}"#).unwrap();

    assert!(incident_values.stroke_type.is_some());
}

fn parse_json_rpc_result<T>(path: &str) -> T
where
    T: serde::de::DeserializeOwned,
{
    let data = std::fs::read_to_string(path).unwrap();
    parse_json_rpc_data(&data)
}

fn parse_json_rpc_data<T>(data: &str) -> T
where
    T: serde::de::DeserializeOwned,
{
    use json_rpc_types::Response;

    let responses = serde_json::from_str::<Vec<Response<T, serde_json::Value>>>(&data).unwrap();
    responses.into_iter().next().unwrap().payload.unwrap()
}
