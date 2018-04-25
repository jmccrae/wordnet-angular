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
extern crate rusqlite;

mod wordnet_model;
mod wordnet;
//mod glosstag;
mod omwn;
mod links;
mod wordnet_read;

use std::str::FromStr;
use wordnet::{WNKey, WordNet};
use wordnet_model::Synset;
use clap::{App, Arg, ArgMatches};
use std::process::exit;
use rocket::State;
use rocket::Response;
use rocket::Request;
use rocket::request::{FromRequest,Outcome};
use rocket::http::hyper::header::{Location,CacheDirective,CacheControl};
use rocket::Outcome::Success;
use rocket::http::{ContentType, Status};
use std::io::Cursor;
use std::fs::File;
use handlebars::{Handlebars};
use std::collections::HashMap;
use rocket::config::{Environment, Config as RocketConfig};
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
        -> Result<Response<'r>, String> {
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
        -> Result<Response<'r>, String> {
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
        -> Result<Response<'r>, String> {
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
fn get_static<'r>(state : State<WordNetState>, name : String) -> Response<'r> {
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
            .sized_body(match state.site {
                WordNetSite::Princeton => File::open("src/favicon.ico").unwrap(),
                WordNetSite::Polylingual => File::open("src/polyling-favicon.ico").unwrap()
            })
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
    } else if name == "wn.css" && state.site == WordNetSite::Princeton {
        Response::build()
            .header(ContentType::CSS)
            .header(CacheControl(vec![CacheDirective::MaxAge(86400u32)]))
            .sized_body(Cursor::new(include_str!("wn.css")))
            //.sized_body(File::open("src/wn.css").unwrap())
            .finalize()
    } else if name == "wordnet.nt.gz" && state.site == WordNetSite::Princeton {
        Response::build()
            .header(ContentType::Binary)
            .sized_body(File::open("wordnet.nt.gz").unwrap())
            .finalize()
    } else if name == "polylingwn.css" && state.site == WordNetSite::Polylingual {
        Response::build()
            .header(ContentType::CSS)
            .header(CacheControl(vec![CacheDirective::MaxAge(86400u32)]))
            .sized_body(Cursor::new(include_str!("polylingwn.css")))
            .finalize()
    } else if name == "polylingwn.png" && state.site == WordNetSite::Polylingual {
        Response::build()
            .header(ContentType::PNG)
            .header(CacheControl(vec![CacheDirective::MaxAge(86400u32)]))
            .sized_body(File::open("src/polylingwn.png").unwrap())
            .finalize()
    } else {
        Response::build()
            .status(Status::NotFound)
            .finalize()
    }
}

fn get_synsets(wordnet : &WordNet, index : &str, id : &str) 
        -> Result<Vec<Synset>, String> {
    let wn = if index == "id" {
        vec![wordnet.get_synset(&WNKey::from_str(id)
                .map_err(|_| format!("Not a WordNet ID"))?)
            .map_err(|e| format!("Database error: {}", e))?
            .ok_or(format!("Synset Not Found"))?.clone()]
    } else if index == "lemma" {
        wordnet.get_by_lemma(id)
            .map_err(|e| format!("Database error: {}", e))?
            .iter().map(|x| (*x).clone()).collect()
    } else if index == "ili" {
        vec![wordnet.get_by_ili(id)
                .map_err(|e| format!("Database Error: {}", e))?
                .ok_or(format!("Synset Not Found"))?.clone()]
    } else if index == "sense_key" {
        vec![wordnet.get_by_sense_key(id)
                .map_err(|e| format!("Database Error: {}", e))?
                .ok_or(format!("Synset Not Found"))?.clone()]
     } else {
        vec![wordnet.get_by_old_id(index, &WNKey::from_str(id)
                .map_err(|_| format!("Not a WordNet Key"))?)
                .map_err(|e| format!("Database Error: {}", e))?
                .ok_or(format!("Synset Not Found"))?.clone()]
    };
    Ok(wn)
}

#[get("/json/<index>/<id>")]
fn synset<'r>(index : String, id : String, 
              status : State<WordNetState>) 
        -> Result<Response<'r>,String> {
    let synsets = get_synsets(&status.wordnet, &index, &id)?;
    let json = serde_json::to_string(&synsets)
        .map_err(|e| format!("Failed to serialize synset: {}", e))?;
    Ok(Response::build()
        .header(ContentType::JSON)
        .sized_body(Cursor::new(json))
        .finalize())
}

#[get("/json_rel/<id>")]
fn rel_targets<'r>(id : String, status : State<WordNetState>) -> Result<Response<'r>,String> {
    let synset = status.wordnet.get_synset(&WNKey::from_str(&id)
                .map_err(|_| format!("Not a WordNet ID"))?)
            .map_err(|e| format!("Database error: {}", e))?
            .ok_or(format!("Synset Not Found"))?;
    let mut targets = Vec::new();
    for rel in synset.relations {
        if let Some(ss) = status.wordnet.get_synset(&WNKey::from_str(&rel.target)
            .map_err(|_| format!("WordNet ID link not valid!"))?)
            .map_err(|_| format!("Could not read WordNet"))? {
            targets.push(ss);
        }
    }
    let json = serde_json::to_string(&targets)
        .map_err(|e| format!("Failed to serialize synset: {}", e))?;
    Ok(Response::build()
        .header(ContentType::JSON)
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
        state : State<WordNetState>) -> Result<String, String> {
    let mut results = Vec::new();
    if index == "lemma" {
        for s in state.wordnet.list_by_lemma(&key, "en", 10).map_err(|e| format!("Database error: {}", e))? {
//            if s.starts_with(&key) {
                results.push(AutocompleteResult {
                    display: s.to_string(),
                    item: s.to_string()
                })
//            }
        }   
    } else if index.starts_with("lemma") {
        let lang = index[6..].to_string();
        for s in state.wordnet.list_by_lemma(&key, &lang, 10).map_err(|e| format!("Database error: {}", e))? {
//            if s.starts_with(&key) {
                results.push(AutocompleteResult {
                    display: s.to_string(),
                    item: s.to_string()
                })
//            }
        }   
 
    } else if index == "id" {
        let key2 = autocomplete_wn_key(&key)?;
        for s in state.wordnet.list_by_id(&key2, 10).map_err(|e| format!("Database error: {}", e))? {
            if s.to_string().starts_with(&key) {
                results.push(AutocompleteResult {
                    display: s.to_string(),
                    item: s.to_string()
                })
            }
        }   
    } else if index == "ili" {
        for s in state.wordnet.list_by_ili(&key, 10).map_err(|e| format!("Database error: {}", e))? {
            if s.starts_with(&key) {
                results.push(AutocompleteResult {
                    display: s.to_string(),
                    item: s.to_string()
                })
            }
        }   
     } else if index == "sense_key" {
        for s in state.wordnet.list_by_sense_key(&key, 10).map_err(|e| format!("Database error: {}", e))? {
            if s.starts_with(&key) {
                results.push(AutocompleteResult {
                    display: s.to_string(),
                    item: s.to_string()
                })
            }
        }   
     } else {
        let key2 = autocomplete_wn_key(&key)?;
        for s in state.wordnet.list_by_old_id(&index, &key2, 10).map_err(|e| format!("Database error: {}", e))? {
            if s.to_string().starts_with(&key) {
                results.push(AutocompleteResult {
                    display: s.to_string(),
                    item: s.to_string()
                })
            }
        }   
}
    serde_json::to_string(&results).map_err(|e| format!("Json error: {}", e))
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
    

fn negotiated<'r>(state : State<WordNetState>, idx : &'static str, key : String, neg : ContentNegotiation) -> Response<'r> {
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
            ContentNegotiation::Html => { index(state) },
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
fn lemma<'r>(state : State<WordNetState>, key : String, neg : ContentNegotiation) -> Response<'r> { negotiated(state, "lemma", key, neg) }
#[get("/id/<key>")]
fn id<'r>(state : State<WordNetState>, key : String, neg : ContentNegotiation) -> Response<'r> { negotiated(state, "id", key, neg) }
#[get("/ili/<key>")]
fn ili<'r>(state : State<WordNetState>, key : String, neg : ContentNegotiation) -> Response<'r> { negotiated(state, "ili", key, neg) }
#[get("/sense_key/<key>")]
fn sense_key<'r>(state : State<WordNetState>, key : String, neg : ContentNegotiation) -> Response<'r> { negotiated(state, "sense_key", key, neg) }
#[get("/pwn30/<key>")]
fn pwn30<'r>(state : State<WordNetState>, key : String, neg : ContentNegotiation) -> Response<'r> { negotiated(state, "pwn30", key, neg) }
#[get("/pwn21/<key>")]
fn pwn21<'r>(state : State<WordNetState>, key : String, neg : ContentNegotiation) -> Response<'r> { negotiated(state, "pwn21", key, neg) }
#[get("/pwn20/<key>")]
fn pwn20<'r>(state : State<WordNetState>, key : String, neg : ContentNegotiation) -> Response<'r> { negotiated(state, "pwn20", key, neg) }
#[get("/pwn171/<key>")]
fn pwn171<'r>(state : State<WordNetState>, key : String, neg : ContentNegotiation) -> Response<'r> { negotiated(state, "pwn171", key, neg) }
#[get("/pwn17/<key>")]
fn pwn17<'r>(state : State<WordNetState>, key : String, neg : ContentNegotiation) -> Response<'r> { negotiated(state, "pwn17", key, neg) }
#[get("/pwn16/<key>")]
fn pwn16<'r>(state : State<WordNetState>, key : String, neg : ContentNegotiation) -> Response<'r> { negotiated(state, "pwn16", key, neg) }
fn is_old_wn_key(s : &str) -> bool {
    if s.len() == 10 {
        (0usize..8usize).all(|i| s.as_bytes()[i] >= 48 && s.as_bytes()[i] <= 57)
    } else if s.len() == 11 {
        (0usize..9usize).all(|i| s.as_bytes()[i] >= 48 && s.as_bytes()[i] <= 57)
    } else {
        false
    }
}
#[get("/wn31/<key>")]
fn wn31<'r>(key : String, neg : ContentNegotiation) -> Response<'r> { 
    if is_old_wn_key(&key) {
        renegotiated("id", key[1..key.len()].to_string(), neg) 
    } else {
        renegotiated("lemma", key[..(key.len()-2)].to_string(), neg)
    }
}
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
#[get("/wn31.nt.gz")]
fn wn31ntgz<'r>() -> Response<'r> {
    Response::build()
        .status(Status::SeeOther)
        .header(Location("/static/wordnet.nt.gz".to_owned()))
        .finalize()
}

#[get("/")]
fn index<'r>(state : State<WordNetState>) -> Response<'r> {
    Response::build()
        .header(CacheControl(vec![CacheDirective::MaxAge(86400u32)]))
        //.sized_body(File::open("src/index.html").unwrap())
        .sized_body(match state.site {
            WordNetSite::Princeton => Cursor::new(include_str!("index.html")),
            WordNetSite::Polylingual => Cursor::new(include_str!("polyling-index.html"))
        })
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
    port : u16,
    site : WordNetSite
}

impl Config {
    fn new(matches : &ArgMatches) -> Result<Config, &'static str> {
        let wn_file = matches.value_of("wn").unwrap_or("data/wn31.xml");
        let port = str::parse::<u16>(matches.value_of("port").unwrap_or("8000"))
            .map_err(|_| "Port must be an integer")?;
        let site = match matches.value_of("site").unwrap_or("princeton") {
            "princeton" => WordNetSite::Princeton,
            "polylingual" => WordNetSite::Polylingual,
            _ => return Err("Bad site")
        };
        Ok(Config {
            wn_file: wn_file.to_string(),
            reload: matches.is_present("reload"),
            port: port,
            site: site
        })
    }
}

struct WordNetState {
    wordnet: WordNet,
    handlebars: Handlebars,
    site : WordNetSite
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
            wordnet_model::PartOfSpeech::from_str(v2)
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
        wordnet_read::load_pwn(config.wn_file)
      .map_err(|e| format!("Failed to load WordNet: {}", e))?
    } else {
        eprintln!("Opening WordNet data");
        WordNet::new()
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
    eprintln!("WordNet loaded");
    Ok(WordNetState {
        wordnet: wordnet,
        handlebars: handlebars,
        site: config.site
    })
}

#[derive(Clone,Debug,PartialEq)]
enum WordNetSite {
    Princeton,
    Polylingual
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
        .arg(Arg::with_name("site")
             .short("s")
             .value_name("princeton|polylingual")
             .help("The site design to use")
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
                    eprintln!("Starting at port {}", config.port);
                    rocket::custom(
                        RocketConfig::build(Environment::Staging)
                                .port(config.port)
                                .finalize()
                                .expect("Could not configure Rocket"), false)
                        .manage(state)
                        .mount("/", routes![
                                about, ontology, ontology_html, license,
                                get_xml, get_ttl, get_rdf, rel_targets,
                                index, synset, get_flag,
                                autocomplete_lemma, get_static,
                                lemma, id, ili, sense_key, 
                                wn30, wn21, wn20, wn17,
                                wn171, wn16, wn31, wn31ntgz,
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
