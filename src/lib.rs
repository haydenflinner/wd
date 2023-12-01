#![feature(async_fn_in_trait)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![feature(iter_intersperse)]
#![feature(pattern)]
// Drainrs
// #![feature(iter_intersperse)]
#![feature(hash_raw_entry)]
// Open to not using this but it's what's used for now.
#![feature(inherent_associated_types)]
// End drainrs

pub mod app;

pub mod components;

pub mod event;

pub mod tui;

pub mod action;

pub mod logging;

pub mod drainrs;

pub mod utils;

pub mod dateparser;
