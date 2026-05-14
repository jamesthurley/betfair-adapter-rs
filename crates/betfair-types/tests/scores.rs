#[test]
fn parse_scores_fixtures() {
    use betfair_types::types::scores_aping::list_scores;

    for path in [
        "./tests/resources/scores_list_scores_finished.json",
        "./tests/resources/scores_list_scores_in_progress.json",
        "./tests/resources/scores_list_scores_pending.json",
    ] {
        parse_json_rpc_result::<list_scores::ReturnType>(path);
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
        parse_json_rpc_result::<list_incidents::ReturnType>(path);
    }
}

#[test]
fn parse_available_events_fixture() {
    use betfair_types::types::scores_aping::list_available_events;

    parse_json_rpc_result::<list_available_events::ReturnType>(
        "./tests/resources/scores_list_available_events.json",
    );
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

fn parse_json_rpc_result<T>(path: &str)
where
    T: serde::de::DeserializeOwned + IntoIterator,
{
    use json_rpc_types::Response;

    let data = std::fs::read_to_string(path).unwrap();
    let responses = serde_json::from_str::<Vec<Response<T, serde_json::Value>>>(&data).unwrap();
    let result = responses.into_iter().next().unwrap().payload.unwrap();

    assert!(result.into_iter().next().is_some());
}
