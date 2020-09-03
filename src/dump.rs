#![allow(dead_code)]
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

mod wordnet;
mod wordnet_model;
mod omwn;
mod links;
mod glosstag;
mod wordnet_read;

use wordnet::{WNKey,WordNet};
use wordnet_model::Synset;
use std::collections::HashMap;
use clap::{App,Arg};
use handlebars::{Handlebars};
use std::str::FromStr;


#[derive(Clone,Debug,Serialize,Deserialize)]
struct SynsetsHB {
    synsets : Vec<Synset>,
    entries : HashMap<String, Vec<Synset>>,
    index : String,
    name : String,
    license : &'static str,
    site : &'static str

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

fn make_synsets_hb(synsets : Vec<Synset>, index : &str, 
                   name : &str, site : &WordNetSite) -> SynsetsHB {
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
        synsets: synsets,
        entries: entries,
        index : index.to_owned(),
        name: name.to_owned(),
        license: license,
        site: site_url
    }
}

fn lemma_escape(h : &handlebars::Helper,
                _ : &Handlebars,
                rc : &mut handlebars::RenderContext) -> Result<(), handlebars::RenderError> {
    let param = h.param(0).and_then(|v| v.value().as_str()).unwrap_or("");
    rc.writer.write(param.replace(" ", "_").into_bytes().as_ref())?;
    Ok(())
}

fn lemma_escape2(h : &handlebars::Helper,
                _ : &Handlebars,
                rc : &mut handlebars::RenderContext) -> Result<(), handlebars::RenderError> {
    let param = h.param(0).and_then(|v| v.value().as_str()).unwrap_or("");
    rc.writer.write(param[0..param.len()-2].replace(" ", "_").into_bytes().as_ref())?;
    Ok(())
}

fn escape_quote(h : &handlebars::Helper,
    _ : &Handlebars,
    rc : &mut handlebars::RenderContext) -> Result<(), handlebars::RenderError> {
    let param = h.param(0).and_then(|v| v.value().as_str()).unwrap_or("");
    rc.writer.write(param.replace("\"","\\\"").into_bytes().as_ref())?;
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

#[derive(Clone,Debug,PartialEq)]
enum WordNetSite {
    Princeton,
    Polylingual,
    English
}



fn main() {
    let app = App::new("wordnet-rdf-dump")
        .version("1.0")
        .author("John P. McCrae <john@mccr.ae>")
        .about("WordNet Angular RDF Dump Utility")
        .arg(Arg::with_name("site")
             .short("s")
             .value_name("princeton|polylingual|en")
             .help("The site design to use")
             .takes_value(true))
        .arg(Arg::with_name("pos")
             .long("pos")
             .value_name("POS")
             .help("Only dump the particular part of speech")
             .takes_value(true))
        .arg(Arg::with_name("wn")
            .long("wn")
            .value_name("wn31.xml")
            .help("The WordNet file in GWC LMF-XML format, e.g., http://john.mccr.ae/wn31.xml. Default is data/wn31.xml. If specified the dump program will first load this database")
            .takes_value(true));

    let matches = app.clone().get_matches();
    let site = match matches.value_of("site").unwrap_or("princeton") {
            "princeton" => WordNetSite::Princeton,
            "polylingual" => WordNetSite::Polylingual,
            "en" => WordNetSite::English,
            _ => panic!("Bad site")
        };
    match matches.value_of("wn") {
        Some(wn_file) => {
            eprintln!("Loading WordNet data");
            if site == WordNetSite::Princeton {
                wordnet_read::load_pwn(wn_file)
                    .expect("Failed to load WordNet");
            } else if site == WordNetSite::English {
                wordnet_read::load_enwn(wn_file)
                    .expect("Failed to load WordNet");
            } else {
                wordnet_read::load_gwn(wn_file)
                    .expect("Failed to load WordNet");
            }
        },
        None => {}
    };
    let wordnet = wordnet::WordNet::new();
    let mut handlebars = Handlebars::new();
    handlebars.register_template_string("ttl", include_str!("ttl-dump.hbs"))
        .expect("Could not load ttl.hbs");
    handlebars.register_helper("lemma_escape", Box::new(lemma_escape));
    handlebars.register_helper("lemma_escape2", Box::new(lemma_escape2));
    handlebars.register_helper("escape_quote", Box::new(escape_quote));
    handlebars.register_helper("long_pos", Box::new(long_pos));
    println!("@prefix dc: <http://purl.org/dc/terms/> .
@prefix ili: <http://ili.globalwordnet.org/ili/> .
@prefix lime: <http://www.w3.org/ns/lemon/lime#> .
@prefix ontolex: <http://www.w3.org/ns/lemon/ontolex#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix schema: <http://schema.org/> .
@prefix skos: <http://www.w3.org/2004/02/skos/core#> .
@prefix synsem: <http://www.w3.org/ns/lemon/synsem#> .
@prefix wn: <https://globalwordnetnet.github.io/schemas/wn#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .");
    if site == WordNetSite::Princeton {
        println!("@prefix wordnetlicense: <http://wordnet.princeton.edu/wordnet/license/> .
@prefix pwnlemma: <http://wordnet-rdf.princeton.edu/rdf/lemma/> .
@prefix pwnid: <http://wordnet-rdf.princeton.edu/id/> .");
    } else if site == WordNetSite::English {
        println!("@prefix wordnetlicense: <https://github.com/globalwordnet/english-wordnet/blob/master/LICENSE.md> .
@prefix pwnlemma: <https://en-word.net/lemma/> .
@prefix pwnid: <https://en-word.net/id/> .

<https://en-word.net/> a lime:Lexicon, skos:ConceptSet ;
  lime:language \"en\" ;
  lime:linguisticCatalog wn: ;");
println!("
  lime:lexicalEntries {} ;
  lime:concepts {} .
", wordnet.entries().expect("DB error"), wordnet.synsets().expect("DB error"));
    }

    let filter = matches.value_of("pos").unwrap_or("");

    for synset_id in wordnet.get_synset_ids().expect("Could not read database") {
        if synset_id.ends_with(filter) {
            println!("{}", handlebars.render("ttl", 
                &make_synsets_hb(get_synsets(&wordnet, "id", &synset_id.to_string()).
                                 expect("Could not get synsets"),"id",&synset_id.to_string(), &site))
                     .expect("Could not apply template"));
        }
    }
}
