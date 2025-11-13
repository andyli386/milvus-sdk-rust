use milvus::tests_common::*;
#[tokio::test]
async fn test_release_collection() {
    let (client, collection) = create_test_collection(true).await.unwrap();

    let result = client.release_collection(collection.name()).await;

    assert!(result.is_ok());
}
