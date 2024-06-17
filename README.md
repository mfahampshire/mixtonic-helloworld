# `mixtonic helloworld`

Running the [`tonic` crate HelloWorld example](https://github.com/hyperium/tonic/blob/master/examples/helloworld-tutorial.md) over the mixnet for initial gRPC-over-mixnet example project structure. 

## run
```
# terminal window 1: run this and wait for 27 bytes to be read initially: this is the grpc server 
cargo run --bin helloworld-server

# terminal window 2: 
cargo run --bin helloworld-client
```

## todo
- [ ] sort out non-utf8 noise on return 
- [ ] readme arch diagram
- [x] take destination client as cli arg
- [ ] find source of occasional broken pipe in server 
- [ ] non-naive awaiting init for gRPC server 
- [x] replace fixed buffers with streaming
- [ ] `tokio::io::copy()` / proper stream r/w split in `client.rs`
