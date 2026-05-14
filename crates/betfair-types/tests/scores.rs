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

#[test]
#[ignore = "performance smoke test; run explicitly with --ignored --nocapture"]
fn measure_tennis_values_conversion_performance() {
    use std::hint::black_box;
    use std::time::Instant;

    use betfair_types::scores::ScoresValuesExt as _;
    use betfair_types::types::scores_aping::{list_incidents, list_scores};

    const ITERATIONS: usize = 100;

    let incidents_data =
        std::fs::read_to_string("./tests/resources/scores_list_incidents_finished.json").unwrap();
    let scores_data =
        std::fs::read_to_string("./tests/resources/scores_list_scores_in_progress.json").unwrap();

    let parse_incidents_start = Instant::now();
    let mut parsed_incident_maps = 0;
    for _ in 0..ITERATIONS {
        let parsed = parse_json_rpc_data::<list_incidents::ReturnType>(&incidents_data);
        parsed_incident_maps += parsed
            .iter()
            .map(|incidents| incidents.incidents.len())
            .sum::<usize>();
        black_box(&parsed);
    }
    let parse_incidents_elapsed = parse_incidents_start.elapsed();

    let incidents = parse_json_rpc_data::<list_incidents::ReturnType>(&incidents_data);
    let incident_values = incidents
        .iter()
        .flat_map(|incidents| incidents.incidents.values())
        .filter_map(|incident| incident.values.as_ref())
        .collect::<Vec<_>>();

    let convert_incidents_start = Instant::now();
    let mut converted_incidents = 0;
    for _ in 0..ITERATIONS {
        for values in &incident_values {
            let tennis_values = values.tennis_incident_values().unwrap();
            converted_incidents +=
                usize::from(tennis_values.player1.is_some() || tennis_values.stroke_type.is_some());
            black_box(tennis_values);
        }
    }
    let convert_incidents_elapsed = convert_incidents_start.elapsed();

    let parse_scores_start = Instant::now();
    let mut parsed_score_maps = 0;
    for _ in 0..ITERATIONS {
        let parsed = parse_json_rpc_data::<list_scores::ReturnType>(&scores_data);
        parsed_score_maps += parsed.len();
        black_box(&parsed);
    }
    let parse_scores_elapsed = parse_scores_start.elapsed();

    let scores = parse_json_rpc_data::<list_scores::ReturnType>(&scores_data);
    let score_values = scores
        .iter()
        .filter_map(|score| score.values.as_ref())
        .collect::<Vec<_>>();

    let convert_scores_start = Instant::now();
    let mut converted_scores = 0;
    for _ in 0..ITERATIONS {
        for values in &score_values {
            let tennis_values = values.tennis_score_values().unwrap();
            converted_scores += usize::from(tennis_values.player1.is_some());
            black_box(tennis_values);
        }
    }
    let convert_scores_elapsed = convert_scores_start.elapsed();

    println!(
        "\n\
        tennis_values_performance iterations={ITERATIONS}\n\
        listIncidents: parse_total={parse_incidents_elapsed:?} \
        convert_total={convert_incidents_elapsed:?} \
        parsed_maps={parsed_incident_maps} \
        converted_maps={converted_incidents} \
        convert_per_map={:?}\n\
        listScores: parse_total={parse_scores_elapsed:?} \
        convert_total={convert_scores_elapsed:?} \
        parsed_maps={parsed_score_maps} \
        converted_maps={converted_scores} \
        convert_per_map={:?}",
        convert_incidents_elapsed / (incident_values.len() as u32 * ITERATIONS as u32),
        convert_scores_elapsed / (score_values.len() as u32 * ITERATIONS as u32),
    );
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
