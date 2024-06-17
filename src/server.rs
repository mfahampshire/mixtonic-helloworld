use hello_world::greeter_server::{Greeter, GreeterServer};
use hello_world::{HelloReply, HelloRequest};
use nym_sdk::mixnet::{
    MixnetClientBuilder, MixnetMessageSender, ReconstructedMessage, StoragePaths,
};
use nym_sphinx_anonymous_replies::requests::AnonymousSenderTag;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::task;
use tokio_stream::StreamExt;
use tokio_util::codec::{BytesCodec, FramedRead};
use tonic::{transport::Server, Request, Response, Status};

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

#[derive(Debug, Default)]
pub struct MyGreeter {}

#[tonic::async_trait]
impl Greeter for MyGreeter {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        println!("<< Got a request: {:?}", request);

        let reply = HelloReply {
            message: format!("Hello {}!", request.into_inner().name),
        };

        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // let (tx, mut rx) = mpsc::channel(100);

    task::spawn(async move {
        let addr = "127.0.0.1:50051".parse().unwrap();
        let greeter = MyGreeter::default();
        Server::builder()
            .add_service(GreeterServer::new(greeter))
            .serve(addr)
            .await
            .unwrap();
    });

    //    nym_bin_common::logging::setup_logging();
    println!("creating client...");
    let config_dir = PathBuf::from("/tmp/mixnet-client");
    let storage_paths = StoragePaths::new_from_dir(&config_dir).unwrap();
    let client = MixnetClientBuilder::new_with_default_storage(storage_paths)
        .await
        .unwrap()
        .build()
        .unwrap();

    let mut client = client.connect_to_mixnet().await.unwrap();

    let client_addr = client.nym_address().clone();
    let sender = client.split_sender();
    println!("client created: {}", &client_addr);

    let stream = TcpStream::connect("127.0.0.1:50051").await.unwrap();
    let (read, mut write) = stream.into_split();

    let surbs: Arc<Mutex<Option<AnonymousSenderTag>>> = Arc::new(Mutex::new(None));
    let rx_surbs = surbs.clone();
    let tx_surbs = surbs.clone();

    tokio::time::sleep(tokio::time::Duration::from_secs(4)).await; // TODO wait until grpc server is listening (see first bytes in console) instead of just sleeping
    println!("gRPC up: start sending");

    task::spawn(async move {
        loop {
            let mut messages: Vec<ReconstructedMessage> = Vec::new();
            while let Some(new_messages) = client.wait_for_messages().await {
                if new_messages.is_empty() {
                    println!("<< got empty message: most likely SURB request");
                    continue;
                }
                messages = new_messages;
                break;
            }

            println!("<< received {:?} from mixnet", messages);
            // tx.send(message).await.unwrap();
            for message in messages {
                println!("<< incoming sender_tag: {:?}", message.sender_tag);
                {
                    // this panics when task below aquires the tx_surbs_guard - why?
                    let mut rx_surbs_guard = rx_surbs.try_lock().unwrap();
                    if rx_surbs_guard.is_none() {
                        *rx_surbs_guard = Some(message.sender_tag.unwrap());
                        println!("<< parsed and set a sender tag from incoming");
                    }
                }
                println!("after rx_surbs_guard lock and set scope");
                write.write_all(&message.message).await.unwrap();
            }
        }
    });

    task::spawn(async move {
        let encoder: BytesCodec = BytesCodec::new();
        let mut reader = FramedRead::new(read, encoder);

        {
            while let Some(bytes) = reader.next().await {
                println!(
                    ">> read {:?} bytes from reader.next(): {:?}",
                    bytes.as_ref().unwrap().len(),
                    bytes.as_ref().unwrap()
                );
                if let Some(address) = tx_surbs.lock().await.clone() {
                    println!(
                        ">> sending {:?} as reply to {}",
                        bytes.as_ref().unwrap(),
                        address.clone()
                    );
                    sender
                        .send_reply(address.clone(), bytes.unwrap())
                        .await
                        .unwrap();
                }
            }
        }
    });

    tokio::signal::ctrl_c().await.unwrap();
    Ok(())
}
