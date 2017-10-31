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
extern crate sled;

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
use rocket::Request;
use rocket::config::{Environment, Config as RocketConfig};
use rocket::request::{FromRequest,Outcome};
use rocket::http::hyper::header::{Location,CacheDirective,CacheControl};
use rocket::Outcome::Success;
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

#[get("/rdf/<index>/<name>")]
fn get_rdf<'r>(state : State<WordNetState>, index : String, name : String) 
        -> Result<Response<'r>, &'static str> {
    Ok(Response::build()
       .header(ContentType::new("application","rdf+xml"))
       .sized_body(Cursor::new(
            state.handlebars.render("rdfxml", &make_synsets_hb(get_synsets(&state.wordnet, &index, &name)?,index,name)).map_err(|e| {
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
        .header(CacheControl(vec![CacheDirective::MaxAge(86400u32)]))
        .sized_body(File::open(&format!("flags/{}.gif", code))?)
        .finalize())
}

#[get("/static/<name>")]
fn get_static<'r>(name : String) -> Response<'r> {
    if name == "app.js" {
        Response::build()
            .header(ContentType::JavaScript)
            .header(CacheControl(vec![CacheDirective::MaxAge(86400u32)]))
            .sized_body(Cursor::new(include_str!("app.js")))
            //.sized_body(File::open("src/app.js").unwrap())
            .finalize()
    } else if name == "favicon.ico" {
        Response::build()
            .header(CacheControl(vec![CacheDirective::MaxAge(86400u32)]))
            //.sized_body(Cursor::new(include_str!("favicon.ico")))
            .sized_body(File::open("src/favicon.ico").unwrap())
            .finalize()
    } else if name == "synset.html" {
        Response::build()
            .header(CacheControl(vec![CacheDirective::MaxAge(86400u32)]))
            .sized_body(Cursor::new(include_str!("synset.html")))
            //.sized_body(File::open("src/synset.html").unwrap())
            .finalize()
    } else if name == "wordnet.html" {
        Response::build()
            .header(CacheControl(vec![CacheDirective::MaxAge(86400u32)]))
            .sized_body(Cursor::new(include_str!("wordnet.html")))
            //.sized_body(File::open("src/wordnet.html").unwrap())
            .finalize()
    } else if name == "relation.html" {
        Response::build()
            .header(CacheControl(vec![CacheDirective::MaxAge(86400u32)]))
            .sized_body(Cursor::new(include_str!("relation.html")))
            //.sized_body(File::open("src/relation.html").unwrap())
            .finalize()
    } else if name == "princeton.png" {
        Response::build()
            .header(ContentType::PNG)
            .header(CacheControl(vec![CacheDirective::MaxAge(86400u32)]))
            .sized_body(File::open("src/princeton.png").unwrap())
            .finalize()
    } else if name == "verbnet.gif" {
        Response::build()
            .header(ContentType::GIF)
            .header(CacheControl(vec![CacheDirective::MaxAge(86400u32)]))
            .sized_body(File::open("src/verbnet.gif").unwrap())
            .finalize()
    } else if name == "wikipedia.png" {
        Response::build()
            .header(ContentType::PNG)
            .header(CacheControl(vec![CacheDirective::MaxAge(86400u32)]))
            .sized_body(File::open("src/wikipedia.png").unwrap())
            .finalize()
    } else if name == "wn.css" {
        Response::build()
            .header(ContentType::CSS)
            .header(CacheControl(vec![CacheDirective::MaxAge(86400u32)]))
            .sized_body(Cursor::new(include_str!("wn.css")))
            //.sized_body(File::open("src/wn.css").unwrap())
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

enum ContentNegotiation { Html, RdfXml, Turtle, Json }

impl<'a,'r> FromRequest<'a,'r> for ContentNegotiation {
    type Error = String;
    fn from_request(request: &'a Request<'r>) -> Outcome<ContentNegotiation, String> {
        for value in request.headers().get("Accepts") {
            if value.starts_with("text/html") {
                return Success(ContentNegotiation::Html);
            } else if value.starts_with("application/rdf+xml") {
                return Success(ContentNegotiation::RdfXml);
            } else if value.starts_with("text/turtle") {
                return Success(ContentNegotiation::Turtle);
            } else if value.starts_with("application/x-turtle") {
                return Success(ContentNegotiation::Turtle);
            } else if value.starts_with("application/json") {
                return Success(ContentNegotiation::Json);
            } else if value.starts_with("application/javascript") {
                return Success(ContentNegotiation::Json);
            }
        }
        Success(ContentNegotiation::Html)
    }
}
    

fn negotiated<'r>(idx : &'static str, key : String, neg : ContentNegotiation) -> Response<'r> {
    if key.ends_with(".rdf") {
        renegotiated(idx,key[0..(key.len()-4)].to_string(), ContentNegotiation::RdfXml)
    } else if key.ends_with(".ttl") {
        renegotiated(idx,key[0..(key.len()-4)].to_string(), ContentNegotiation::Turtle)
    } else if key.ends_with(".json") {
        renegotiated(idx,key[0..(key.len()-5)].to_string(), ContentNegotiation::Json)
    } else if key.ends_with(".html") {
        renegotiated(idx,key[0..(key.len()-5)].to_string(), ContentNegotiation::Html)
    } else {
        match neg {
            ContentNegotiation::Html => { index() },
            ContentNegotiation::RdfXml => {
                Response::build()
                    .status(Status::SeeOther)
                    .header(Location(format!("/rdf/{}/{}", idx, key)))
                    .finalize()
            },
            ContentNegotiation::Turtle => {
                Response::build()
                    .status(Status::SeeOther)
                    .header(Location(format!("/ttl/{}/{}", idx, key)))
                    .finalize()
            },
            ContentNegotiation::Json => {
                Response::build()
                    .status(Status::SeeOther)
                    .header(Location(format!("/{}/{}", idx, key)))
                    .finalize()
            }
        }
    }
}

fn renegotiated<'r>(idx : &'static str, key : String, neg : ContentNegotiation) -> Response<'r> {
    if key.ends_with(".rdf") {
        renegotiated(idx,key[0..(key.len()-4)].to_string(), ContentNegotiation::RdfXml)
    } else if key.ends_with(".ttl") {
        renegotiated(idx,key[0..(key.len()-4)].to_string(), ContentNegotiation::Turtle)
    } else if key.ends_with(".json") {
        renegotiated(idx,key[0..(key.len()-5)].to_string(), ContentNegotiation::Json)
    } else if key.ends_with(".html") {
        renegotiated(idx,key[0..(key.len()-5)].to_string(), ContentNegotiation::Html)
    } else {
        match neg {
            ContentNegotiation::Html => { 
                Response::build()
                    .status(Status::SeeOther)
                    .header(Location(format!("/{}/{}", idx, key)))
                    .finalize()
            },
            ContentNegotiation::RdfXml => {
                Response::build()
                    .status(Status::SeeOther)
                    .header(Location(format!("/rdf/{}/{}", idx, key)))
                    .finalize()
            },
            ContentNegotiation::Turtle => {
                Response::build()
                    .status(Status::SeeOther)
                    .header(Location(format!("/ttl/{}/{}", idx, key)))
                    .finalize()
            },

            ContentNegotiation::Json => {
                Response::build()
                    .status(Status::SeeOther)
                    .header(Location(format!("/json/{}/{}", idx, key)))
                    .finalize()
            }
        }
    }
}


#[get("/lemma/<key>")]
fn lemma<'r>(key : String, neg : ContentNegotiation) -> Response<'r> { negotiated("lemma", key, neg) }
#[get("/id/<key>")]
fn id<'r>(key : String, neg : ContentNegotiation) -> Response<'r> { negotiated("id", key, neg) }
#[get("/ili/<key>")]
fn ili<'r>(key : String, neg : ContentNegotiation) -> Response<'r> { negotiated("ili", key, neg) }
#[get("/sense_key/<key>")]
fn sense_key<'r>(key : String, neg : ContentNegotiation) -> Response<'r> { negotiated("sense_key", key, neg) }
#[get("/pwn30/<key>")]
fn pwn30<'r>(key : String, neg : ContentNegotiation) -> Response<'r> { negotiated("pwn30", key, neg) }
#[get("/pwn21/<key>")]
fn pwn21<'r>(key : String, neg : ContentNegotiation) -> Response<'r> { negotiated("pwn21", key, neg) }
#[get("/pwn20/<key>")]
fn pwn20<'r>(key : String, neg : ContentNegotiation) -> Response<'r> { negotiated("pwn20", key, neg) }
#[get("/pwn171/<key>")]
fn pwn171<'r>(key : String, neg : ContentNegotiation) -> Response<'r> { negotiated("pwn171", key, neg) }
#[get("/pwn17/<key>")]
fn pwn17<'r>(key : String, neg : ContentNegotiation) -> Response<'r> { negotiated("pwn17", key, neg) }
#[get("/pwn16/<key>")]
fn pwn16<'r>(key : String, neg : ContentNegotiation) -> Response<'r> { negotiated("pwn16", key, neg) }
#[get("/wn31/<key>")]
fn wn31<'r>(key : String, neg : ContentNegotiation) -> Response<'r> { renegotiated("id", key[1..key.len()].to_string(), neg) }
#[get("/wn30/<key>")]
fn wn30<'r>(key : String, neg : ContentNegotiation) -> Response<'r> { renegotiated("pwn30", key, neg) }
#[get("/wn21/<key>")]
fn wn21<'r>(key : String, neg : ContentNegotiation) -> Response<'r> { renegotiated("pwn21", key, neg) }
#[get("/wn20/<key>")]
fn wn20<'r>(key : String, neg : ContentNegotiation) -> Response<'r> { renegotiated("pwn20", key, neg) }
#[get("/wn171/<key>")]
fn wn171<'r>(key : String, neg : ContentNegotiation) -> Response<'r> { renegotiated("pwn171", key, neg) }
#[get("/wn17/<key>")]
fn wn17<'r>(key : String, neg : ContentNegotiation) -> Response<'r> { renegotiated("pwn17", key, neg) }
#[get("/wn16/<key>")]
fn wn16<'r>(key : String, neg : ContentNegotiation) -> Response<'r> { renegotiated("pwn16", key, neg) }

#[get("/")]
fn index<'r>() -> Response<'r> {
    Response::build()
        .header(CacheControl(vec![CacheDirective::MaxAge(86400u32)]))
        //.sized_body(File::open("src/index.html").unwrap())
        .sized_body(Cursor::new(include_str!("index.html")))
        .finalize()
}

#[get("/about")]
fn about<'r>() -> Response<'r> {
    Response::build()
        .header(CacheControl(vec![CacheDirective::MaxAge(86400u32)]))
        //.sized_body(File::open("src/about.html").unwrap())
        .sized_body(Cursor::new(include_str!("about.html")))
        .finalize()
}

#[get("/license")]
fn license<'r>() -> Response<'r> {
    Response::build()
        .header(CacheControl(vec![CacheDirective::MaxAge(86400u32)]))
        //.sized_body(File::open("src/license.html").unwrap())
        .sized_body(Cursor::new(include_str!("license.html")))
        .finalize()
}


#[get("/ontology")]
fn ontology<'r>() -> Response<'r> {
    Response::build()
        .header(ContentType::new("application","rdf+xml"))
        .header(CacheControl(vec![CacheDirective::MaxAge(86400u32)]))
        //.sized_body(File::open("src/ontology.rdf").unwrap())
        .sized_body(Cursor::new(include_str!("ontology.rdf")))
        .finalize()
}

#[get("/ontology.html")]
fn ontology_html<'r>() -> Response<'r> {
    Response::build()
        .header(CacheControl(vec![CacheDirective::MaxAge(86400u32)]))
        //.sized_body(File::open("src/ontology.html").unwrap())
        .sized_body(Cursor::new(include_str!("ontology.html")))
        .finalize()
}

        
#[derive(Clone)]
struct Config {
    wn_file : String,
    reload : bool,
    port : u16
}

impl Config {
    fn new(matches : &ArgMatches) -> Result<Config, &'static str> {
        let wn_file = matches.value_of("wn").unwrap_or("data/wn31.xml");
        let port = str::parse::<u16>(matches.value_of("port").unwrap_or("8000"))
            .map_err(|_| "Port must be an integer")?;
        Ok(Config {
            wn_file: wn_file.to_string(),
            reload: matches.is_present("reload"),
            port: port
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
    handlebars.register_template_string("rdfxml", include_str!("rdfxml.hbs"))
        .expect("Could not load rdfxml.hbs");
    handlebars.register_helper("lemma_escape", Box::new(lemma_escape));
    handlebars.register_helper("long_pos", Box::new(long_pos));
    let wordnet = if config.reload  {
        eprintln!("Loading WordNet data");
        WordNet::load(config.wn_file)
      .map_err(|e| format!("Failed to load WordNet: {}", e))?
    } else {
        eprintln!("Opening WordNet data");
        WordNet::new_using_indexes()
    };
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
        .arg(Arg::with_name("reload")
             .long("reload")
             .help("Reload the indexes from the sources")
             .takes_value(false))
        .arg(Arg::with_name("port")
             .short("p")
             .value_name("port")
             .help("The port to start the server on")
             .takes_value(true))
        .arg(Arg::with_name("wn")
            .long("wn")
            .value_name("wn31.xml")
            .help("The WordNet file in GWC LMF-XML format, e.g., http://john.mccr.ae/wn31.xml. Default is data/wn31.xml")
            .takes_value(true));
    let matches = app.clone().get_matches();
    match Config::new(&matches) {
        Ok(config) => 
            match prepare_server(config.clone()) {
                Ok(state) => {
                    rocket::custom(
                        RocketConfig::build(Environment::Staging)
                                .port(config.port)
                                .finalize()
                                .expect("Could not configure Rocket"), false)
                        .manage(state)
                        .mount("/", routes![
                                about, ontology, ontology_html, license,
                                get_xml, get_ttl, get_rdf,
                                index, synset, get_flag,
                                autocomplete_lemma, get_static,
                                lemma, id, ili, sense_key, 
                                wn30, wn21, wn20, wn17,
                                wn171, wn16, wn31,
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
