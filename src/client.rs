use hello_world::greeter_client::GreeterClient;
use hello_world::HelloRequest;
use nym_sdk::mixnet::{MixnetClient, MixnetMessageSender, Recipient};
use std::env;
use std::str::FromStr;
use tokio::net::TcpListener;
use tokio::task;
use tokio_stream::StreamExt;
use tokio_util::codec::{BytesCodec, FramedRead};

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let server_addr =
        Recipient::from_str(&args.get(1).expect("missing server nym address")).unwrap();

    //    nym_bin_common::logging::setup_logging();
    println!("creating client...");
    let mut listen_client = MixnetClient::connect_new().await.unwrap();
    let client_addr = listen_client.nym_address().clone();
    let sender = listen_client.split_sender();
    println!("client created: {}", &client_addr);

    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    task::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let codec = BytesCodec::new();
        let mut decoder = FramedRead::new(&mut socket, codec);
        while let Some(bytes) = decoder.next().await {
            println!(">> sending {:?} to {server_addr}", bytes.as_ref().unwrap());
            sender
                .send_plain_message(server_addr, bytes.unwrap())
                .await
                .unwrap();
        }
    });

    task::spawn(async move {
        while let Some(new_message) = listen_client.wait_for_messages().await {
            if new_message.is_empty() {
                println!(
                    "<< mixnet listener got an empty message, this is probably a SURB request"
                );
                continue;
            } else {
                println!("<< mixnet listener got message from mixnet: {new_message:?}");
                println!("{}", String::from_utf8_lossy(&new_message[0].message));
            }
        }
    });

    // start gRPC and start listening on local 8080
    // TODO work out a way of sending this to split rd,wr - doesn't like the rd on its own, need to implement some sort of addr trait
    // maybe I can just to do the same stream split as in the server?
    let mut client = GreeterClient::connect("http://127.0.0.1:8080")
        .await
        .unwrap();
    println!("{client:#?}");

    let request = tonic::Request::new(HelloRequest {
        name: "NymTonic".into(),
    });
    println!(">> request: {request:#?}");

    let response = client.say_hello(request).await.unwrap();
    println!("<< response: {response:#?}");

    // keep looping so it doesnt die
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}
