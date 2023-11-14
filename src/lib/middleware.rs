pub mod middleware {
    use std::sync::Arc;
    use tonic::{Request, Response, Status};
    use ring::digest::{Context, SHA256};
    use data_encoding::{HEXLOWER};
    use tokio::sync::Mutex;
    use crate::proto::middleware::{PutMiddlewareRequest, PutMiddlewareResponse};
    use crate::proto::middleware::middleware_server::Middleware;

    #[derive(Debug, Default)]
    pub struct MyMiddleware {
    }

    #[tonic::async_trait]
    impl Middleware for MyMiddleware {
        async fn put(&self, request: Request<PutMiddlewareRequest>) -> Result<Response<PutMiddlewareResponse>, Status> {
            // let mut client = DataCapsuleClient::connect("http://[::1]:50051").await?;
            // let request = tonic::Request::new(PutRequest {
            //     block_hash: hash.to_string()
            // });
            // let response = client.get(request).await?;
            todo!()
        }
    }
}

