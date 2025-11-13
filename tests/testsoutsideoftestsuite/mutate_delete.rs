use milvus::tests_common::*;
use milvus::{
    client::Client, data::FieldColumn, error::Result, mutate::DeleteOptions,
    schema::CollectionSchema,
};

async fn insert_data(
    client: &Client,
    collection: &CollectionSchema,
    count: i64,
) -> Result<Vec<i64>> {
    let ids = (0..count).collect::<Vec<_>>();
    let vectors = (0..count * DEFAULT_DIM)
        .map(|i| i as f32)
        .collect::<Vec<_>>();

    let fields = vec![
        FieldColumn::new(collection.get_field("id").unwrap(), ids.clone()),
        FieldColumn::new(collection.get_field(DEFAULT_VEC_FIELD).unwrap(), vectors),
    ];

    client.insert(collection.name(), fields, None).await?;

    Ok(ids)
}

#[tokio::test]
async fn test_delete() {
    let (client, collection) = create_test_collection(false).await.unwrap();
    let ids = insert_data(&client, &collection, 10).await.unwrap();

    let result = client
        .delete(
            collection.name().to_string(),
            &DeleteOptions::with_ids(ids.into()),
        )
        .await;

    assert!(result.is_ok());
}
