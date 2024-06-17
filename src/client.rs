use hello_world::greeter_client::GreeterClient;
use hello_world::HelloRequest;

use nym_sdk::mixnet::{
    MixnetClient, MixnetMessageSender, Recipient,
};
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
    // old code: left here to remind what was here
    // let (tx, mut rx) = mpsc::channel(100);

    let server_addr: Recipient = Recipient::from_str("Hb7bWBHUpyiqus4mWsHooBVD545u4u27VsiVcR645GC5.82JfXAEWKbujJv5KXT9HUc9eCHvDp9envWXhx46b9sBT@3RGUju1J3HB6qV4zwPdXTUxZTdFd6RPkpzwkrteici9b").unwrap();

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
            // tx.send(bytes.unwrap()).await.unwrap();
            println!(">> sending {:?} to {server_addr}", bytes.as_ref().unwrap());
            sender
                .send_plain_message(server_addr, bytes.unwrap())
                .await
                .unwrap();
        }
        // old code: left here to remind what was here
        // let mut buf = vec![0; 1024];
        // loop {
        //     let n = socket.read(&mut buf).await.unwrap();
        //     println!(">> socket read {} bytes", n);
        //     let mut dst = vec![0u8; n];
        //     dst.clone_from_slice(&buf[0..n]);
        //     tx.send(dst).await.unwrap();
        // }
    });

    // old code: took out a step and just send message in task above now we have codec
    // task::spawn(async move {
    //     if rx.is_empty() {
    //         println!("nothing in rx");
    //     } else {
    //         println!("something in rx chann");
    //     }
    //     while let Some(buf) = rx.recv().await {
    //         println!(">> received: {:?} on socket, sending to mixnet", buf);
    //         sender.send_plain_message(server_addr, buf).await.unwrap();
    //     }
    // });

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
