use betfair_rpc_server_mock::Server;
use betfair_types::types::scores_aping::{
    EventId, ResponseCode, UpdateKey, UpdateSequence, list_scores,
};
use pretty_assertions::assert_eq;
use serde_json::json;

#[test_log::test(tokio::test)]
async fn list_scores_uses_json_rpc_transport() {
    let server = Server::new().await;

    let response = json!([
        {
            "eventId": "30317485",
            "eventTypeId": "2",
            "updateContext": {
                "eventTime": "09:54:03",
                "lastUpdated": "2021-03-02T09:54:03.000Z",
                "updateSequence": 241,
                "updateType": "score"
            },
            "values": {
                "player1": "7660786",
                "player2": "8505197",
                "eventTypeStatus": "inPlay"
            },
            "eventStatus": "IN_PROGRESS",
            "responseCode": "OK"
        }
    ]);
    server
        .mock_authenticated_rpc_from_json::<list_scores::Parameters>(response)
        .expect(1)
        .mount(&server.bf_api_mock_server)
        .await;

    let client = server.client().await;
    let (client, _) = client.authenticate().await.unwrap();

    let result = client
        .send_request(
            list_scores::Parameters::builder()
                .update_keys(vec![
                    UpdateKey::builder()
                        .event_id(EventId::new("30317485"))
                        .last_update_sequence_processed(UpdateSequence(232))
                        .build(),
                ])
                .build(),
        )
        .await
        .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].response_code, ResponseCode::Ok);
}
