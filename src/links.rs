use std::path::Path;
use wordnet::WordNetLoadError;
use std::collections::HashMap;
use std::io::BufReader;
use std::fs::{read_dir,File};
use xml::reader::{EventReader, XmlEvent};

fn load_verbs<P : AsRef<Path>>(path : P) -> Result<HashMap<String,Vec<String>>, WordNetLoadError> {
    let file = BufReader::new(File::open(path)?);
    let parse = EventReader::new(file);

    let mut vnid : Option<String> = None;
    let mut vn2wn : HashMap<String, Vec<String>> = HashMap::new();

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
                            vn2wn.entry(vn.clone())
                                .or_insert_with(|| Vec::new())
                                .extend(elems.map(|a| a.to_string()));
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

    Ok(vn2wn)
}

fn load_all_verbs() -> Result<HashMap<String, Vec<String>>, WordNetLoadError> {
    let paths = read_dir("data/verbnet")?;
    let mut verbnet_links = HashMap::new();

    for path in paths {
        let path = path?.path();
        if path.ends_with(".xml") {
            verbnet_links.extend(load_verbs(path)?);
        }
    }
    Ok(verbnet_links)
}


//fn load_w3c(wn20 : HashMap<WNKey, WNKey>) -> Result<HashMap<String, Vec<String>>> {
//
//}




