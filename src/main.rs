#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate stable_skiplist;
extern crate xml;
#[macro_use]
extern crate quick_error;
extern crate clap;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate handlebars;

mod wordnet;
mod glosstag;
mod omwn;
mod links;

use std::str::FromStr;
use wordnet::{WNKey, WordNet, Synset};
use clap::{App, Arg, ArgMatches};
use std::process::exit;
use rocket::State;
use rocket::Response;
use rocket::http::{ContentType, Status};
use std::io::Cursor;
use std::fs::File;
use handlebars::{Handlebars};
use std::collections::HashMap;
//use stable_skiplist::OrderedSkipList;
//use wordnet::Sense;

#[derive(Clone,Debug,Serialize,Deserialize)]
struct SynsetsHB {
    synsets : Vec<Synset>,
    entries : HashMap<String, Vec<Synset>>,
    index : String,
    name : String
}

fn make_synsets_hb(synsets : Vec<Synset>, index : String, 
                   name : String) -> SynsetsHB {
    let mut entries = HashMap::new();
    for synset in synsets.iter() {
        for sense in synset.lemmas.iter() {
            let mut s2 = synset.clone();
            s2.lemmas = vec![sense.clone()];
            s2.relations.retain(|r| {
                match r.src_word {
                    None => true,
                    Some(ref s) => *s == sense.lemma
                }
            });
            entries.entry(format!("{}-{}", sense.lemma, synset.pos.to_string()))
                .or_insert_with(|| Vec::new())
                .push(s2);
        }
    }
    SynsetsHB {
        synsets: synsets,
        entries: entries,
        index : index,
        name: name
    }
}

#[get("/ttl/<index>/<name>")]
fn get_ttl<'r>(state : State<WordNetState>, index : String, name : String) 
        -> Result<Response<'r>, &'static str> {
    Ok(Response::build()
       .header(ContentType::new("text","turtle"))
       .sized_body(Cursor::new(
            state.handlebars.render("ttl", &make_synsets_hb(get_synsets(&state.wordnet, &index, &name)?,index,name)).map_err(|e| {
                    eprintln!("{}", e);
                    "Could not apply template"
                })?))
       .finalize())
}


#[get("/xml/<index>/<name>")]
fn get_xml<'r>(state : State<WordNetState>, index : String, name : String) 
        -> Result<Response<'r>, &'static str> {
    Ok(Response::build()
       .header(ContentType::XML)
       .sized_body(Cursor::new(
            state.handlebars.render("xml", &make_synsets_hb(get_synsets(&state.wordnet, &index, &name)?,index,name)).map_err(|e| {
                    eprintln!("{}", e);
                    "Could not apply template"
                })?))
       .finalize())
}

#[get("/flag/<code>")]
fn get_flag<'r>(code : String) -> Result<Response<'r>,::std::io::Error> {
    Ok(Response::build()
        .header(ContentType::GIF)
        .sized_body(File::open(&format!("flags/{}.gif", code))?)
        .finalize())
}

#[get("/static/<name>")]
fn get_static<'r>(name : String) -> Response<'r> {
    if name == "app.js" {
        Response::build()
            .header(ContentType::JavaScript)
            .sized_body(File::open("src/app.js").unwrap())
//            .sized_body(Cursor::new(include_str!("app.js")))
            .finalize()
    } else if name == "favicon.ico" {
        Response::build()
            .sized_body(File::open("src/favicon.ico").unwrap())
            .finalize()
    } else if name == "synset.html" {
        Response::build()
            .sized_body(File::open("src/synset.html").unwrap())
            .finalize()
    } else if name == "wordnet.html" {
        Response::build()
            .sized_body(File::open("src/wordnet.html").unwrap())
            .finalize()
    } else if name == "relation.html" {
        Response::build()
            .sized_body(File::open("src/relation.html").unwrap())
            .finalize()
    } else if name == "princeton.png" {
        Response::build()
            .header(ContentType::PNG)
            .sized_body(File::open("src/princeton.png").unwrap())
            .finalize()
    } else if name == "verbnet.gif" {
        Response::build()
            .header(ContentType::GIF)
            .sized_body(File::open("src/verbnet.gif").unwrap())
            .finalize()
    } else if name == "wikipedia.png" {
        Response::build()
            .header(ContentType::PNG)
            .sized_body(File::open("src/wikipedia.png").unwrap())
            .finalize()
    } else if name == "wn.css" {
        Response::build()
            .header(ContentType::CSS)
            .sized_body(File::open("src/wn.css").unwrap())
            .finalize()
    } else {
        Response::build()
            .status(Status::NotFound)
            .finalize()
    }
}

fn get_synsets(wordnet : &WordNet, index : &str, id : &str) 
        -> Result<Vec<Synset>, &'static str> {
    let wn = if index == "id" {
        vec![wordnet.get_synset(&WNKey::from_str(id)
                .map_err(|_| "Not a WordNet ID")?)
            .ok_or("Synset Not Found")?.clone()]
    } else if index == "lemma" {
        wordnet.get_by_lemma(id).iter().map(|x| (*x).clone()).collect()
    } else if index == "ili" {
        vec![wordnet.get_by_ili(id)
                .ok_or("Synset Not Found")?.clone()]
    } else if index == "sense_key" {
        vec![wordnet.get_by_sense_key(id)
                .ok_or("Synset Not Found")?.clone()]
     } else {
        vec![wordnet.get_by_old_id(index, &WNKey::from_str(id)
                .map_err(|_| "Not a WordNet Key")?)?
                .ok_or("Synset Not Found")?.clone()]
    };
    Ok(wn)
}

#[get("/json/<index>/<id>")]
fn synset<'r>(index : String, id : String, 
              status : State<WordNetState>) 
        -> Result<Response<'r>,&'static str> {
    let synsets = get_synsets(&status.wordnet, &index, &id)?;
    let json = serde_json::to_string(&synsets)
        .map_err(|_| "Failed to serialize synset")?;
    Ok(Response::build()
        .sized_body(Cursor::new(json))
        .finalize())
}

#[derive(Clone,Debug,Serialize,Deserialize)]
struct AutocompleteResult {
    display: String,
    item: String
}

fn autocomplete_wn_key(k : &str) -> Result<WNKey, &'static str> {
    if k.len() <= 10 {
        let mut k2 = String::from(k);
        k2.push_str(&"00000000-n"[k.len()..]);
        WNKey::from_str(&k2)
            .map_err(|_| "Not a WordNet ID")
    } else {
        Err("Not a WordNet ID")
    }
}

#[get("/autocomplete/<index>/<key>")]
fn autocomplete_lemma(index : String, key : String, 
        state : State<WordNetState>) -> Result<String, &'static str> {
    let mut results = Vec::new();
    if index == "lemma" {
        for s in state.wordnet.list_by_lemma(&key, 10) {
            if s.starts_with(&key) {
                results.push(AutocompleteResult {
                    display: s.to_string(),
                    item: s.to_string()
                })
            }
        }   
    } else if index == "id" {
        let key2 = autocomplete_wn_key(&key)?;
        for s in state.wordnet.list_by_id(&key2, 10) {
            if s.to_string().starts_with(&key) {
                results.push(AutocompleteResult {
                    display: s.to_string(),
                    item: s.to_string()
                })
            }
        }   
    } else if index == "ili" {
        for s in state.wordnet.list_by_ili(&key, 10) {
            if s.starts_with(&key) {
                results.push(AutocompleteResult {
                    display: s.to_string(),
                    item: s.to_string()
                })
            }
        }   
     } else if index == "sense_key" {
        for s in state.wordnet.list_by_sense_key(&key, 10) {
            if s.starts_with(&key) {
                results.push(AutocompleteResult {
                    display: s.to_string(),
                    item: s.to_string()
                })
            }
        }   
     } else {
        let key2 = autocomplete_wn_key(&key)?;
        for s in state.wordnet.list_by_old_id(&index, &key2, 10)? {
            if s.to_string().starts_with(&key) {
                results.push(AutocompleteResult {
                    display: s.to_string(),
                    item: s.to_string()
                })
            }
        }   
}
    serde_json::to_string(&results).map_err(|_| "Could not translate to JSON")
}

#[get("/lemma/<_key>")]
fn lemma<'r>(_key : String) -> Response<'r> { index() }
#[get("/id/<_key>")]
fn id<'r>(_key : String) -> Response<'r> { index() }
#[get("/ili/<_key>")]
fn ili<'r>(_key : String) -> Response<'r> { index() }
#[get("/sense_key/<_key>")]
fn sense_key<'r>(_key : String) -> Response<'r> { index() }
#[get("/pwn30/<_key>")]
fn pwn30<'r>(_key : String) -> Response<'r> { index() }
#[get("/pwn21/<_key>")]
fn pwn21<'r>(_key : String) -> Response<'r> { index() }
#[get("/pwn20/<_key>")]
fn pwn20<'r>(_key : String) -> Response<'r> { index() }
#[get("/pwn171/<_key>")]
fn pwn171<'r>(_key : String) -> Response<'r> { index() }
#[get("/pwn17/<_key>")]
fn pwn17<'r>(_key : String) -> Response<'r> { index() }
#[get("/pwn16/<_key>")]
fn pwn16<'r>(_key : String) -> Response<'r> { index() }

#[get("/")]
fn index<'r>() -> Response<'r> {
    Response::build()
        .sized_body(File::open("src/index.html").unwrap())
        //.sized_body(Cursor::new(include_str!("index.html")))
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
    wordnet: WordNet,
    handlebars: Handlebars
}

fn lemma_escape(h : &handlebars::Helper,
                _ : &Handlebars,
                rc : &mut handlebars::RenderContext) -> Result<(), handlebars::RenderError> {
    let param = h.param(0).and_then(|v| v.value().as_str()).unwrap_or("");
    try!(rc.writer.write(param.replace(" ", "_").into_bytes().as_ref()));
    Ok(())
}

fn long_pos(h : &handlebars::Helper,
                _ : &Handlebars,
                rc : &mut handlebars::RenderContext) -> Result<(), handlebars::RenderError> {
    let param = h.param(0)
        .ok_or_else(|| handlebars::RenderError::new("No parameter for pos_long"))
        .and_then(|v| {
            let v2 = v.value().as_str()
                .ok_or_else(|| handlebars::RenderError::new("No parameter value for pos long"))?;
            eprintln!("{}", v2);
            wordnet::PartOfSpeech::from_str(v2)
                .map_err(|e| handlebars::RenderError::new(&format!("{}", e)))
        })?;
    rc.writer.write(param.as_long_string().as_bytes().as_ref())?;
    Ok(())
}


fn prepare_server(config : Config) -> Result<WordNetState, String> {
    let mut handlebars = Handlebars::new();
    handlebars.register_template_string("xml", include_str!("xml.hbs"))
        .expect("Could not load xml.hbs");
    handlebars.register_template_string("ttl", include_str!("ttl.hbs"))
        .expect("Could not load ttl.hbs");
    handlebars.register_helper("lemma_escape", Box::new(lemma_escape));
    handlebars.register_helper("long_pos", Box::new(long_pos));
    eprintln!("Loading WordNet data");
    let wordnet = WordNet::load(config.wn_file)
      .map_err(|e| format!("Failed to load WordNet: {}", e))?;
    // Quick loading code for testing
    //let mut wordnet = WordNet {
    //    synsets : HashMap::new(),
    //    by_lemma : HashMap::new(),
    //    by_ili : HashMap::new(),
    //    by_sense_key : HashMap::new(),
    //    by_old_id : HashMap::new(),
    //    id_skiplist : OrderedSkipList::new(),
    //    lemma_skiplist : OrderedSkipList::new(),
    //    ili_skiplist : OrderedSkipList::new(),
    //    sense_key_skiplist : OrderedSkipList::new(),
    //    old_skiplist : HashMap::new()
    //};
    //wordnet.synsets.insert(WNKey::from_str("00001740-n").unwrap(), Synset {
    //    definition: "feline mammal usually having thick soft fur and no ability to roar: domestic cats; wildcats".to_string(),
    //    lemmas: vec![Sense {
    //        lemma: "cat".to_string(), 
    //        forms: vec!["cats".to_string()],
    //        sense_key: "cat%1:05:00::".to_string(),
    //        subcats: Vec::new()
    //    }, Sense {
    //        lemma: "true cat".to_string(),
    //        forms: Vec::new(),
    //        sense_key: "true_cat%1:05:00::".to_string(),
    //        subcats: vec!["I see the %s".to_string()]
    //    }],
    //    id: WNKey::from_str("00001740-n").unwrap(),
    //    ili: "i46593".to_string(),
    //    pos: wordnet::PartOfSpeech::Noun,
    //    subject: "noun.animal".to_string(),
    //    relations: vec![
    //        wordnet::Relation {
    //            src_word: Some("cat".to_string()),
    //            trg_word: Some("catty".to_string()),
    //            rel_type: "derivation".to_string(),
    //            target: "00001234-n".to_string(),
    //        },
    //        wordnet::Relation {
    //            src_word: None,
    //            trg_word: None,
    //            rel_type: "hypernym".to_string(),
    //            target: "00005678-n".to_string()
    //        }],
    //    old_keys: HashMap::new(),
    //    gloss: None,
    //    foreign: HashMap::new(),
    //    links: vec![links::Link { link_type: links::LinkType::Wikipedia, target: "Cat".to_string()}]
    //});
    //wordnet.by_lemma.insert("cat".to_string(), vec![WNKey::from_str("00001740-n").unwrap()]);
    Ok(WordNetState {
        wordnet: wordnet,
        handlebars: handlebars
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
                        .mount("/", routes![
                                get_xml, get_ttl,
                                index, synset, get_flag,
                                autocomplete_lemma, get_static,
                                lemma, id, ili, sense_key, 
                                pwn30, pwn21, pwn20, pwn17,
                                pwn171, pwn16]).launch();
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
