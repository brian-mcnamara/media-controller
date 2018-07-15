extern crate hyper;
extern crate base64;
extern crate hyper_tls;
extern crate http;
extern crate config;
#[macro_use]
extern crate futures;
use hyper::Client;
use hyper::rt::{self, Future, Stream};
use hyper::{Body, Request, Response};
use hyper::header::{AUTHORIZATION};
use base64::encode;
use hyper::body::Payload;
use http::StatusCode;
use std::collections::HashMap;
use config::*;

mod event;
mod event_stream;

fn main() {
    let mut settings = Config::default();
    settings.merge(File::with_name("conf/production.json")).unwrap();
    let setting_map = settings.try_into::<HashMap<String, String>>().unwrap();
    let https_connector = hyper_tls::HttpsConnector::new(2).unwrap();

    let auth = format!("{}:{}", setting_map.get("username").unwrap(), setting_map.get("token").unwrap());
    let encoded_auth = format!("Basic {}", encode(auth));
    let url: &[u8] = setting_map.get("endpoint").unwrap().as_ref();
    println!("{}", encoded_auth);
    let req = Request::get(url)
        .header("Authorization", encoded_auth.as_str())
        .body(Body::empty()).unwrap();
    let event_stream = event_stream::EventSource::new(https_connector).request(req);
    event_stream.for_each(|event| {
        println!("Got event {}", std::str::from_utf8(event.data.as_ref()).unwrap());
        Ok(())
    });

}
