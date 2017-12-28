extern crate futures;
extern crate bytes;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_service;
extern crate tokio_proto;

use std::io;
use std::str;
use bytes::BytesMut;


//Our encoding item
use tokio_io::codec::{Encoder, Decoder};
pub struct LineCodec;

impl Decoder for LineCodec {
    type Item = String;
    type Error = io::Error;
    fn decode(&mut self, src : &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        //Traverse to find \n
        if let Some(i) = src.iter().position(|&b| b == b'\n') {
            //Get the entire line
            let line = src.split_to(i);

            //Take off the newline of remaining buf
            src.split_to(1);

            //Turn the data into a UTF string
            match str::from_utf8(&line) {
                Ok(s) => Ok(Some(s.to_string())),
                Err(_) => Err(io::Error::new(io::ErrorKind::Other,"invalid UTF-8"))
            }
        } else {
            Ok(None)
        }
    }
}

impl Encoder for LineCodec {
    type Item = String;
    type Error = io::Error;

    fn encode(&mut self, item : Self::Item, dst : &mut BytesMut) -> Result<(), Self::Error> {
        dst.extend(item.as_bytes());
        dst.extend(b"\n");
        Ok(())
    }
}

//Our protocol. Essentially just a wrapper around the encoder and decoder,
//that allows them to be properly bound together into a server socket
use tokio_proto::pipeline::ServerProto;
use tokio_io::{AsyncRead, AsyncWrite};
use tokio_io::codec::Framed;

pub struct LineProto;

impl<T: AsyncRead + AsyncWrite + 'static> ServerProto<T> for LineProto {
    //For  this protocol, 'Request' matches 'Item' of the codec Decoder
    type Request = String;

    //Ditto Response <-> Encoder
    type Response = String;

    //Codec shit
    type Transport = Framed<T, LineCodec>;
    type BindTransport = Result<Self::Transport, io::Error>;
    fn bind_transport(&self, io: T) -> Self::BindTransport {
        Ok(io.framed(LineCodec))
    }
}

//Our server service
use tokio_service::Service;
use futures::{future, Future};
use std::sync::{Arc, Mutex};

//Struct for tracking service stats
struct EchoStats {
    count : i32
}

impl EchoStats {
    fn new() -> EchoStats {
        EchoStats {count : 0}
    }
}

//The server service itself
pub struct Echo {
    stats : Arc<Mutex<EchoStats>>
}

impl Echo {
    fn new() -> Echo {
        Echo {stats : Arc::new(Mutex::new(EchoStats::new()))}
    }
}


impl Service for Echo {
    //Again, these types must match protocol. Deduction would be nice, but
    //I suppose its a bit much to ask our compileaeons for now, eh?
    type Request = String;
    type Response = String;

    //For non streaming, we will always use io:Error
    type Error = io::Error;

    //The FUTURE.
    //More specifically, this is what each server transaction (IE a request
    // awaiting a response) is wrapped in to ensure asynchronicity.
    // Doesn't necessarily have to be boxed, but, eh.
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        //Increment our stat counter
        let mut stats = self.stats.lock().unwrap();
        stats.count += 1;

        let result = format!("{message} - {count}\n", message = req, count = stats.count);
        //Just send the request back in a box.
        Box::new(future::ok(String::from(result)))
    }
}


//Run a server with our service!
use tokio_proto::TcpServer;
use std::thread;
use std::time::Duration;
fn main() {
    //Spool up the server in a bg thread
    println!("Running server");
    thread::spawn(|| {
        let addr = "127.0.0.1:12345".parse().unwrap();
        let server = TcpServer::new(LineProto, addr);

        server.serve(|| Ok(Echo::new()));
    });


    //After a few moments, call a test function
    thread::sleep(Duration::new(3,0));
    println!("Running test");
    testo();
    thread::sleep(Duration::new(3,0));
}


//dummy
use std::io::prelude::*;
use std::net::TcpStream;
fn testo() {
    match TcpStream::connect("127.0.0.1:12345") {
        Ok(mut stream) => {
            let _ = stream.write(b"what");
            let _ = stream.write(b"the");
            let _ = stream.write(b"fuck");
            let _ = stream.write(b"did");
            let _ = stream.write(b"you");
            let _ = stream.write(b"just");
        },
        Err(_) => {
            println!("Failed to connect to server in testing branch");
        }
    }
}