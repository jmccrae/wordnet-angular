use std::path::Path;
use wordnet::{WNKey,WordNetLoadError,Synset};
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
pub fn load_links(wn20 : &HashMap<WNKey,WNKey>,
                  ili : &HashMap<String,WNKey>,
                  synsets : &mut HashMap<WNKey, Synset>) {
    eprintln!("Loading VerbNet");
    let verbs = load_all_verbs().unwrap_or_else(|e| {
        eprintln!("Failed to load VerbNet: {}", e);
        HashMap::new()
    });
    eprintln!("Loading W3C Links");
    let w3c = load_w3c(&wn20).unwrap_or_else(|e| {
        eprintln!("Failed to load W3C: {}", e);
        HashMap::new()
    });
    eprintln!("Loading Wikipedia Links");
    let wwim = load_wwim(&ili).unwrap_or_else(|e| {
        eprintln!("Failed to load Wikipedia Links: {}", e);
        HashMap::new()
    });

    for (_,synset) in synsets.iter_mut() {
       if let Some(w) = w3c.get(&synset.id) {
           synset.links.push(Link { link_type: LinkType::W3C, target: w.to_string() });
       }
       if let Some(vs) = wwim.get(&synset.id) {
           for v in vs {
               synset.links.push(Link { link_type: LinkType::Wikipedia, target: v.to_string() });
           }
       }
       for s in synset.lemmas.iter() {
           if let Some(vs) = verbs.get(&s.sense_key) {
               for v in vs {
                   if !synset.links.iter().any(|l| l.target == *v) {
                       synset.links.push(Link {
                           link_type: LinkType::VerbNet,
                           target: v.to_string()
                       });
                   }
               }
           }
       }
    }
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


fn load_w3c(wn20s : &HashMap<WNKey, WNKey>) -> Result<HashMap<WNKey, String>,WordNetLoadError> {
    let file = BufReader::new(File::open("data/w3c-wn20.csv")?);

    let mut map = HashMap::new();

    for line in file.lines() {
        let line = line?;
        let mut elems = line.split(",");
        if let Some(url) = elems.next() {
            if let Some(wn20) = elems.next() {
                let wn20key = WNKey::from_str(wn20)?;
                if let Some(wn30) = wn20s.get(&wn20key) {
                    map.insert(wn30.clone(), url[1..(url.len()-1)].to_string());
                }
            }
        }
    }

    Ok(map)
}

fn load_wwim(ilis : &HashMap<String, WNKey>) -> Result<HashMap<WNKey, Vec<String>>, WordNetLoadError> {
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
                    if let Some(id) = ilis.get(&ili) {
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




