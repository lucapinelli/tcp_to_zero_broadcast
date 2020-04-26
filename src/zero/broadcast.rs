use zmq::{Context, Error, Socket, PUB};

pub struct Broadcast {
    socket: Socket,
}

impl Broadcast {
    pub fn new(endpoint: &str) -> Result<Self, Error> {
        let context = Context::new();
        let socket = context.socket(PUB)?;
        socket.bind(endpoint)?;

        Ok(Broadcast { socket })
    }

    pub fn send(&mut self, topic: &str, message: &str) -> Result<(), Error> {
        trace!("ZMQ sending topic=\"{}\", message: {}", topic, message);
        self.socket.send(topic, zmq::SNDMORE)?;
        self.socket.send(message, 0)?;

        Ok(())
    }
}
