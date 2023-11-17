pub mod middleware {
    use tonic::{Request, Response, Status};

    use crate::proto::middleware::{PutMiddlewareRequest, PutMiddlewareResponse};
    use crate::proto::middleware::middleware_server::Middleware;

    #[derive(Debug, Default)]
    pub struct MyMiddleware {
    }

    #[tonic::async_trait]
    impl Middleware for MyMiddleware {
        async fn put(&self, _request: Request<PutMiddlewareRequest>) -> Result<Response<PutMiddlewareResponse>, Status> {
            // let mut client = DataCapsuleClient::connect("http://[::1]:50051").await?;
            // let request = tonic::Request::new(PutRequest {
            //     block_hash: hash.to_string()
            // });
            // let response = client.get(request).await?;
            todo!()
        }
    }
}

