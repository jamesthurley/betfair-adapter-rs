#[test]
fn parse_score_fixture() {
    use betfair_types::types::scores_aping::list_scores;

    let data = std::fs::read_to_string("./tests/resources/score.json").unwrap();
    let result = serde_json::from_str::<list_scores::ReturnType>(&data).unwrap();

    assert_eq!(result.len(), 1);
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
