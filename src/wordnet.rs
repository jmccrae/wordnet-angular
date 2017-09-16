use std::fs::File;
use std::path::Path;
use std::io::BufReader;
use xml::reader::{EventReader, XmlEvent};
use xml::attribute::OwnedAttribute;
use std::collections::HashMap;

pub struct Synset {
    pub definition : String,
    pub lemmas : Vec<String>
}

pub struct WordNet {
    pub synsets : HashMap<String, Synset>
}

fn attr_value(attr : &Vec<OwnedAttribute>, name : &'static str) -> Option<String> {
    attr.iter().find(|a| a.name.local_name == name).map(|a| a.value.clone())
}

impl WordNet {
    pub fn load<P : AsRef<Path>>(path : P) -> Result<WordNet, WordNetLoadError> {
        let file = BufReader::new(File::open(path)?);

        let parse = EventReader::new(file);

        let mut lexical_entry_id : Option<String> = None;
        let mut entry_id_to_lemma = HashMap::new();
        let mut synset_to_entry = HashMap::new();
        let mut synset_id = None;
        let mut in_def = false;
        let mut definition = None;
        let mut synsets = HashMap::new();

        for e in parse {
            match e {
                Ok(XmlEvent::StartElement{ name, attributes, .. }) => {
                    if name.local_name == "LexicalEntry" {
                        match attr_value(&attributes, "id") {
                            Some(id) => {
                                lexical_entry_id = Some(id)
                            },
                            None => {
                                return Err(WordNetLoadError::Schema(
                                    "LexicalEntry does not have an ID"));
                            }
                        }
                    } else if name.local_name == "Lemma" {
                        let entry_id = match lexical_entry_id {
                            Some(ref i) => i.clone(),
                            None => {
                                return Err(WordNetLoadError::Schema(
                                    "Lemma outside of LexicalEntry"))
                            }
                        };
                        let lemma = match attr_value(&attributes, "writtenForm") {
                            Some(l) => l,
                            None => {
                                return Err(WordNetLoadError::Schema(
                                    "Lemma does not have writtenForm"));
                            }
                        };
                        entry_id_to_lemma.insert(entry_id, lemma);
                    } else if name.local_name == "Sense" {
                        let entry_id = match lexical_entry_id {
                            Some(ref i) => i.clone(),
                            None => {
                                return Err(WordNetLoadError::Schema(
                                    "Lemma outside of LexicalEntry"))
                            }
                         };
                         let target = match attr_value(&attributes, "synset") {
                            Some(s) => s,
                            None => {
                                return Err(WordNetLoadError::Schema(
                                    "Sense does not have a synset"));
                            }
                        };
                        synset_to_entry.entry(target)
                            .or_insert_with(|| Vec::new())
                            .push(entry_id);
                    } else if name.local_name == "Synset" {
                        match attr_value(&attributes, "id") {
                            Some(i) => { synset_id = Some(i) },
                            None => {
                                return Err(WordNetLoadError::Schema(
                                    "Synset does not have an id"));
                            }
                        };
                    } else if name.local_name == "Definition" {
                        in_def = true;
                    }
                },
                Ok(XmlEvent::EndElement { name, .. }) => {
                    if name.local_name == "LexicalEntry" {
                        lexical_entry_id = None
                    } else if name.local_name == "Synset" {
                        let defn = definition.ok_or(
                            WordNetLoadError::Schema(
                                "Synset without definition"))?;
                        let ssid = synset_id.unwrap();
                        let entries = synset_to_entry.get(&ssid)
                            .map(|x| x.clone())
                            .unwrap_or_else(|| Vec::new());
                        synsets.insert(ssid,
                            Synset {
                                definition: defn,
                                lemmas: entries
                            });
                            
                        synset_id = None;
                        definition = None;
                    } else if name.local_name == "Definition" {
                        in_def = false;
                    }
                },
                Ok(XmlEvent::Characters(s)) => {
                    if in_def {
                        definition = Some(s);
                    }
                },
                Ok(_) => {},
                Err(e) => { return Err(WordNetLoadError::Xml(e)); }
            }
        }
        Ok(WordNet{
            synsets: synsets
        })
    }
}

quick_error! {
    #[derive(Debug)]
    pub enum WordNetLoadError {
        Io(err: ::std::io::Error) { 
            from()
            cause(err)
        }
        Xml(err: ::xml::reader::Error) {
            from()
            cause(err)
        }
        Schema(msg : &'static str) {
            description(msg)
        }
    }
}
