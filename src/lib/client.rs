use crate::proto::data_capsule::data_capsule_client::DataCapsuleClient;
use crate::proto::data_capsule::{GetRequest, GetResponse};

#[tokio::main]
pub async fn get(hash: &str) -> Result<GetResponse, Box<dyn std::error::Error>> {
    let mut client = DataCapsuleClient::connect("http://[::1]:50051").await?;
    let request = tonic::Request::new(GetRequest {
        block_hash: hash.to_string()
    });
    let response = client.get(request).await?;

    Ok(response.get_ref().clone())
}

// pub fn getSize(hash: &str) -> usize {
//     let response = get(hash);
//     let size = response.unwrap().block.unwrap().data.len();
//     // println!("{}", size);
//     return size.clone();
// }
