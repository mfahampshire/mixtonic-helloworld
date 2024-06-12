use hello_world::greeter_client::GreeterClient;
use hello_world::HelloRequest;

use nym_sdk::mixnet::{MixnetClient, MixnetMessageSender, Recipient, ReconstructedMessage};
use nym_sphinx_anonymous_replies::requests::AnonymousSenderTag;
use std::str::FromStr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::task;

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, mut rx) = mpsc::channel(100);

    //    nym_bin_common::logging::setup_logging();
    println!("creating client...");
    let mut listen_client = MixnetClient::connect_new().await.unwrap();
    let client_addr = listen_client.nym_address().clone();
    let sender = listen_client.split_sender();
    println!("client created: {}", &client_addr);

    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    task::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 1024];
        loop {
            let n = socket.read(&mut buf).await.unwrap();
            println!(">> socket read {} bytes", n);
            let mut dst = vec![0u8; n];
            dst.clone_from_slice(&buf[0..n]);
            tx.send(dst).await.unwrap();
        }
    });

    task::spawn(async move {
        if rx.is_empty() {
            println!("nothing in rx");
        } else {
            println!("something in rx chann");
        }
        while let Some(buf) = rx.recv().await {
            println!(">> received: {:?} on socket, sending to mixnet", buf);
            sender
                .send_plain_message(Recipient::from_str("rFEtFii4qk2TMdG9B9feL5YtQaMfMgC6vipsAuR8DgB.43dcDX6Urco4XMZgLbMCBf54GGSgHokNbG1MMQRZudEu@CHaxTLDqQ42M5vwErRABmhiwW9i4vxfUayhjaM92ytRv").unwrap(), buf)
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
                println!("{}", String::from_utf8_lossy(&new_message[0].message))
            }
        }
    });

    // start gRPC and start listening on local 8080
    let mut client = GreeterClient::connect("http://127.0.0.1:8080")
        .await
        .unwrap();
    println!("{client:#?}");

    let request = tonic::Request::new(HelloRequest {
        name: "Tonic".into(),
    });
    println!(">> request: {request:#?}");

    let response = client.say_hello(request).await.unwrap();
    println!("<< response: {response:#?}");

    // keep looping so it doesnt die
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}
