#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate stable_skiplist;
extern crate xml;
#[macro_use]
extern crate quick_error;
extern crate clap;

mod wordnet;

use wordnet::WordNet;
use clap::{App, Arg, ArgMatches};
use std::process::exit;
use rocket::State;
use rocket::Response;
use rocket::http::{ContentType, Status};
use stable_skiplist::Bound::{Included, Unbounded};
use std::io::Cursor;


#[get("/static/<name>")]
fn get_static<'r>(name : String) -> Response<'r> {
    if name == "app.js" {
        Response::build()
            .header(ContentType::JavaScript)
            .sized_body(Cursor::new(include_str!("app.js")))
            .finalize()
    } else {
        Response::build()
            .status(Status::NotFound)
            .finalize()
    }
}

#[get("/json/<index>/<id>")]
fn synset<'r>(index : String, id : String, 
              status : State<WordNetState>) 
        -> Result<Response<'r>,&'static str> {
    let synsets = (if index == "wn31" {
        status.wordnet.synsets.get(&id)
            .ok_or_else(|| "Synset Not Found")
            .map(|x| vec![x.clone()])
    } else if index == "lemma" {
        match status.wordnet.by_lemma.get(&id)
            .ok_or_else(|| "Synset Not Found") {
            Ok(x) => {
                Ok(x.iter().map(|y| {
                    status.wordnet.synsets.get(y)
                        .expect("Synset ID not found")
                        .clone()
                }).collect())
            },
            Err(e) => Err(e)
        }
    } else if index == "ili" {
        status.wordnet.by_ili.get(&id)
            .ok_or_else(|| "Synset Not Found")
            .map(|x| {
                vec![status.wordnet.synsets.get(x)
                    .expect("Synset ID not found")
                    .clone()]
            })
    } else {
        Err("Bad ID")
    })?;
    Ok(Response::build()
        .sized_body(Cursor::new("serde_json"))
        .finalize())
}

#[get("/autocomplete/lemma/<key>")]
fn autocomplete_lemma(key : String, state : State<WordNetState>) 
    -> String {
    let mut buf = String::new();
    for s in state.wordnet.lemma_skiplist.range(Included(&key), Unbounded).take(10) {
        buf.push_str(s);
        buf.push_str("\n");
    }   
    buf
}

#[get("/")]
fn index<'r>() -> Response<'r> {
    Response::build()
        .sized_body(Cursor::new(include_str!("index.html")))
        .finalize()
}

struct Config {
    wn_file : String
}

impl Config {
    fn new(matches : &ArgMatches) -> Result<Config, &'static str> {
        let wn_file = matches.value_of("wn").unwrap_or("data/wn31.xml");
        Ok(Config {
            wn_file: wn_file.to_string()
        })
    }
}

struct WordNetState {
    wordnet: WordNet
}

fn prepare_server(config : Config) -> Result<WordNetState, String> {
    eprintln!("Loading WordNet data");
    let wordnet = WordNet::load(config.wn_file)
      .map_err(|e| format!("Failed to load WordNet: {}", e))?;
    eprintln!("Loaded {} synsets", wordnet.synsets.len());
    Ok(WordNetState {
        wordnet: wordnet
    })
}

fn main() {
    let mut app = App::new("wordnet-angular")
        .version("1.0")
        .author("John P. McCrae <john@mccr.ae>")
        .about("WordNet Angular Interface")
        .arg(Arg::with_name("wn")
            .long("wn")
            .value_name("wn31.xml")
            .help("The WordNet file in GWC LMF-XML format, e.g., http://john.mccr.ae/wn31.xml. Default is data/wn31.xml")
            .takes_value(true));
    let matches = app.clone().get_matches();
    match Config::new(&matches) {
        Ok(config) => 
            match prepare_server(config) {
                Ok(state) => {
                    rocket::ignite()
                        .manage(state)
                        .mount("/", routes![index, synset,
                                autocomplete_lemma, get_static]).launch();
                },
                Err(msg) => {
                    eprintln!("{}", msg);
                    exit(-1)
                }
            },
        Err(msg) => {
            println!("Failed: {}",msg);
            app.print_help().expect("Could not print help");
            println!("");
            exit(-1)
        }
    }
}
