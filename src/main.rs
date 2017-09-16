#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate skiplist;
extern crate xml;
#[macro_use]
extern crate quick_error;
extern crate clap;

mod wordnet;

use wordnet::WordNet;
use clap::{App, Arg, ArgMatches};
use std::process::exit;
use rocket::State;

#[get("/json/wn31/<id>")]
fn synset_wn31(id : String, status : State<WordNetState>) -> Result<String, &'static str> {
    match status.wordnet.synsets.get(&id) {
        Some(ref synset) => { Ok(synset.definition.clone()) },
        None => { Err("Fail") }
    }
}

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
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
    let wordnet = WordNet::load(config.wn_file)
      .map_err(|e| format!("Failed to load WordNet: {}", e))?;
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
                        .mount("/", routes![index, synset_wn31]).launch();
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
