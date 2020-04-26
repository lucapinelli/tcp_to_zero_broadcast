
# tcp-to-zero-broadcast

## Scope of the application

The application receives TCP messages and it broadcasts them using ZeroMQ.

Each message must be a valid JSON.
As convention, to terminate a message the byte `7` must be used.

## Logs

The application use the [env_logger](https://docs.rs/env_logger/0.7.1/env_logger/)
crate to handle the logs.

e.g. during the development to set the log level to `debug` run:

```sh
RUST_LOG=debug cargo run
```
