use std::fs::File;
use std::path::Path;
use std::io::{BufRead,BufReader};
use xml::reader::{EventReader, XmlEvent};
use xml::attribute::OwnedAttribute;
use std::collections::HashMap;
use stable_skiplist::OrderedSkipList;
use glosstag::{Gloss,read_glosstag_corpus};

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Synset {
    pub definition : String,
    pub lemmas : Vec<Sense>,
    pub id : String,
    pub ili : String,
    pub pos : String,
    pub subject : String,
    pub relations : Vec<Relation>,
    pub old_keys : HashMap<String, Vec<String>>,
    pub gloss : Option<Vec<Gloss>>
}

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Sense {
    pub lemma : String,
    pub forms : Vec<String>,
    pub sense_key : String,
    pub subcats : Vec<String>
}

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Relation {
    pub src_word : Option<String>,
    pub trg_word : Option<String>,
    pub rel_type : String,
    pub target : String
}

pub struct WordNet {
    pub synsets : HashMap<String, Synset>,
    pub by_lemma : HashMap<String, Vec<String>>,
    pub by_ili : HashMap<String, String>,
    pub by_sense_key : HashMap<String, String>,
    pub by_old_id : HashMap<String, HashMap<String, String>>,
    pub id_skiplist : OrderedSkipList<String>,
    pub lemma_skiplist : OrderedSkipList<String>,
    pub ili_skiplist : OrderedSkipList<String>,
    pub sense_key_skiplist : OrderedSkipList<String>,
    pub old_skiplist : HashMap<String, OrderedSkipList<String>>
}

fn attr_value(attr : &Vec<OwnedAttribute>, name : &'static str) -> Option<String> {
    attr.iter().find(|a| a.name.local_name == name).map(|a| a.value.clone())
}

fn clean_id(s : &str) -> String {
    s.chars().skip(5).collect()
}

impl WordNet {
    pub fn load<P : AsRef<Path>>(path : P) -> Result<WordNet, WordNetLoadError> {
        let file = BufReader::new(File::open(path)?);

        let parse = EventReader::new(file);

        let mut lexical_entry_id : Option<String> = None;
        let mut entry_lemma = None;
        let mut sense_keys = HashMap::new();
        let mut entry_id_to_lemma = HashMap::new();
        let mut synset_to_entry = HashMap::new();
        let mut sense_to_lemma = HashMap::new();
        let mut sense_to_synset = HashMap::new();
        let mut subcats = HashMap::new();
        let mut synset = None;
        let mut synset_id = None;
        let mut synset_ili_pos_subject = None;
        let mut in_def = false;
        let mut definition = None;
        let mut synsets = HashMap::new();
        let mut relations = HashMap::new();

        let mut lemma_skiplist = OrderedSkipList::new();

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
                        entry_lemma = Some(lemma.clone());
                        entry_id_to_lemma.insert(entry_id, lemma.clone());
                        if !lemma_skiplist.contains(&lemma) {
                            lemma_skiplist.insert(lemma);
                        }
                    } else if name.local_name == "Sense" {
                        let entry_id = match lexical_entry_id {
                            Some(ref i) => i.clone(),
                            None => {
                                return Err(WordNetLoadError::Schema(
                                    "Lemma outside of LexicalEntry"))
                            }
                         };
                         let target = clean_id(&attr_value(&attributes, "synset")
                            .ok_or_else(|| WordNetLoadError::Schema(
                                    "Sense does not have a synset"))?);
                         match attr_value(&attributes, "identifier") {
                            Some(i) => {
                                sense_keys.entry(entry_id.clone())
                                    .or_insert_with(|| HashMap::new())
                                    .insert(target.clone(), i);
                            },
                            None => {}
                         };
                         synset = Some(target.clone());
                         synset_to_entry.entry(target.clone())
                            .or_insert_with(|| Vec::new())
                            .push(entry_id);
                         let sense_id = attr_value(&attributes, "id")
                            .ok_or_else(|| WordNetLoadError::Schema(
                                "Sense without id"))?;
                         let word = entry_lemma.clone()
                            .ok_or_else(|| WordNetLoadError::Schema(
                                "SenseRelation before Lemma"))?;
                         sense_to_lemma.insert(sense_id.clone(), word);
                         sense_to_synset.insert(sense_id, target);
                    } else if name.local_name == "SenseRelation" {
                        let typ = attr_value(&attributes, "relType")
                            .ok_or_else(|| WordNetLoadError::Schema(
                                "SenseRelation without relType"))?;
                        let targ = attr_value(&attributes, "target")
                            .ok_or_else(|| WordNetLoadError::Schema(
                                "SenseRelation without target"))?;
                        let ss = synset.clone()
                            .ok_or_else(|| WordNetLoadError::Schema(
                                "SenseRelation outside of Sense"))?;
                        let word = entry_lemma.clone()
                            .ok_or_else(|| WordNetLoadError::Schema(
                                "SenseRelation before Lemma"))?;
                        relations.entry(ss)
                            .or_insert_with(|| Vec::new())
                            .push(Relation {
                                src_word: Some(word),
                                trg_word: None,
                                rel_type: typ,
                                target: targ
                            });
                    } else if name.local_name == "SyntacticBehaviour" {
                        let entry_id = lexical_entry_id.clone()
                            .ok_or_else(|| WordNetLoadError::Schema(
                                "SyntacticBehaviour outside of LexicalEntry"))?;
                        let subcat = attr_value(&attributes, "subcategorizationFrame")
                            .ok_or_else(|| WordNetLoadError::Schema(
                                "SyntacticBehaviour has no subcategorizationFrame"))?;
                        subcats.entry(entry_id)
                            .or_insert_with(|| Vec::new())
                            .push(subcat);
                    } else if name.local_name == "Synset" {
                        synset_id = Some(clean_id(&attr_value(&attributes, "id")
                            .ok_or_else(|| WordNetLoadError::Schema(
                                    "Synset does not have an id"))?));
                        synset_ili_pos_subject = Some((
                            attr_value(&attributes, "ili")
                            .ok_or_else(|| WordNetLoadError::Schema(
                                "Synset does not have ILI"))?,
                            attr_value(&attributes, "partOfSpeech")
                            .ok_or_else(|| WordNetLoadError::Schema(
                                "Synset does not have ILI"))?,
                            attr_value(&attributes, "subject")
                            .ok_or_else(|| WordNetLoadError::Schema(
                                "Synset does not have ILI"))?));
                    } else if name.local_name == "Definition" {
                        in_def = true;
                    } else if name.local_name == "SynsetRelation" {
                        let typ = attr_value(&attributes, "relType")
                            .ok_or_else(|| WordNetLoadError::Schema(
                                "SynsetRelation without relType"))?;
                        let targ = clean_id(&attr_value(&attributes, "target")
                            .ok_or_else(|| WordNetLoadError::Schema(
                                "SynsetRelation without target"))?);
                        let ss = synset_id.clone()
                            .ok_or_else(|| WordNetLoadError::Schema(
                                "SynsetRelation outside of Sense"))?;
                        relations.entry(ss)
                            .or_insert_with(|| Vec::new())
                            .push(Relation {
                                src_word: None,
                                trg_word: None,
                                rel_type: typ,
                                target: targ
                            });
    }
                },
                Ok(XmlEvent::EndElement { name, .. }) => {
                    if name.local_name == "LexicalEntry" {
                        lexical_entry_id = None;
                        entry_lemma = None;
                    } else if name.local_name == "Sense" {
                        synset = None;
                    } else if name.local_name == "Synset" {
                        let defn = definition.ok_or(
                            WordNetLoadError::Schema(
                                "Synset without definition"))?;
                        let ssid = synset_id.unwrap();
                        let entries = synset_to_entry.get(&ssid)
                            .map(|x| x.clone())
                            .unwrap_or_else(|| Vec::new())
                            .iter()
                            .map(|x| {
                                Sense {
                                    lemma: entry_id_to_lemma.get(x)
                                        .expect("Entry must have lemma")
                                        .clone(),
                                    // TODO
                                    forms: Vec::new(),
                                    sense_key: sense_keys[x][&ssid].clone(),
                                    subcats: subcats.get(x)
                                        .map(|x| x.clone())
                                        .unwrap_or_else(|| Vec::new())
                                        .clone()
                                }
                            })
                            .collect();
                        let (ili, pos, subject) = synset_ili_pos_subject.clone()
                            .expect("ILI/POS/Subject not set");
                        let rels = relations.get(&ssid)
                            .map(|x| x.clone())
                            .unwrap_or_else(|| Vec::new())
                            .iter()
                            .map(|r| {
                                if r.src_word.is_some() {
                                    Relation {
                                        src_word: r.src_word.clone(),
                                        trg_word: Some(sense_to_lemma[&r.target].clone()),
                                        rel_type: r.rel_type.clone(),
                                        target: sense_to_synset[&r.target].clone()
                                    }
                                } else {
                                    r.clone()
                                }
                            })
                            .collect();
                        synsets.insert(ssid.clone(),
                            Synset {
                                definition: defn,
                                lemmas: entries,
                                id: ssid,
                                ili: ili,
                                pos: pos,
                                subject: subject,
                                relations: rels,
                                old_keys: HashMap::new(),
                                gloss: None
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
        let mut wordnet = WordNet{
            synsets: synsets,
            by_lemma: HashMap::new(),
            by_ili: HashMap::new(),
            by_sense_key: HashMap::new(),
            by_old_id: HashMap::new(),
            lemma_skiplist: lemma_skiplist,
            ili_skiplist: OrderedSkipList::new(),
            id_skiplist: OrderedSkipList::new(),
            sense_key_skiplist: OrderedSkipList::new(),
            old_skiplist: HashMap::new()
        };
        build_indexes(&mut wordnet);
        build_tabs(&mut wordnet)?;
        build_glosstags(&mut wordnet)?;
        Ok(wordnet)
    }
}

fn build_glosstags(wordnet : &mut WordNet)
         -> Result<(), WordNetLoadError> {
    let ref wn30_idx = wordnet.by_old_id["pwn30"];
    let ref sense_keys = wordnet.by_sense_key;
    let mut result = read_glosstag_corpus("data/merged/adj.xml", wn30_idx, sense_keys)?;
    result.extend(read_glosstag_corpus("data/merged/adv.xml", wn30_idx, sense_keys)?);
    result.extend(read_glosstag_corpus("data/merged/noun.xml", wn30_idx, sense_keys)?);
    result.extend(read_glosstag_corpus("data/merged/verb.xml", wn30_idx, sense_keys)?);
    for (k,v) in result.iter() {
        wordnet.synsets.get_mut(k)
            .map(|x| {
                x.gloss = Some(v.clone())
            });
    }
    Ok(())
}

fn build_tab<P : AsRef<Path>>(file : P, 
    index : &str,
    by_ili : &HashMap<String, String>,
    synsets : &mut HashMap<String, Synset>,
    by_tab : &mut HashMap<String, String>,
    skiplist : &mut OrderedSkipList<String>) -> Result<(),::std::io::Error> {
    let file = BufReader::new(File::open(file)?);
    for line in file.lines() {
        let line = line?;
        let mut elems = line.split("\t");
        match elems.next() {
            Some(ili) => {
                match elems.next() {
                    Some(id) => {
                        skiplist.insert(id.to_string());
                        match by_ili.get(ili) {
                            Some(wnid) => {
                                by_tab.insert(id.to_string(), wnid.to_string());
                                synsets.entry(wnid.clone())
                                    .or_insert_with(|| panic!("ILI not in WordNet??"))
                                    .old_keys.entry(index.to_string())
                                    .or_insert_with(|| Vec::new())
                                    .push(id.to_string());
                            },
                            None => {}
                        };
                    },
                    None => {}
                }
            },
            None => {}
        }
    }
    Ok(())
}
    

fn build_indexes(wordnet : &mut WordNet) {
    for (id, synset) in wordnet.synsets.iter() {
        wordnet.ili_skiplist.insert(synset.ili.clone());
        wordnet.id_skiplist.insert(id.clone());
        wordnet.by_ili.insert(synset.ili.clone(), id.clone());
        for sense in synset.lemmas.iter() {
            wordnet.by_sense_key.insert(sense.sense_key.clone(), id.clone());
            wordnet.sense_key_skiplist.insert(sense.sense_key.clone());
            wordnet.by_lemma.entry(sense.lemma.clone())
                .or_insert_with(|| Vec::new())
                .push(id.clone());
        }
    }
}

fn build_tabs(wordnet : &mut WordNet) -> Result<(),::std::io::Error> {
    for tab in ["pwn15", "pwn16", "pwn17", "pwn171", "pwn20",
                "pwn21", "pwn30"].iter() {
        let mut map = HashMap::new();
        let mut list = OrderedSkipList::new();
        let path = format!("data/ili-map-{}.tab", tab);
        build_tab(path, tab, &wordnet.by_ili, 
            &mut wordnet.synsets, &mut map, &mut list)?;
        wordnet.by_old_id.insert(tab.to_string(), map);
        wordnet.old_skiplist.insert(tab.to_string(), list);
    }
    Ok(())
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
