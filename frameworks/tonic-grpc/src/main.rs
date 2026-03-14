use std::path::Path;
use tonic::transport::{Identity, Server, ServerTlsConfig};
use tonic::{Request, Response, Status};

pub mod benchmark {
    tonic::include_proto!("benchmark");
}

use benchmark::benchmark_service_server::{BenchmarkService, BenchmarkServiceServer};
use benchmark::{SumReply, SumRequest};

#[derive(Default)]
pub struct BenchmarkServiceImpl;

#[tonic::async_trait]
impl BenchmarkService for BenchmarkServiceImpl {
    async fn get_sum(
        &self,
        request: Request<SumRequest>,
    ) -> Result<Response<SumReply>, Status> {
        let req = request.into_inner();
        let reply = SumReply {
            result: req.a + req.b,
        };
        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service = BenchmarkServiceServer::new(BenchmarkServiceImpl::default());

    let cert_path = "/certs/server.crt";
    let key_path = "/certs/server.key";
    let has_cert = Path::new(cert_path).exists() && Path::new(key_path).exists();

    if has_cert {
        let cert = tokio::fs::read(cert_path).await?;
        let key = tokio::fs::read(key_path).await?;
        let identity = Identity::from_pem(cert, key);

        let h2c_handle = tokio::spawn({
            let svc = service.clone();
            async move {
                Server::builder()
                    .add_service(svc)
                    .serve("0.0.0.0:8080".parse().unwrap())
                    .await
            }
        });

        println!("Application started.");

        let tls_handle = tokio::spawn(async move {
            Server::builder()
                .tls_config(ServerTlsConfig::new().identity(identity))
                .unwrap()
                .add_service(service)
                .serve("0.0.0.0:8443".parse().unwrap())
                .await
        });

        tokio::try_join!(h2c_handle, tls_handle)?;
    } else {
        println!("Application started.");
        Server::builder()
            .add_service(service)
            .serve("0.0.0.0:8080".parse().unwrap())
            .await?;
    }

    Ok(())
}
