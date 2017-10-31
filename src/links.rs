use std::path::Path;
use wordnet::{WNKey,WordNetLoadError,WordNet};
use std::collections::HashMap;
use std::io::{BufRead,BufReader};
use std::fs::{read_dir,File};
use xml::reader::{EventReader, XmlEvent};
use std::str::FromStr;
use std::ffi::OsStr;

#[derive(Clone,Debug,Serialize,Deserialize)]
pub enum LinkType { VerbNet, W3C, Wikipedia }

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Link {
    pub link_type : LinkType,
    pub target : String
}

/// Load all links to VerbNet, W3C and Wikipedia
pub fn load_links(wordnet : &mut WordNet) -> Result<(), WordNetLoadError> {
    {
        eprintln!("Loading VerbNet");
        let verbs = load_all_verbs().unwrap_or_else(|e| {
            eprintln!("Failed to load VerbNet: {}", e);
            HashMap::new()
        });
        for (sense_key, vs) in verbs {
            if let Some(key) = wordnet.get_id_by_sense_key(&sense_key)? {
                for v in vs {
                    wordnet.insert_link(&key, LinkType::VerbNet, v)?;
                }
            }
        }
    }
    {
        eprintln!("Loading W3C Links");
        let w3c = load_w3c(&wordnet).unwrap_or_else(|e| {
            eprintln!("Failed to load W3C: {}", e);
            HashMap::new()
        });
        for (key, target) in w3c {
            wordnet.insert_link(&key, LinkType::W3C, target)?;
        }
    }
    {
        eprintln!("Loading Wikipedia Links");
        let wwim = load_wwim(&wordnet).unwrap_or_else(|e| {
            eprintln!("Failed to load Wikipedia Links: {}", e);
            HashMap::new()
        });
        for (key, targets) in wwim {
            for target in targets {
                wordnet.insert_link(&key, LinkType::Wikipedia, target)?;
            }
        }
    }
    Ok(())
}

fn load_verbs<P : AsRef<Path>>(path : P) -> Result<HashMap<String,Vec<String>>, WordNetLoadError> {
    let file = BufReader::new(File::open(path)?);
    let parse = EventReader::new(file);

    let mut vnid : Option<String> = None;
    let mut wn2vn : HashMap<String, Vec<String>> = HashMap::new();

    for e in parse {
        match e {
            Ok(XmlEvent::StartElement{ name, attributes, .. }) => {
                if name.local_name == "VNCLASS" {
                    vnid = attributes.iter().find(|a| a.name.local_name == "ID")
                        .map(|a| a.value.clone());
                } else if name.local_name == "MEMBER" {
                    let wn_str = attributes.iter()
                        .find(|a| a.name.local_name == "wn")
                        .map(|a| a.value.clone())
                        .unwrap_or_else(|| "".to_string());
                    let elems = wn_str.split(" ");
                    match vnid {
                        Some(ref vn) => {
                            for wn in elems {
                                wn2vn.entry(format!("{}::", wn))
                                    .or_insert_with(|| Vec::new())
                                    .push(vn.clone());
                            }
                        },
                        None => {
                            eprintln!("Members without id");
                        }
                    }
                }
            },
            Ok(_) => {},
            Err(e) => return Err(WordNetLoadError::Xml(e))
        }
    }

    Ok(wn2vn)
}

fn load_all_verbs() -> Result<HashMap<String, Vec<String>>, WordNetLoadError> {
    let paths = read_dir("data/verbnet")?;
    let mut verbnet_links = HashMap::new();

    for path in paths {
        let path = path?.path();
        if path.extension() == Some(OsStr::new("xml")) {
            verbnet_links.extend(load_verbs(path)?);
        }
    }
    Ok(verbnet_links)
}


fn load_w3c(wordnet : &WordNet) -> Result<HashMap<WNKey, String>,WordNetLoadError> {
    let file = BufReader::new(File::open("data/w3c-wn20.csv")?);

    let mut map = HashMap::new();

    for line in file.lines() {
        let line = line?;
        let mut elems = line.split(",");
        if let Some(url) = elems.next() {
            if let Some(wn20) = elems.next() {
                let wn20key = WNKey::from_str(wn20)?;
                if let Some(wn30) = wordnet.get_id_by_old_id("pwn20", &wn20key)
                        .expect("WordNet 2.0 Index not loaded but loading W3C") {
                    map.insert(wn30.clone(), url[1..(url.len()-1)].to_string());
                }
            }
        }
    }

    Ok(map)
}

fn load_wwim(wordnet : &WordNet) -> Result<HashMap<WNKey, Vec<String>>, WordNetLoadError> {
    let file = BufReader::new(File::open("data/ili-map-dbpedia.ttl")?);

    let mut map = HashMap::new();

    for line in file.lines() {
        let line = line?;
        if !line.starts_with("#") {
            let mut elems = line.split(" ");
            if let Some(ili_url) = elems.next() {
                elems.next();
                if let Some(dbpedia_url) = elems.next() {
                    let ili = ili_url[30..(ili_url.len()-1)].to_string();
                    let wiki_id = dbpedia_url[29..(dbpedia_url.len()-1)].to_string();
                    if let Some(id) = wordnet.get_id_by_ili(&ili)? {
                        map.entry(id.clone())
                            .or_insert_with(|| Vec::new())
                            .push(wiki_id);
                    }
                }
            }
        }
    }

    Ok(map)
}




