#![cfg(feature = "milvus-tests")]

use milvus::tests_common::*;
#[tokio::test]
async fn test_get_partition_stats() {
    let (client, collection) = create_test_collection(true).await.unwrap();
    client
        .create_partition(collection.name().to_string(), "test_partition".to_string())
        .await
        .unwrap();

    let result = client
        .get_partition_stats(collection.name().to_string(), "test_partition".to_string())
        .await;

    assert!(result.is_ok());

    let stats = result.unwrap();
    assert!(stats.contains_key("row_count"));
}
