#![feature(custom_attribute, iterator_flatten)]

extern crate activitypub;
#[macro_use]
extern crate activitystreams_derive;
extern crate activitystreams_traits;
extern crate array_tool;
extern crate base64;
extern crate chrono;
extern crate failure;
#[macro_use]
extern crate failure_derive;
extern crate gettextrs;
extern crate hex;
extern crate heck;
#[macro_use]
extern crate hyper;
extern crate openssl;
extern crate pulldown_cmark;
extern crate reqwest;
extern crate rocket;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;

pub mod activity_pub;
pub mod utils;
