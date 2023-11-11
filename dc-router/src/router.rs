
pub mod router {
    tonic::include_proto!("router");
    use std::collections::HashMap;
    use std::net::SocketAddr;
    use router_server::Router;
    use tonic::{IntoRequest, Request, Response, Status};

    #[derive(Debug, Default)]
    pub struct MyRouter {
        routes: HashMap<Vec<u8>, SocketAddr>
    }

    #[tonic::async_trait]
    impl Router for MyRouter {
        async fn route(
            &self,
            request: Request<RouteRequest>,
        ) -> Result<Response<RouteReply>, Status> {
            println!("Got a routing request: {:?}", request);

            let dest = &self.routes.get(&request.get_ref().destination);
            let reply = RouteReply {
                source: Vec::from([0]),
                data: Vec::new()
            };

            Ok(Response::new(reply))

        }
    }
}

