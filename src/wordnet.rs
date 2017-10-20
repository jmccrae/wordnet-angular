//! Functions for handling the in-memory model of WordNet and loading it form
//! disk
use glosstag::{Gloss,read_glosstag_corpus};
use omwn::load_omwn;
use serde::de::{Visitor, Deserializer, Error as DeError};
use serde::{Serialize, Serializer,Deserialize};
use stable_skiplist::OrderedSkipList;
use std::collections::HashMap;
use std::fmt::{Formatter, Result as FormatResult};
use std::fs::File;
use std::io::{BufRead,BufReader};
use std::path::Path;
use std::str::FromStr;
use xml::attribute::OwnedAttribute;
use xml::reader::{EventReader, XmlEvent};
use links::{Link,load_links};
use stable_skiplist::Bound::{Included, Unbounded};
use stable_skiplist::ordered_skiplist::{Iter};
use std::iter::Take;
use sled;
use sled::Tree;
use serde_json;

/// A WordNet part of speech
#[derive(Clone,Debug)]
pub enum PartOfSpeech {
    Noun, Verb, Adjective, Adverb, AdjectiveSatellite, Other
}

impl FromStr for PartOfSpeech {
    type Err = WordNetLoadError;
    fn from_str(s : &str) -> Result<PartOfSpeech, WordNetLoadError> { 
        match s {
            "n" => Ok(PartOfSpeech::Noun),
            "v" => Ok(PartOfSpeech::Verb),
            "a" => Ok(PartOfSpeech::Adjective),
            "s" => Ok(PartOfSpeech::AdjectiveSatellite),
            "r" => Ok(PartOfSpeech::Adverb),
            "x" => Ok(PartOfSpeech::Other),
            _ => Err(WordNetLoadError::Schema("Bad part of speech value"))
        }
    }
}

impl ToString for PartOfSpeech {
    fn to_string(&self) -> String {
        match *self {
            PartOfSpeech::Noun => "n".to_string(),
            PartOfSpeech::Verb => "v".to_string(),
            PartOfSpeech::Adjective => "a".to_string(),
            PartOfSpeech::AdjectiveSatellite => "s".to_string(),
            PartOfSpeech::Adverb => "r".to_string(),
            PartOfSpeech::Other => "x".to_string()
        }
    }
}

impl PartOfSpeech {
    pub fn as_long_string(&self) -> &'static str {
        match *self {
            PartOfSpeech::Noun => "noun",
            PartOfSpeech::Verb => "verb",
            PartOfSpeech::Adjective => "adjective",
            PartOfSpeech::AdjectiveSatellite => "adjective_satellite",
            PartOfSpeech::Adverb => "adverb",
            PartOfSpeech::Other => "other"
        }
    }
}

impl Serialize for PartOfSpeech {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for PartOfSpeech {
    fn deserialize<D>(deserializer: D) -> Result<PartOfSpeech, D::Error>
        where D: Deserializer<'de> {
        deserializer.deserialize_str(PartOfSpeechVisitor)        
    }
}

struct PartOfSpeechVisitor;

impl<'de> Visitor<'de> for PartOfSpeechVisitor {
    type Value = PartOfSpeech;

    fn expecting(&self, formatter : &mut Formatter) -> FormatResult {
        formatter.write_str("A part of speech value as a single letter: n,v,a,r,s or x")
    }

    fn visit_str<E>(self, value : &str) -> Result<PartOfSpeech, E>  where E : DeError {
        PartOfSpeech::from_str(value)
            .map_err(|e| E::custom(e))
    }

    fn visit_string<E>(self, value : String) -> Result<PartOfSpeech, E> where E : DeError {
        PartOfSpeech::from_str(&value)
            .map_err(|e| E::custom(e))
    }
}




/// A WordNet Key consisting of 8 digits and a part of speech.
/// This data structure stores the value as a 4-byte integer to save memory
#[derive(Clone,Debug,PartialEq,Eq,Hash,PartialOrd)]
pub struct WNKey(u32);

//impl WNKey {
//    /// Create from an ID and a part of speech
//    pub fn new(id : u32, pos : char) -> Result<WNKey, WordNetLoadError> {
//        match pos {
//            'n' => Ok(WNKey((id << 8) + 1)),
//            'v' => Ok(WNKey((id << 8) + 2)),
//            'a' => Ok(WNKey((id << 8) + 3)),
//            'r' => Ok(WNKey((id << 8) + 4)),
//            's' => Ok(WNKey((id << 8) + 5)),
//            'p' => Ok(WNKey((id << 8) + 6)),
//            'x' => Ok(WNKey((id << 8) + 7)),
//            _ => Err(WordNetLoadError::BadKey(format!("Bad WN POS: {}", pos)))
//        }
//    }
//        
//}

impl FromStr  for WNKey {
    type Err = WordNetLoadError;
    fn from_str(s : &str) -> Result<WNKey, WordNetLoadError> { 
        if s.len() != 10 {
            Err(WordNetLoadError::BadKey(format!("Bad WN Key: {}", s)))
        } else {
            let num = u32::from_str(&s.chars().take(8).collect::<String>())
                .map_err(|_| WordNetLoadError::BadKey(format!("Bad WN Key: {}", s)))? << 8;
            match s.chars().skip(9).next() {
                Some('n') => Ok(WNKey(0x00000001 | num)),
                Some('v') => Ok(WNKey(0x00000002 | num)),
                Some('a') => Ok(WNKey(0x00000003 | num)),
                Some('r') => Ok(WNKey(0x00000004 | num)),
                Some('s') => Ok(WNKey(0x00000005 | num)),
                Some('p') => Ok(WNKey(0x00000006 | num)),
                Some('x') => Ok(WNKey(0x00000007 | num)),
                _ => Err(WordNetLoadError::BadKey(format!("Bad WN Key: {}", s)))
            }
        }
    }
}

impl ToString for WNKey {
    fn to_string(&self) -> String { 
        match self.0 & 0x0000000f {
            1 => format!("{:08}-n", (self.0 & 0xfffffff0) >> 8),
            2 => format!("{:08}-v", (self.0 & 0xfffffff0) >> 8),
            3 => format!("{:08}-a", (self.0 & 0xfffffff0) >> 8),
            4 => format!("{:08}-r", (self.0 & 0xfffffff0) >> 8),
            5 => format!("{:08}-s", (self.0 & 0xfffffff0) >> 8),
            6 => format!("{:08}-p", (self.0 & 0xfffffff0) >> 8),
            7 => format!("{:08}-x", (self.0 & 0xfffffff0) >> 8),
            _ => format!("{:08}-?", (self.0 & 0xfffffff0) >> 8)
        }
    }
}

impl Serialize for WNKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for WNKey {
    fn deserialize<D>(deserializer: D) -> Result<WNKey, D::Error>
        where D: Deserializer<'de> {
        deserializer.deserialize_str(WNKeyVisitor)        
    }
}

struct WNKeyVisitor;

impl<'de> Visitor<'de> for WNKeyVisitor {
    type Value = WNKey;

    fn expecting(&self, formatter : &mut Formatter) -> FormatResult {
        formatter.write_str("A WordNet key such as 00001740-a")
    }

    fn visit_str<E>(self, value : &str) -> Result<WNKey, E>  where E : DeError {
        WNKey::from_str(value)
            .map_err(|e| E::custom(e))
    }

    fn visit_string<E>(self, value : String) -> Result<WNKey, E> where E : DeError {
        WNKey::from_str(&value)
            .map_err(|e| E::custom(e))
    }
}

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Synset {
    pub definition : String,
    pub lemmas : Vec<Sense>,
    pub id : WNKey,
    pub ili : String,
    pub pos : PartOfSpeech,
    pub subject : String,
    pub relations : Vec<Relation>,
    pub old_keys : HashMap<String, Vec<WNKey>>,
    pub gloss : Option<Vec<Gloss>>,
    pub foreign : HashMap<String, Vec<String>>,
    pub links : Vec<Link>
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
    synsets : Tree,
    by_lemma : Tree,
    by_ili : Tree,
    by_sense_key : Tree,
    by_old_id : HashMap<String, Tree>
}


impl WordNet {
    pub fn new() -> WordNet {
        WordNet {
            synsets: sled::Config::default()
                .path("sled/synsets".to_owned()).tree(),
            by_lemma: sled::Config::default()
                .path("sled/by_lemma".to_owned()).tree(),
            by_ili: sled::Config::default()
                .path("sled/by_ili".to_owned()).tree(),
            by_sense_key: sled::Config::default()
                .path("sled/by_sense_key".to_owned()).tree(),
            by_old_id: HashMap::new()
        }
    }
 
    pub fn set_synsets(&mut self, values : HashMap<WNKey, Synset>) -> Result<(),WordNetLoadError> {
        for (k, v) in values {
            self.set_synset(k, v)?;
        }
        Ok(())
    }

    pub fn set_synset(&mut self, key : WNKey, synset : Synset) -> Result<(),WordNetLoadError> {
        let key_str = key.to_string();
        let val_str = serde_json::to_string(&synset)?;
        self.synsets.set(Vec::from(key_str.as_bytes()),
                          Vec::from(val_str.as_bytes()));
        //self.by_ili.set(Vec::from(synset.ili.as_bytes()),
        //            Vec::from(key_str.as_bytes()));
        //for sense in synset.lemmas {
        //    let lemmas = Vec::new();
        //    if let Some(prev_data) = self.by_lemma.get(key_str.as_bytes()) {
        //        let pls : Vec<WNKey> = serde_json:from_slice(prev_data.as_slice())
        //            .expect("Database corrpution");
        //        lemmas.extend(pls);
        //    }
        //    lemmas.push(key_str);
        //    let lemmas_val = serde_json::to_string(&lemmas)
        //        .expect("Cannot encode to database");

        //    self.by_lemma.set(Vec::from(sense.lemma.as_bytes()),
        //                    Vec::from(lemmas_val.as_bytes()));
        //    self.by_sense_key.set(Vec::from(sense.sense_key.as_bytes()),
        //                    Vec:;from(key_str.as_bytes()));
        //}                        
        // TODO : Old Keys
        Ok(())
    }
         
    pub fn synsets<'a>(&'a self) -> Box<Iterator<Item=Synset>+'a> {
        let iter = self.synsets.iter()
                 .map(|k| {
                      serde_json::from_slice(k.1.as_slice()) 
                          .expect("Database corrpution")
                 });
        Box::new(iter)
    }
            
    pub fn get_synset(&self, key : &WNKey) -> Option<Synset> { 
        let key_u8 = key.to_string();
        eprintln!("Sled get synset");
        self.synsets.get(key_u8.as_bytes()).map(|data|
            serde_json::from_slice(data.as_slice())
                .expect("Database corruption"))
    }
    pub fn get_by_lemma(&self, lemma : &str) -> Vec<Synset> { 
        let mut v = Vec::new();
        if let Some(vstr) = self.by_lemma.get(lemma.as_bytes()) {
            let values : Vec<WNKey> = serde_json::from_slice(vstr.as_slice())
                .expect("Database corrpution");
            for v2 in values {
                if let Some(s) = self.get_synset(&v2) {
                    v.push(s);
                }
            }
        }
        v
    }
    pub fn get_id_by_ili(&self, ili : &str) -> Option<WNKey> {
        self.by_ili.get(ili.as_bytes())
            .map(|v| serde_json::from_slice(v.as_slice()).expect("Corrupt database"))
    }
    pub fn get_by_ili(&self, ili : &str) -> Option<Synset> {
        self.by_ili.get(ili.as_bytes())
            .map(|v| serde_json::from_slice(v.as_slice()).expect("Corrupt database"))
            .and_then(|l| self.get_synset(&l))
    }
    pub fn get_id_by_sense_key(&self, sense_key : &str) -> Option<WNKey> {
        self.by_sense_key.get(sense_key.as_bytes())
            .map(|v| serde_json::from_slice(v.as_slice()).expect("Corrupt database"))
    }
    pub fn get_by_sense_key(&self, sense_key : &str) -> Option<Synset> {
        self.by_sense_key.get(sense_key.as_bytes())
            .map(|v| serde_json::from_slice(v.as_slice()).expect("Corrupt database"))
            .and_then(|l| self.get_synset(&l))
    }
    pub fn get_id_by_old_id(&self, index : &str, id : &WNKey) -> Result<Option<WNKey>,&'static str> {
        let map = self.by_old_id.get(index).ok_or("No index")?;
        Ok(map.get(id.to_string().as_bytes())
                   .map(|v| serde_json::from_slice(v.as_slice()).expect("Corrupt database")))
    }
    pub fn get_by_old_id(&self, index : &str, id : &WNKey) -> Result<Option<Synset>,&'static str> {
        let map = self.by_old_id.get(index).ok_or("No index")?;
        Ok(map.get(id.to_string().as_bytes())
           .map(|v| serde_json::from_slice(v.as_slice()).expect("Corrupt database"))
           .and_then(|l| self.get_synset(&l)))
    }
    pub fn set_old_id(&mut self, index : &str, id : &WNKey, old_id : &WNKey) -> Result<(),&'static str> {
        self.by_old_id.entry(index.to_owned())
            .or_insert_with(|| sled::Config::default()
                .path("sled/by_ili".to_owned()).tree())
            .set(Vec::from(old_id.to_string().as_bytes()), 
                 Vec::from(id.to_string().as_bytes()));
        if let Some(mut synset) = self.get_synset(id) {
            synset
                .old_keys.entry(index.to_string())
                    .or_insert_with(|| Vec::new())
                    .push(old_id.clone());
            self.set_synset(id.clone(), synset)
                .map_err(|_| "Could not write synset")?;
        }
        Ok(())
    }
    pub fn list_by_id(&self, key : &WNKey, 
                      limit : usize) -> Vec<WNKey> {
        let key_str = key.to_string();
        eprintln!("Sled list by ID");
        self.synsets.scan(key_str.as_bytes()).map(|i|
            WNKey::from_str(&String::from_utf8(i.0).expect("Database corrupt"))
                .expect("Database corrupt"))
            .take(limit)
            .collect()
    }
    pub fn list_by_lemma(&self, lemma : &String,
                          limit : usize) -> Vec<String> {
        self.by_lemma.scan(lemma.as_bytes()).map(|i| 
            serde_json::from_slice(i.1.as_slice()).expect("Database corrupt"))
            .take(limit)
            .collect()
     }
    pub fn list_by_ili(&self, ili : &String,
                        limit : usize) -> Vec<String> {
        self.by_ili.scan(ili.as_bytes()).map(|i| 
            serde_json::from_slice(i.1.as_slice()).expect("Database corrupt"))
            .take(limit)
            .collect()
     }
    pub fn list_by_sense_key(&self, sense_key : &String,
                              limit : usize) -> Vec<String> {
        self.by_sense_key.scan(sense_key.as_bytes()).map(|i| 
            serde_json::from_slice(i.1.as_slice()).expect("Database corrupt"))
            .take(limit)
            .collect()
    }
    pub fn list_by_old_id(&self, index : &str, key : &WNKey,
                      limit : usize) -> Result<Vec<WNKey>,&'static str> {
        let list = self.by_old_id.get(index).ok_or("Index not found")?;
        Ok(list.scan(key.to_string().as_bytes()).map(|i| {
            let z = i.1;
            serde_json::from_slice(z.as_slice()).expect("Database corrupt")
        }).take(limit).collect())
    }
}

fn attr_value(attr : &Vec<OwnedAttribute>, name : &'static str) -> Option<String> {
    attr.iter().find(|a| a.name.local_name == name).map(|a| a.value.clone())
}

fn clean_id(s : &str) -> Result<WNKey, WordNetLoadError> {
    let s2 : String = s.chars().skip(5).collect();
    WNKey::from_str(&s2)
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
        let mut relations : HashMap<WNKey,Vec<Relation>> = HashMap::new();

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
                                    "Sense does not have a synset"))?)?;
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
                                    "Synset does not have an id"))?)?);
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
                                "SynsetRelation without target"))?)?;
                        let ss = synset_id.clone()
                            .ok_or_else(|| WordNetLoadError::Schema(
                                "SynsetRelation outside of Sense"))?;
                        relations.entry(ss)
                            .or_insert_with(|| Vec::new())
                            .push(Relation {
                                src_word: None,
                                trg_word: None,
                                rel_type: typ,
                                target: targ.to_string()
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
                                        target: sense_to_synset[&r.target].to_string()
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
                                pos: PartOfSpeech::from_str(&pos)?,
                                subject: subject,
                                relations: rels,
                                old_keys: HashMap::new(),
                                gloss: None,
                                foreign: HashMap::new(),
                                links: Vec::new()
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
        let mut wordnet = WordNet::new();
        wordnet.set_synsets(synsets)?;
        build_indexes(&mut wordnet);
        build_tabs(&mut wordnet)?;
        build_glosstags(&mut wordnet)?;
        build_omwn(&mut wordnet)?;
        load_links(&mut wordnet)?;
        //eprintln!("size_of synsets: {}", wordnet.synsets.len());
        //eprintln!("size_of by_lemma: {}", wordnet. by_lemma.len());
        //eprintln!("size_of by_ili: {}", wordnet.by_ili.len());
        //eprintln!("size_of by_sense_key: {}", wordnet.by_sense_key.len());
        //eprintln!("size_of by_old_id: {}", wordnet.by_old_id.len());
        //eprintln!("size_of id_skiplist: {}", wordnet.id_skiplist.len());
        //eprintln!("size_of lemma_skiplist: {}", wordnet.lemma_skiplist.len());
        //eprintln!("size_of ili_skiplist: {}", wordnet.ili_skiplist.len());
        //eprintln!("size_of sense_key_skiplist: {}", wordnet.sense_key_skiplist.len());
        //eprintln!("size_of old_skiplist: {}", wordnet.old_skiplist.len());

        Ok(wordnet)
    }
}

fn build_glosstags(wordnet : &mut WordNet)
         -> Result<(), WordNetLoadError> {
    eprintln!("Loading gloss tags (adj)");
    let mut result = read_glosstag_corpus("data/merged/adj.xml", &wordnet)?;
    eprintln!("Loading gloss tags (adv)");
    result.extend(read_glosstag_corpus("data/merged/adv.xml", &wordnet)?);
    eprintln!("Loading gloss tags (noun)");
    result.extend(read_glosstag_corpus("data/merged/noun.xml", &wordnet)?);
    eprintln!("Loading gloss tags (verb)");
    result.extend(read_glosstag_corpus("data/merged/verb.xml", &wordnet)?);
    for (k,v) in result.iter() {
        if let Some(mut s) = wordnet.get_synset(k) {
            s.gloss = Some(v.clone());
            wordnet.set_synset(k.clone(), s)?;
        }
    }
    Ok(())
}

fn build_tab<P : AsRef<Path>>(file : P, 
    index : &str,
    wordnet : &mut WordNet) -> Result<(),WordNetLoadError> {
    let file = BufReader::new(File::open(file)?);
    for line in file.lines() {
        let line = line?;
        let mut elems = line.split("\t");
        match elems.next() {
            Some(ili) => {
                match elems.next() {
                    Some(id) => {
                        let wnid2 = wordnet.get_id_by_ili(ili).map(|x| x.clone());
                        match wnid2 {
                            Some(wnid) => {
                                wordnet.set_old_id(index, 
                                    &wnid,
                                    &WNKey::from_str(id)?)
                                .map_err(|e| WordNetLoadError::BadKey(e.to_owned()))?;
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
//    eprintln!("Building indexes");
//    for (id, synset) in wordnet.synsets.iter() {
//        wordnet.ili_skiplist.insert(synset.ili.clone());
//        wordnet.id_skiplist.insert(id.clone());
//        wordnet.by_ili.insert(synset.ili.clone(), id.clone());
//        for sense in synset.lemmas.iter() {
//            wordnet.by_sense_key.insert(sense.sense_key.clone(), id.clone());
//            wordnet.sense_key_skiplist.insert(sense.sense_key.clone());
//            wordnet.by_lemma.entry(sense.lemma.clone())
//                .or_insert_with(|| Vec::new())
//                .push(id.clone());
//        }
//    }
}

fn build_tabs(wordnet : &mut WordNet) -> Result<(),WordNetLoadError> {
    for tab in ["pwn15", "pwn16", "pwn17", "pwn171", "pwn20",
                "pwn21", "pwn30"].iter() {
        eprintln!("Loading Tab {}", tab);
        let path = format!("data/ili-map-{}.tab", tab);
        build_tab(path, tab, wordnet)?;
    }
    Ok(())
}

fn build_omwn(wordnet : &mut WordNet) -> Result<(), WordNetLoadError> {
    for lang in ["als","arb","bul","cmn","qcn","ell","fas","fin","fra",
                 "heb","hrv","isl","ita","jpn","cat","eus","glg","spa",
                 "ind","zsm","nld","nno","nob","pol","por","ron",
                 "slk","lit","slv","swe","tha"].iter() {
        eprintln!("Loading OMWN {}", lang);
        let project = match *lang {
            "cmn" => "cow",
            "qcn" => "cwn",
            "ind" => "msa",
            "zsm" => "msa",
            "cat" => "mcr",
            "eus" => "mcr",
            "glg" => "mcr",
            "spa" => "mcr",
            "nno" => "nor",
            "nob" => "nor",
            "lit" => "slk",
            x => x
        };
       let omwn = load_omwn(format!("data/wns/{}/wn-data-{}.tab", project, lang),
            &wordnet)?;
       for (key, values) in omwn.iter() {
           if let Some(mut s2) = wordnet.get_synset(&key) {
               let mut vs = values.clone();
               vs.dedup();
               s2.foreign.insert(lang.to_string(), vs);
               wordnet.set_synset(key.clone(), s2)?;
           }
       }
    }
    Ok(())

}



quick_error! {
    #[derive(Debug)]
    pub enum WordNetLoadError {
        Io(err: ::std::io::Error) { 
            from()
            display("I/O error: {}", err)
            cause(err)
        }
        Xml(err: ::xml::reader::Error) {
            from()
            display("XML error: {}", err)
            cause(err)
        }
        Schema(msg : &'static str) {
            description(msg)
        }
        JsonSerialization(err : ::serde_json::Error) {
            from()
            display("JSON error: {}", err)
            cause(err)
        }
        BadKey(msg : String) {
            description(msg)
        }
    }
}
