# `mixtonic helloworld`

Running the [`tonic` crate HelloWorld example](https://github.com/hyperium/tonic/blob/master/examples/helloworld-tutorial.md) over the mixnet for initial gRPC-over-mixnet example project structure. 

## run
```
# terminal window 1: run this and wait for `>> 27 [0, 0, 18, 4, 0, 0, 0, 0, 0, 0, 4, 0, 16, 0, 0, 0, 5, 0, 0, 64, 0, 0, 6, 1, 0, 0, 0]` to appear in console 
cargo run --bin helloworld-server

# terminal window 2: 
cargo run --bin helloworld-client
```

## todo
- [ ] sort out non-utf8 noise on return 
- [ ] readme arch diagram
- [ ] non-naive awaiting init for gRPC server 
- [ ] replace fixed buffers with streaming
- [ ] `tokio::io::copy()`
