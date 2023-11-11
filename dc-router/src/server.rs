pub mod server {
    tonic::include_proto!("data_capsule");
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use data_capsule_server::DataCapsule;
    use tonic::{Request, Response, Status};
    use ring::digest::{Context, Digest, SHA256};

    #[derive(Debug, Default)]
    pub struct MyDataCapsule {
        data: DataCapsuleServerData
    }

    fn calculate_hash<T: Hash>(t: &T) -> u64 {
        // let mut s = Sha256
        // t.hash(&mut s);
        return 0;
    }

    #[tonic::async_trait]
    impl DataCapsule for MyDataCapsule {
        async fn get(&self, request: Request<GetRequest>) -> Result<Response<GetResponse>, Status> {
            println!("Got a get request: {:?}", request);
            let reply = GetResponse {
                block: self.data.content.get(&request.get_ref().block_hash).cloned()
            };
            Ok(Response::new(reply))
        }

        async fn put(&self, request: Request<PutRequest>) -> Result<Response<PutResponse>, Status> {
            println!("Got a put request: {:?}", request);
            // request.get_ref().block.unwrap().hash()

            let reply = PutResponse {
                success: true
            };
            Ok(Response::new(reply))
        }

        async fn leafs(&self, request: Request<LeafsRequest>) -> Result<Response<LeafsResponse>, Status> {
            let reply = LeafsResponse {
                leaf_ids: self.data.leafs.clone()
            };
            Ok(Response::new(reply))
        }



    }
}

