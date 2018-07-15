use hyper::Error;
use hyper::body::{Body, Chunk};
use futures::{Poll,Stream, Async, Future};
use std::vec::Vec;
use std::mem;
use hyper::client::connect::Connect;
use hyper::{Request, Client, Response};
use event::Event;
use hyper::client::ResponseFuture;
use hyper::rt::run;
use futures::future::ok;

enum EventState<C> {
    Uninitialized(),
    Connect(Request<Body>, Client<C, Body>),
    Connecting(Request<Body>, Client<C, Body>, ResponseFuture),
    Stream(Request<Body>, Client<C, Body>, Response<Body>),
    EndStream(Request<Body>, Client<C, Body>),
    Failue(Request<Body>),
    //Error()
}

pub struct EventSource<C> {
    event: Vec<u8>,
    status: Status,
    client: Client<C, Body>,
    future: EventState<C>
}


impl<C> Stream for EventState<C> {
    type Item = Event;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let next = match self {
            EventState::Uninitialized() => {
                println!("Error! Must initialize before calling poll");
                EventState::Uninitialized()
            }
            EventState::Connect(req, client) => {
                let request = client.request(req);
                EventState::Connecting(req.into(), client, request)
            },
            EventState::Connecting(req, client, req_future) => {
                req_future.and_then(|body| {
                    EventState::Stream(req.into(), client,body)
                })

            },
            EventState::Stream(req, client, res_body) => {
                let stream = EventStream::new(res_body.into_body());
                let event = try_ready!(stream);
                match event {
                    Ok(Async::Ready(Some(event))) => {
                        EventState::Stream(req.into(), client, res_body.into())
                        //return Ok(Some(event));
                    },
                    Ok(Async::Ready(None)) => {
                        EventState::Connect(req.into(), client)
                    }

                }
            },
            EventState::EndStream(req, client) => {
                EventState::Connect(req.into(), client)
            },
            EventState::Failue(req) => {
                EventState::Failue(req.into())
            }
        }
        *self = next;
    }


}

impl<C> EventSource<C> where C: Connect{
    pub fn new(conn: C) -> EventSource<C> {
        Self {
            client: Client::builder().build(conn),
            event: Vec::new(),
            status: Status::UNINITIALIZED,
            future: EventState::Uninitialized()
        }
    }

    pub fn request(&mut self, req: Request<Body>) -> Poll<Option<Self::Item>, Self::Error> {
        self.future = EventState::Connect(req, self.client);
        loop {
            self.future.poll()
        }
    }
}

pub struct EventStream {
    body: Body
}

impl EventStream {
    pub fn new(body: Body) -> EventStream {
        Self{
            body
        }
    }
}

enum Status {
    UNINITIALIZED,
    STARTED

}

impl Stream for EventStream {
    type Item=Event;
    type Error=Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        loop {
            match self.status {
                Status::UNINITIALIZED => {
                    let chunk: Option<Chunk> = try_ready!(self.body.poll());

                    match chunk {
                        // TODO: convert chunk into Event
                        Some(chunk) => {
//                            println!("initial {}", ::std::str::from_utf8(chunk.as_ref()).unwrap());
                            self.event.extend_from_slice(chunk.as_ref());
                            self.status = Status::STARTED;
                        },
                        None => {
                            println!("None");
                            return Ok(Async::Ready(None));
                        }
                    }
                }
                Status::STARTED => {
                    let chunk: Option<Chunk> = try_ready!(self.body.poll());
                    match chunk {
                        Some(chunk) => {
//                            println!("chunk {:x?}", ::std::str::from_utf8(chunk.as_ref()).unwrap());
                            if chunk.as_ref() == [b'\n', b'\n']  {
//                                println!("Recieved");
                                self.status = Status::UNINITIALIZED;
                                let ret = mem::replace(&mut self.event, Vec::new());
                                return Ok(Async::Ready(Some(Self::Item{body: ret})));
                            } else {
                                self.event.extend_from_slice(chunk.as_ref());
                            }
                        },
                        None => {
                            return Ok(Async::Ready(None));
                        }
                    }
                }
            }
        }
    }
}
