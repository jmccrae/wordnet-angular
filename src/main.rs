#[macro_use] extern crate rocket;
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
mod glosstag;
mod omwn;
mod links;
mod wordnet_read;

use std::str::FromStr;
use wordnet::{WNKey, WordNet};
use wordnet_model::Synset;
use clap::{App, Arg, ArgMatches};
use std::process::exit;
use rocket::Request;
use rocket::request::{FromRequest,Outcome};
use rocket::http::ContentType;
use rocket::response::content::{RawHtml, RawJson};
use rocket::response::Redirect;
use std::env;
use std::fs::File;
use std::fs;
use std::path::Path;
use std::ops::Deref;
use handlebars::Handlebars;
use std::collections::HashMap;
use rocket::config::Config as RocketConfig;
use once_cell::sync::Lazy;
use std::sync::{Mutex, MutexGuard};

#[derive(Clone,Debug,Serialize,Deserialize)]
struct SynsetsHB {
    synsets : Vec<Synset>,
    entries : HashMap<String,HashMap<String, Vec<Synset>>>,
    index : String,
    name : String,
    license : &'static str,
    site : &'static str
}

fn make_synsets_hb(synsets : Vec<Synset>, index : String, 
                   name : String, site : &WordNetSite) -> SynsetsHB {
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
            entries.entry(sense.language.clone())
                .or_insert_with(|| HashMap::new())
                .entry(format!("{}-{}", sense.lemma, synset.pos.to_string()))
                .or_insert_with(|| Vec::new())
                .push(s2);
        }
    }
    let license = match site {
        WordNetSite::Princeton => "http://wordnet.princeton.edu/wordnet/license/",
        WordNetSite::English => "https://github.com/globalwordnet/english-wordnet/blob/master/LICENSE.md",
        WordNetSite::Polylingual => "http://creativecommons.org/licenses/by/4.0/"
    };
    let site_url = match site {
        WordNetSite::Princeton => "http://wordnet-rdf.princeton.edu",
        WordNetSite::English => "https://en-word.net",
        WordNetSite::Polylingual => "http://polylingwn.linguistic-lod.org"
    };
    SynsetsHB {
        synsets,
        entries,
        index,
        name,
        license,
        site: site_url
    }
}

//fn html_utf8() -> ContentType { ContentType::with_params("text", "html", ("charset", "UTF-8")) }

#[get("/ttl/<index>/<name>")]
fn get_ttl<'r>(index : &str, name : &str) 
        -> Result<(ContentType, String), String> {
    let state = WordNetState::get();
    Ok((ContentType::new("text","turtle"), 
            state.handlebars.render("ttl", 
                &make_synsets_hb(get_synsets(&state.wordnet, &index, &name)?,
                index.to_string(),name.to_string(),&state.site)).map_err(|e| {
                    eprintln!("{}", e);
                    "Could not apply template"
                })?))
}

#[get("/rdf/<index>/<name>")]
fn get_rdf<'r>(index : &str, name : &str) 
        -> Result<(ContentType, String), String> {
    let state = WordNetState::get();
    Ok((ContentType::new("application","rdf+xml"), state.handlebars.render("rdfxml", &make_synsets_hb(get_synsets(&state.wordnet, &index, &name)?,index.to_string(),name.to_string(),&state.site)).map_err(|e| {
                    eprintln!("{}", e);
                    "Could not apply template"
                })?))
}



#[get("/xml/<index>/<name>")]
fn get_xml<'r>(index : &str, name : &str) 
        -> Result<(ContentType, String), String> {
    let state = WordNetState::get();
    let xml_template = match *state.site {
        WordNetSite::Polylingual => "xml-poly",
        WordNetSite::English => "xml-english",
        _ => "xml"
    };
    Ok((ContentType::XML, state.handlebars.render(xml_template, &make_synsets_hb(get_synsets(&state.wordnet, &index, &name)?,index.to_string(),name.to_string(),&state.site)).map_err(|e| {
                    eprintln!("{}", e);
                    "Could not apply template"
                })?))
}

#[get("/flag/<code>")]
fn get_flag<'r>(code : &str) -> Result<(ContentType, File),::std::io::Error> {
    Ok((ContentType::GIF, File::open(&format!("flags/{}.gif", code))?))
}

#[derive(Responder)]
enum StaticResponse {
    I((ContentType, &'static str)),
    F((ContentType, File)),
}

#[get("/static/<name>")]
fn get_static(name : &str) -> Option<StaticResponse> {
    let state = WordNetState::get();
    if name == "app.js" {
        if *state.site == WordNetSite::Princeton || *state.site == WordNetSite::English {
            Some(StaticResponse::I((ContentType::JavaScript, include_str!("app.js"))))
        } else {
            Some(StaticResponse::I((ContentType::JavaScript, include_str!("polyling-app.js"))))
        }

    } else if name == "favicon.ico" && *state.site == WordNetSite::English {
        Some(StaticResponse::F((ContentType::Icon, File::open("src/english-favicon.ico").unwrap())))
    } else if name == "favicon.ico" {
        Some(StaticResponse::F((ContentType::Icon, File::open("src/favicon.ico").unwrap())))
    } else if name == "synset.html" {
        if *state.site == WordNetSite::Princeton || *state.site == WordNetSite::English {
            Some(StaticResponse::I((ContentType::HTML, include_str!("synset.html"))))
        } else {
            Some(StaticResponse::I((ContentType::HTML, include_str!("polyling-synset.html"))))
        }
    } else if name == "wordnet.html" {
        if *state.site == WordNetSite::Princeton {
            Some(StaticResponse::I((ContentType::HTML, include_str!("wordnet.html"))))
        } else if *state.site == WordNetSite::English {
            Some(StaticResponse::I((ContentType::HTML, include_str!("english-wordnet.html"))))
        } else {
            Some(StaticResponse::I((ContentType::HTML, include_str!("polyling-wordnet.html"))))
        }
    } else if name == "relation.html" {
        Some(StaticResponse::I((ContentType::HTML, include_str!("relation.html"))))
    } else if name == "princeton.png" {
        Some(StaticResponse::F((ContentType::PNG, File::open("src/princeton.png").unwrap())))
    } else if name == "verbnet.gif" {
        Some(StaticResponse::F((ContentType::GIF, File::open("src/verbnet.gif").unwrap())))
    } else if name == "wikipedia.png" {
        Some(StaticResponse::F((ContentType::PNG, File::open("src/wikipedia.png").unwrap())))
    } else if name == "wn.css" && *state.site == WordNetSite::Princeton {
        Some(StaticResponse::I((ContentType::CSS, include_str!("wn.css"))))
    } else if name == "wordnet.nt.gz" && *state.site == WordNetSite::Princeton {
        Some(StaticResponse::I((ContentType::Binary, "wordnet.nt.gz")))
    } else if name == "polylingwn.css" && *state.site == WordNetSite::Polylingual {
        Some(StaticResponse::I((ContentType::CSS, include_str!("polylingwn.css"))))
    } else if name == "polylingwn.svg" && *state.site == WordNetSite::Polylingual {
        Some(StaticResponse::I((ContentType::SVG, "polylingwn.svg")))
    } else if name == "english.css" && *state.site == WordNetSite::English {
        Some(StaticResponse::I((ContentType::CSS, include_str!("english.css"))))
    } else if name == "english.svg" && *state.site == WordNetSite::English {
        Some(StaticResponse::F((ContentType::SVG, File::open("src/english.svg").unwrap())))
    } else if name == "english-wordnet-2019.ttl.gz" && *state.site == WordNetSite::English {
        Some(StaticResponse::F((ContentType::Binary, File::open("src/english-wordnet-2019.ttl.gz").unwrap())))
     } else if name == "english-wordnet-2019.xml.gz" && *state.site == WordNetSite::English {
        Some(StaticResponse::F((ContentType::Binary, File::open("src/english-wordnet-2019.xml.gz").unwrap())))
     } else if name == "english-wordnet-2019.zip" && *state.site == WordNetSite::English {
        Some(StaticResponse::F((ContentType::Binary, File::open("src/english-wordnet-2019.zip").unwrap())))
    } else if name == "english-wordnet-2020.ttl.gz" && *state.site == WordNetSite::English {
        Some(StaticResponse::F((ContentType::Binary, File::open("src/english-wordnet-2020.ttl.gz").unwrap())))
     } else if name == "english-wordnet-2020.xml.gz" && *state.site == WordNetSite::English {
        Some(StaticResponse::F((ContentType::Binary, File::open("src/english-wordnet-2020.xml.gz").unwrap())))
     } else if name == "english-wordnet-2020.zip" && *state.site == WordNetSite::English {
        Some(StaticResponse::F((ContentType::Binary, File::open("src/english-wordnet-2020.zip").unwrap())))
     } else if name == "english-wordnet-2021.ttl.gz" && *state.site == WordNetSite::English {
        Some(StaticResponse::F((ContentType::Binary, File::open("src/english-wordnet-2021.ttl.gz").unwrap())))
     } else if name == "english-wordnet-2021.xml.gz" && *state.site == WordNetSite::English {
        Some(StaticResponse::F((ContentType::Binary, File::open("src/english-wordnet-2021.xml.gz").unwrap())))
     } else if name == "english-wordnet-2021.zip" && *state.site == WordNetSite::English {
        Some(StaticResponse::F((ContentType::Binary, File::open("src/english-wordnet-2021.zip").unwrap())))
     } else if name == "english-wordnet-2022.ttl.gz" && *state.site == WordNetSite::English {
        Some(StaticResponse::F((ContentType::Binary, File::open("src/english-wordnet-2022.ttl.gz").unwrap())))
     } else if name == "english-wordnet-2022.xml.gz" && *state.site == WordNetSite::English {
        Some(StaticResponse::F((ContentType::Binary, File::open("src/english-wordnet-2022.xml.gz").unwrap())))
     } else if name == "english-wordnet-2022.zip" && *state.site == WordNetSite::English {
        Some(StaticResponse::F((ContentType::Binary, File::open("src/english-wordnet-2022.zip").unwrap())))
     } else if name == "english-wordnet-2023.ttl.gz" && *state.site == WordNetSite::English {
        Some(StaticResponse::F((ContentType::Binary, File::open("src/english-wordnet-2023.ttl.gz").unwrap())))
     } else if name == "english-wordnet-2023.xml.gz" && *state.site == WordNetSite::English {
        Some(StaticResponse::F((ContentType::Binary, File::open("src/english-wordnet-2023.xml.gz").unwrap())))
     } else if name == "english-wordnet-2023.zip" && *state.site == WordNetSite::English {
        Some(StaticResponse::F((ContentType::Binary, File::open("src/english-wordnet-2023.zip").unwrap())))
     } else if name == "english-wordnet-2024.ttl.gz" && *state.site == WordNetSite::English {
        Some(StaticResponse::F((ContentType::Binary, File::open("src/english-wordnet-2024.ttl.gz").unwrap())))
     } else if name == "english-wordnet-2024.xml.gz" && *state.site == WordNetSite::English {
        Some(StaticResponse::F((ContentType::Binary, File::open("src/english-wordnet-2024.xml.gz").unwrap())))
     } else if name == "english-wordnet-2024.zip" && *state.site == WordNetSite::English {
        Some(StaticResponse::F((ContentType::Binary, File::open("src/english-wordnet-2024.zip").unwrap())))
      } else {
        let paths = fs::read_dir("src/res/").expect("No resource directory");

        for path in paths {
            let path_str = path.unwrap().file_name().to_string_lossy().into_owned();
            if path_str == name {
                if name.ends_with(".css") {
                    return Some(StaticResponse::F((ContentType::CSS, File::open("src/res/".to_owned() + &name).unwrap())))
                } else if name.ends_with(".js") {
                    return Some(StaticResponse::F((ContentType::JavaScript, File::open("src/res/".to_owned() + &name).unwrap())))
                }
            }
        }
        None
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
        wordnet.get_by_lemma(id, "en")
            .map_err(|e| format!("Database error: {}", e))?
            .iter().map(|x| (*x).clone()).collect()
    } else if index.starts_with("lemma") {
        wordnet.get_by_lemma(id, &index[6..])
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
//#[response(access_control_allow_origin = "*")]
fn synset(index : &str, id : &str)
        -> Result<RawJson<String>,String> {
    let status = WordNetState::get();
    let synsets = get_synsets(&status.wordnet, &index, &id)?;
    let json = serde_json::to_string(&synsets)
        .map_err(|e| format!("Failed to serialize synset: {}", e))?;
    Ok(RawJson(json))
}

#[get("/json_rel/<id>")]
fn rel_targets(id : &str) -> Result<RawJson<String>, String> {
    let status = WordNetState::get();
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
    Ok(RawJson(json))
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
fn autocomplete_lemma(index : &str, key : &str) -> Result<String, String> {
    let state = WordNetState::get();
    let mut results = Vec::new();
    if index == "lemma" {
        for s in state.wordnet.list_by_lemma(key, "en", 10).map_err(|e| format!("Database error: {}", e))? {
//            if s.starts_with(&key) {
                results.push(AutocompleteResult {
                    display: s.to_string(),
                    item: s.to_string()
                })
//            }
        }   
    } else if index.starts_with("lemma") {
        let lang = index[6..].to_string();
        for s in state.wordnet.list_by_lemma(key, &lang, 10).map_err(|e| format!("Database error: {}", e))? {
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

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ContentNegotiation {
    type Error = String;
    async fn from_request(request: &'r Request<'_>) -> Outcome<ContentNegotiation, String> {
        for value in request.headers().get("Accept") {
            if value.starts_with("text/html") {
                return Outcome::Success(ContentNegotiation::Html);
            } else if value.starts_with("application/rdf+xml") {
                return Outcome::Success(ContentNegotiation::RdfXml);
            } else if value.starts_with("text/turtle") {
                return Outcome::Success(ContentNegotiation::Turtle);
            } else if value.starts_with("application/x-turtle") {
                return Outcome::Success(ContentNegotiation::Turtle);
            } else if value.starts_with("application/json") {
                return Outcome::Success(ContentNegotiation::Json);
            } else if value.starts_with("application/javascript") {
                return Outcome::Success(ContentNegotiation::Json);
            }
        }
        Outcome::Success(ContentNegotiation::Html)
    }
}
    
#[derive(Responder)]
enum NegotiatedResponse {
    Redirect(Redirect),
    Html(RawHtml<&'static str>)
}

fn negotiated(idx : &'static str, key : &str, neg : ContentNegotiation) -> NegotiatedResponse {
    if key.ends_with(".rdf") {
        renegotiated(idx,&key[0..(key.len()-4)], ContentNegotiation::RdfXml)
    } else if key.ends_with(".ttl") {
        renegotiated(idx,&key[0..(key.len()-4)], ContentNegotiation::Turtle)
    } else if key.ends_with(".json") {
        renegotiated(idx,&key[0..(key.len()-5)], ContentNegotiation::Json)
    } else if key.ends_with(".html") {
        renegotiated(idx,&key[0..(key.len()-5)], ContentNegotiation::Html)
    } else {
        match neg {
            ContentNegotiation::Html => { 
                NegotiatedResponse::Html(index())
            },
            ContentNegotiation::RdfXml => {
                NegotiatedResponse::Redirect(Redirect::to(format!("/rdf/{}/{}", idx, key)))
            },
            ContentNegotiation::Turtle => {
                NegotiatedResponse::Redirect(Redirect::to(format!("/ttl/{}/{}", idx, key)))
            },
            ContentNegotiation::Json => {
                NegotiatedResponse::Redirect(Redirect::to(format!("/json/{}/{}", idx, key)))
            }
        }
    }
}

fn renegotiated(idx : &'static str, key : &str, neg : ContentNegotiation) -> NegotiatedResponse {
    if key.ends_with(".rdf") {
        renegotiated(idx,&key[0..(key.len()-4)], ContentNegotiation::RdfXml)
    } else if key.ends_with(".ttl") {
        renegotiated(idx,&key[0..(key.len()-4)], ContentNegotiation::Turtle)
    } else if key.ends_with(".json") {
        renegotiated(idx,&key[0..(key.len()-5)], ContentNegotiation::Json)
    } else if key.ends_with(".html") {
        renegotiated(idx,&key[0..(key.len()-5)], ContentNegotiation::Html)
    } else {
        match neg {
            ContentNegotiation::Html => { 
                NegotiatedResponse::Redirect(Redirect::to(format!("/{}/{}", idx, key)))
            },
            ContentNegotiation::RdfXml => {
                NegotiatedResponse::Redirect(Redirect::to(format!("/rdf/{}/{}", idx, key)))
            },
            ContentNegotiation::Turtle => {
                NegotiatedResponse::Redirect(Redirect::to(format!("/ttl/{}/{}", idx, key)))
            },

            ContentNegotiation::Json => {
                NegotiatedResponse::Redirect(Redirect::to(format!("/json/{}/{}", idx, key)))
            }
        }
    }
}


#[get("/lemma/<key>")]
fn lemma(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { negotiated("lemma", key, neg) }
#[get("/lemma-en/<key>")]
fn lemma_en(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { negotiated("lemma-en", key, neg) }
#[get("/lemma-bg/<key>")]
fn lemma_bg(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { negotiated("lemma-bg", key, neg) }
#[get("/lemma-cs/<key>")]
fn lemma_cs(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { negotiated("lemma-cs", key, neg) }
#[get("/lemma-da/<key>")]
fn lemma_da(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { negotiated("lemma-da", key, neg) }
#[get("/lemma-de/<key>")]
fn lemma_de(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { negotiated("lemma-de", key, neg) }
#[get("/lemma-el/<key>")]
fn lemma_el(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { negotiated("lemma-el", key, neg) }
#[get("/lemma-es/<key>")]
fn lemma_es(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { negotiated("lemma-es", key, neg) }
#[get("/lemma-et/<key>")]
fn lemma_et(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { negotiated("lemma-et", key, neg) }
#[get("/lemma-fi/<key>")]
fn lemma_fi(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { negotiated("lemma-fi", key, neg) }
#[get("/lemma-fr/<key>")]
fn lemma_fr(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { negotiated("lemma-fr", key, neg) }
#[get("/lemma-ga/<key>")]
fn lemma_ga(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { negotiated("lemma-ga", key, neg) }
#[get("/lemma-hr/<key>")]
fn lemma_hr(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { negotiated("lemma-hr", key, neg) }
#[get("/lemma-hu/<key>")]
fn lemma_hu(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { negotiated("lemma-hu", key, neg) }
#[get("/lemma-it/<key>")]
fn lemma_it(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { negotiated("lemma-it", key, neg) }
#[get("/lemma-lt/<key>")]
fn lemma_lt(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { negotiated("lemma-lt", key, neg) }
#[get("/lemma-lv/<key>")]
fn lemma_lv(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { negotiated("lemma-lv", key, neg) }
#[get("/lemma-mt/<key>")]
fn lemma_mt(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { negotiated("lemma-mt", key, neg) }
#[get("/lemma-nl/<key>")]
fn lemma_nl(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { negotiated("lemma-nl", key, neg) }
#[get("/lemma-pl/<key>")]
fn lemma_pl(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { negotiated("lemma-pl", key, neg) }
#[get("/lemma-pt/<key>")]
fn lemma_pt(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { negotiated("lemma-pt", key, neg) }
#[get("/lemma-ro/<key>")]
fn lemma_ro(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { negotiated("lemma-ro", key, neg) }
#[get("/lemma-sk/<key>")]
fn lemma_sk(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { negotiated("lemma-sk", key, neg) }
#[get("/lemma-sl/<key>")]
fn lemma_sl(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { negotiated("lemma-sl", key, neg) }
#[get("/lemma-sv/<key>")]
fn lemma_sv(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { negotiated("lemma-sv", key, neg) }

#[get("/id/<key>")]
fn id(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { negotiated("id", key, neg) }
#[get("/ili/<key>")]
fn ili(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { negotiated("ili", key, neg) }
#[get("/sense_key/<key>")]
fn sense_key(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { negotiated("sense_key", key, neg) }
#[get("/pwn30/<key>")]
fn pwn30(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { negotiated("pwn30", key, neg) }
#[get("/pwn21/<key>")]
fn pwn21(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { negotiated("pwn21", key, neg) }
#[get("/pwn20/<key>")]
fn pwn20(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { negotiated("pwn20", key, neg) }
#[get("/pwn171/<key>")]
fn pwn171(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { negotiated("pwn171", key, neg) }
#[get("/pwn17/<key>")]
fn pwn17(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { negotiated("pwn17", key, neg) }
#[get("/pwn16/<key>")]
fn pwn16(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { negotiated("pwn16", key, neg) }

#[get("/english-wordnet-2019.ttl.gz")]
fn ewn2019ttl() -> Option<(ContentType, File)> {
    let state = WordNetState::get();
    if *state.site.deref() == WordNetSite::English {
        Some((ContentType::Binary, File::open("src/english-wordnet-2019.ttl.gz").unwrap()))
    } else {
        None
    }
}

#[get("/english-wordnet-2019.xml.gz")]
fn ewn2019xml() -> Option<(ContentType, File)> {
    let state = WordNetState::get();
    if *state.site.deref() == WordNetSite::English {
        Some((ContentType::Binary, File::open("src/english-wordnet-2019.xml.gz").unwrap()))
    } else {
        None
    }
}

#[get("/english-wordnet-2019.zip")]
fn ewn2019zip() -> Option<(ContentType, File)> {
    let state = WordNetState::get();
    if *state.site.deref() == WordNetSite::English {
        Some((ContentType::Binary, File::open("src/english-wordnet-2019.zip").unwrap()))
    } else {
        None
    }
}

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
fn wn31(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { 
    if is_old_wn_key(&key) {
        renegotiated("id", &key[1..key.len()], neg) 
    } else {
        renegotiated("lemma", &key[..(key.len()-2)], neg)
    }
}
#[get("/wn30/<key>")]
fn wn30(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { renegotiated("pwn30", key, neg) }
#[get("/wn21/<key>")]
fn wn21(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { renegotiated("pwn21", key, neg) }
#[get("/wn20/<key>")]
fn wn20(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { renegotiated("pwn20", key, neg) }
#[get("/wn171/<key>")]
fn wn171(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { renegotiated("pwn171", key, neg) }
#[get("/wn17/<key>")]
fn wn17(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { renegotiated("pwn17", key, neg) }
#[get("/wn16/<key>")]
fn wn16(key : &str, neg : ContentNegotiation) -> NegotiatedResponse { renegotiated("pwn16", key, neg) }
#[get("/wn31.nt.gz")]
fn wn31ntgz() -> Redirect {
    Redirect::to("/static/wordnet.nt.gz")
}

#[get("/")]
fn index() -> RawHtml<&'static str> {
    let state = WordNetState::get();
    RawHtml(match state.site.deref() {
        WordNetSite::Princeton => include_str!("index.html"),
        WordNetSite::Polylingual => include_str!("polyling-index.html"),
        WordNetSite::English => include_str!("english-index.html")
    })
}

#[get("/about")]
fn about() -> RawHtml<&'static str> {
    RawHtml(include_str!("about.html"))
}

#[get("/license")]
fn license() -> RawHtml<&'static str> {
    RawHtml(include_str!("license.html"))
}


#[get("/ontology")]
fn ontology() -> (ContentType, &'static str) {
    (ContentType::new("application","rdf+xml"), include_str!("ontology.rdf"))
}

#[get("/ontology.html")]
fn ontology_html() -> RawHtml<&'static str> {
    RawHtml(include_str!("ontology.html"))
//    Response::build()
//        .header(html_utf8())
//        .header(CacheControl(vec![CacheDirective::MaxAge(86400u32)]))
//        //.sized_body(File::open("src/ontology.html").unwrap())
//        .sized_body(Cursor::new(include_str!("ontology.html")))
//        .finalize()
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
            "en" => WordNetSite::English,
            _ => return Err("Bad site")
        };
        Ok(Config {
            wn_file: wn_file.to_string(),
            reload: matches.is_present("reload"),
            port,
            site
        })
    }
}

static WORDNETSTATE_WORDNET: Lazy<Mutex<WordNet>> = Lazy::new(|| Mutex::new(WordNet::new()));
static WORDNETSTATE_HANDLEBARS: Lazy<Mutex<Handlebars>> = Lazy::new(|| Mutex::new(Handlebars::new()));
static WORDNETSTATE_SITE: Lazy<Mutex<WordNetSite>> = Lazy::new(|| Mutex::new(WordNetSite::Princeton));

struct WordNetState<'a> {
    wordnet: MutexGuard<'a, WordNet>,
    handlebars: MutexGuard<'a, Handlebars>,
    site : MutexGuard<'a, WordNetSite>
}

impl<'a> WordNetState<'a> {
    fn get() -> WordNetState<'a> {
        WordNetState {
            wordnet: WORDNETSTATE_WORDNET.lock().unwrap(),
            handlebars: WORDNETSTATE_HANDLEBARS.lock().unwrap(),
            site: WORDNETSTATE_SITE.lock().unwrap()
        }
    }
}

fn lemma_escape(h : &handlebars::Helper,
                _ : &Handlebars,
                rc : &mut handlebars::RenderContext) -> Result<(), handlebars::RenderError> {
    let param = h.param(0).and_then(|v| v.value().as_str()).unwrap_or("");
    rc.writer.write(param.replace(" ", "_").into_bytes().as_ref())?;
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

fn check_path(path : &str) -> bool {
    let p = Path::new(path);
    if p.exists() {
        true
    } else {
        match env::current_dir() {
            Ok(home) => {
                eprintln!("Could not find required file at {} (home is {})", 
                          p.display(), home.display());
            },
            Err(_) => {
                eprintln!("Could not find required file at {} or deduce home",
                          p.display());
            }
        }
        false
    }
}


fn prepare_server(config : Config) -> Result<(), String> {
    let mut resources = true;
    resources = config.reload || check_path("wordnet.db") && resources;
    resources = check_path("wordnet.nt.gz") && resources;
    resources = check_path("src") && resources;
    resources = check_path("flags") && resources;
    if !resources {
        exit(-1);
    }
    let mut handlebars = Handlebars::new();
    handlebars.register_template_string("xml", include_str!("xml.hbs"))
        .expect("Could not load xml.hbs");
    handlebars.register_template_string("xml-poly", include_str!("xml-poly.hbs"))
        .expect("Could not load xml-poly.hbs");
    handlebars.register_template_string("xml-english", include_str!("xml-english.hbs"))
        .expect("Could not load xml-english.hbs");
    handlebars.register_template_string("ttl", include_str!("ttl.hbs"))
        .expect("Could not load ttl.hbs");
    handlebars.register_template_string("rdfxml", include_str!("rdfxml.hbs"))
        .expect("Could not load rdfxml.hbs");
    handlebars.register_helper("lemma_escape", Box::new(lemma_escape));
    handlebars.register_helper("long_pos", Box::new(long_pos));
    let wordnet = if config.reload  {
        eprintln!("Loading WordNet data");
        if config.site == WordNetSite::Princeton {
            wordnet_read::load_pwn(config.wn_file)
                .map_err(|e| format!("Failed to load WordNet: {}", e))?
        } else if config.site == WordNetSite::English {
            wordnet_read::load_enwn(config.wn_file)
                .map_err(|e| format!("Failed to load WordNet: {}", e))?
        } else {
            wordnet_read::load_gwn(config.wn_file)
                .map_err(|e| format!("Failed to load WordNet: {}", e))?
        }
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
    let mut wordnet_state = WORDNETSTATE_WORDNET.lock().unwrap();
    *wordnet_state = wordnet;
    let mut handlebars_state = WORDNETSTATE_HANDLEBARS.lock().unwrap();
    *handlebars_state = handlebars;
    let mut site_state = WORDNETSTATE_SITE.lock().unwrap();
    *site_state = config.site;
    Ok(())
}

#[derive(Clone,Debug,PartialEq)]
enum WordNetSite {
    Princeton,
    Polylingual,
    English
}

#[launch]
fn rocket() -> _ {
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
             .value_name("princeton|polylingual|en")
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
                    let mut rocket_config = RocketConfig::release_default();
                    rocket_config.port = config.port;
                    rocket_config.workers = 30;
                    rocket::custom(&rocket_config)
                        .manage(state)
                        .mount("/", routes![
                                about, ontology, ontology_html, license,
                                get_xml, get_ttl, get_rdf, rel_targets,
                                index, synset, get_flag,
                                autocomplete_lemma, get_static,
                                lemma_bg, lemma_cs, lemma_da, lemma_de,
                                lemma_el, lemma_en, lemma_es, lemma_et,
                                lemma_fi, lemma_fr, lemma_ga, lemma_hr,
                                lemma_hu, lemma_it, lemma_lt, lemma_lv,
                                lemma_mt, lemma_nl, lemma_pl, lemma_pt,
                                lemma_ro, lemma_sk, lemma_sl, lemma_sv,
                                lemma, id, ili, sense_key, 
                                wn30, wn21, wn20, wn17,
                                wn171, wn16, wn31, wn31ntgz,
                                pwn30, pwn21, pwn20, pwn17,
                                pwn171, pwn16, ewn2019zip, 
                                ewn2019xml, ewn2019ttl])
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
