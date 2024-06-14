use hello_world::greeter_server::{Greeter, GreeterServer};
use hello_world::{HelloReply, HelloRequest};
use nym_sdk::mixnet::{
    MixnetClient, MixnetClientBuilder, MixnetClientSender, MixnetMessageSender, Recipient,
    ReconstructedMessage, StoragePaths,
};
use nym_sphinx_anonymous_replies::requests::AnonymousSenderTag;
use prost::bytes::Bytes;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex};
use tokio::task;
use tonic::{transport::Server, Request, Response, Status};
use tokio_util::codec::{BytesCodec, FramedRead, FramedWrite};
use tokio_stream::StreamExt;

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
    let (tx, mut rx) = mpsc::channel(100);

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

    task::spawn(async move {
        loop {
            let mut message: Vec<ReconstructedMessage> = Vec::new();
            while let Some(new_message) = client.wait_for_messages().await {
                if new_message.is_empty() {
                    println!("<< got empty message: most likely SURB request");
                    continue;
                }
                message = new_message;
                break;
            }

            println!("<< received {:?} from mixnet", message);
            tx.send(message).await.unwrap();
        }
    });

    tokio::time::sleep(tokio::time::Duration::from_secs(4)).await; // TODO wait until grpc server is listening (see first bytes in console) instead of just sleeping
    println!("gRPC up: start sending");

    let stream = TcpStream::connect("127.0.0.1:50051").await.unwrap();
    let (read, mut write) = stream.into_split();

    let surbs: Arc<Mutex<Option<AnonymousSenderTag>>> = Arc::new(Mutex::new(None));
    let rx_surbs = surbs.clone();
    let tx_surbs = surbs.clone();

    task::spawn(async move {
        while let Some(messages) = rx.recv().await {
            for message in messages {
                println!("DEBUG start of rx.recv(ReconstructedMessages) loop");
                let mut guard = rx_surbs.lock().await;
                if guard.is_none() {
                    *guard = Some(message.sender_tag.unwrap());
                    println!("<< parsed a sender tag from incoming");
                } else {
                    println!("DEBUG could not find sender tag in incoming")
                }

                println!("<< mixnet message: {message:?}");
 
                write.write_all(&message.message).await.unwrap();
            }
        }
    });

    task::spawn(async move {
        let encoder: BytesCodec = BytesCodec::new();
        let mut reader = FramedRead::new(read, encoder);

        let guard = tx_surbs.lock().await;
        println!("surbs parsed: {guard:#?}");

        while let Some(bytes) = reader.next().await {
            println!(">> bytes from reader.next(): {bytes:?}");
            if let Some(address) = guard.clone() {
                println!(">> sending {:?} as reply to {}", bytes.as_ref().unwrap(), address.clone());
                sender.send_reply(address.clone(), bytes.unwrap()).await.unwrap();
            }
        }

        // let encoder = BytesCodec::new();
        // let mut writer = FramedWrite::new(read, encoder);

        // let guard = tx_surbs.lock().await;

        // while let Some(bytes) = writer.next().await {
        //     if let Some(address) = guard.clone() {
        //         sender.send_reply(address.clone(), bytes.unwrap()).await.unwrap();
        //     }
        // }
        // let mut buf = vec![0; 1024];
        // loop {
        //     let n = read.read(&mut buf).await.unwrap();
        //     if n < 1 {
        //         continue;
        //     }
        //     let mut dst = vec![0u8; n];
        //     dst.clone_from_slice(&buf[0..n]);
        //     println!(">> {} {:?}", n, dst);
        //     let guard = tx_surbs.lock().await;
        //     if let Some(address) = guard.clone() {
        //         sender.send_reply(address.clone(), dst).await.unwrap();
        //     }
        // }
    });

    tokio::signal::ctrl_c().await.unwrap();

    Ok(())
}
