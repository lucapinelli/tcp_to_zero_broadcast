
# tcp-to-zero-broadcast

## Scope of the application

The application receives TCP messages and it broadcasts them using ZeroMQ.
As convention, to terminate a message the byte `7` must be used
(therefore the messages cannot contains this byte).

### Configuration

The default configuration is the following:
* the TCP server endpont is `127.0.0.1:1974`
* the message termination byte is `7`
* the ZeroMQ publischer endpoint is `tcp://*:2007`
* the ZeroMQ publischer topic used to send the messages is `parrot`

To customize the configuration add the file `config/local.toml` (you can use the
file `config/default.toml` as template).

## Logs

The application use the [env_logger](https://docs.rs/env_logger/0.7.1/env_logger/)
crate to handle the logs.

To set the log level during the development to `debug` run:

```sh
RUST_LOG=debug cargo run
```
